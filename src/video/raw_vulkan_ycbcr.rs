// src/video/raw_vulkan_ycbcr.rs

use crate::video::vulkan_import::{AhbFormatProps, AhbProps};

type FnGetAhbProps = unsafe extern "system" fn(
    vk::Device,
    *const std::ffi::c_void,
    *mut vk::AndroidHardwareBufferPropertiesANDROID,
) -> vk::Result;

unsafe fn load_get_ahb_props_fn_local(
    instance: &ash::Instance,
    device: vk::Device,
) -> Option<FnGetAhbProps> {
    let name = std::ffi::CStr::from_bytes_with_nul(
        b"vkGetAndroidHardwareBufferPropertiesANDROID\0",
    )
    .unwrap();
    let ptr = instance.get_device_proc_addr(device, name.as_ptr());
    ptr.map(|f| std::mem::transmute(f))
}

use ash::vk;
use ash::util::read_spv;
use std::ffi::CStr;
use std::io::Cursor;

// ── Один слот под текущий AHB кадр ──────────────────────────────────────────
pub struct AhbSlot {
    pub image:          vk::Image,
    pub image_view:     vk::ImageView,
    pub memory:         vk::DeviceMemory,
    pub descriptor_set: vk::DescriptorSet,
    pub external_format: u64,
}

pub struct RawYcbcrPipeline {
    pub ycbcr_conversion:     vk::SamplerYcbcrConversion,
    pub ycbcr_sampler:        vk::Sampler,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub pipeline_layout:      vk::PipelineLayout,
    pub render_pass:          vk::RenderPass,
    pub graphics_pipeline:    vk::Pipeline,
    pub descriptor_pool:      vk::DescriptorPool,

    pub output_image:       vk::Image,
    pub output_image_view:  vk::ImageView,
    pub output_memory:      vk::DeviceMemory,
    pub output_framebuffer: vk::Framebuffer,

    // Единственный слот — заменяется каждый кадр
    pub slot: Option<AhbSlot>,

    pub width:  u32,
    pub height: u32,

    pub fn_get_ahb_props: FnGetAhbProps,
}

/// Command buffer ресурсы — отдельный pool/fence для YCbCr прохода
pub struct YcbcrCommandResources {
    pub command_pool:   vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,
    pub fence:          vk::Fence,
    pub queue:          vk::Queue,
    pub queue_family_index: u32,
}

impl YcbcrCommandResources {
    pub unsafe fn new(
        device: &ash::Device,
        queue_family_index: u32,
        queue: vk::Queue,
    ) -> Option<Self> {
        let command_pool = device
            .create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .queue_family_index(queue_family_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )
            .ok()?;

        let command_buffer = device
            .allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )
            .ok()?[0];

        let fence = device
            .create_fence(
                // SIGNALED: первый wait_for_fences пройдёт сразу
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )
            .ok()?;

        Some(Self {
            command_pool,
            command_buffer,
            fence,
            queue,
            queue_family_index,
        })
    }

    /// Ждём GPU (прошлый кадр), сбрасываем, начинаем запись
    pub unsafe fn begin_frame(&self, device: &ash::Device) {
        device
            .wait_for_fences(&[self.fence], true, u64::MAX)
            .unwrap();
        device.reset_fences(&[self.fence]).unwrap();

        device
            .reset_command_buffer(
                self.command_buffer,
                vk::CommandBufferResetFlags::empty(),
            )
            .unwrap();

        device
            .begin_command_buffer(
                self.command_buffer,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .unwrap();
    }

    /// end + submit, сигналим fence
    pub unsafe fn submit(&self, device: &ash::Device) {
        device.end_command_buffer(self.command_buffer).unwrap();

        let submit_info = vk::SubmitInfo::default()
            .command_buffers(std::slice::from_ref(&self.command_buffer));

        device
            .queue_submit(self.queue, &[submit_info], self.fence)
            .unwrap();
    }

    pub unsafe fn destroy(&self, device: &ash::Device) {
        let _ = device.wait_for_fences(&[self.fence], true, u64::MAX);
        device.destroy_fence(self.fence, None);
        device.destroy_command_pool(self.command_pool, None);
    }
}

impl RawYcbcrPipeline {
    pub unsafe fn new(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
        width: u32,
        height: u32,
        external_format: u64,
        format_props: &AhbFormatProps,
    ) -> Option<Self> {
        let fn_get = load_get_ahb_props_fn_local(instance, device.handle())?;

        // ── 1. YCbCr conversion ──────────────────────────────────────────────
        let mut external_format_vk =
            vk::ExternalFormatANDROID::default().external_format(external_format);

        let ycbcr_info = vk::SamplerYcbcrConversionCreateInfo::default()
            .push_next(&mut external_format_vk)
            .format(vk::Format::UNDEFINED)
            .ycbcr_model(format_props.suggested_ycbcr_model)
            .ycbcr_range(format_props.suggested_ycbcr_range)
            .components(format_props.sampler_ycbcr_conversion_components)
            .x_chroma_offset(format_props.suggested_x_chroma_offset)
            .y_chroma_offset(format_props.suggested_y_chroma_offset)
            .chroma_filter(vk::Filter::NEAREST)
            .force_explicit_reconstruction(false);

        let ycbcr_conversion = device
            .create_sampler_ycbcr_conversion(&ycbcr_info, None)
            .map_err(|e| log::error!("create_sampler_ycbcr_conversion: {:?}", e))
            .ok()?;

        // ── 2. Sampler с YcbcrConversion (immutable) ─────────────────────────
        let mut conversion_info =
            vk::SamplerYcbcrConversionInfo::default().conversion(ycbcr_conversion);

        let sampler_info = vk::SamplerCreateInfo::default()
            .push_next(&mut conversion_info)
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .unnormalized_coordinates(false);

        let ycbcr_sampler = device
            .create_sampler(&sampler_info, None)
            .map_err(|e| log::error!("create_sampler (ycbcr): {:?}", e))
            .ok()?;

        // ── 3. DescriptorSetLayout с immutable sampler ───────────────────────
        let immutable_samplers = [ycbcr_sampler];
        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .immutable_samplers(&immutable_samplers)];

        let descriptor_set_layout = device
            .create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                None,
            )
            .map_err(|e| log::error!("create_descriptor_set_layout: {:?}", e))
            .ok()?;

        // ── 4. Output image (Rgba8Unorm) ─────────────────────────────────────
        let output_image = device
            .create_image(
                &vk::ImageCreateInfo::default()
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .extent(vk::Extent3D { width, height, depth: 1 })
                    .mip_levels(1)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(
                        vk::ImageUsageFlags::COLOR_ATTACHMENT
                            | vk::ImageUsageFlags::SAMPLED
                            | vk::ImageUsageFlags::TRANSFER_SRC,
                    )
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .initial_layout(vk::ImageLayout::UNDEFINED),
                None,
            )
            .map_err(|e| log::error!("create output_image: {:?}", e))
            .ok()?;

        let output_memory = Self::alloc_image_memory(
            instance,
            physical_device,
            device,
            output_image,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        device
            .bind_image_memory(output_image, output_memory, 0)
            .map_err(|e| log::error!("bind output_image_memory: {:?}", e))
            .ok()?;

        let output_image_view = device
            .create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(output_image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    }),
                None,
            )
            .map_err(|e| log::error!("create output_image_view: {:?}", e))
            .ok()?;

        // ── 5. RenderPass ────────────────────────────────────────────────────
        let attachments = [vk::AttachmentDescription::default()
            .format(vk::Format::R8G8B8A8_UNORM)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];

        let color_refs = [vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

        let subpasses = [vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_refs)];

        let dependencies = [vk::SubpassDependency::default()
            .src_subpass(0)
            .dst_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)];

        let render_pass = device
            .create_render_pass(
                &vk::RenderPassCreateInfo::default()
                    .attachments(&attachments)
                    .subpasses(&subpasses)
                    .dependencies(&dependencies),
                None,
            )
            .map_err(|e| log::error!("create_render_pass: {:?}", e))
            .ok()?;

        // ── 6. Framebuffer ───────────────────────────────────────────────────
        let fb_attachments = [output_image_view];
        let output_framebuffer = device
            .create_framebuffer(
                &vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(&fb_attachments)
                    .width(width)
                    .height(height)
                    .layers(1),
                None,
            )
            .map_err(|e| log::error!("create_framebuffer: {:?}", e))
            .ok()?;

        // ── 7. Pipeline Layout ───────────────────────────────────────────────
        let set_layouts = [descriptor_set_layout];
        let pipeline_layout = device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts),
                None,
            )
            .map_err(|e| log::error!("create_pipeline_layout: {:?}", e))
            .ok()?;

        // ── 8. Shaders ───────────────────────────────────────────────────────
        let vert_module = device
            .create_shader_module(
                &vk::ShaderModuleCreateInfo::default().code(vert_spirv_words()),
                None,
            )
            .map_err(|e| log::error!("create vert shader: {:?}", e))
            .ok()?;

        let frag_module = device
            .create_shader_module(
                &vk::ShaderModuleCreateInfo::default().code(frag_spirv_words()),
                None,
            )
            .map_err(|e| log::error!("create frag shader: {:?}", e))
            .ok()?;

        let entry_point = CStr::from_bytes_with_nul(b"main\0").unwrap();

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_module)
                .name(entry_point),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_module)
                .name(entry_point),
        ];

        // ── 9. Graphics Pipeline ─────────────────────────────────────────────
        let vertex_input   = vk::PipelineVertexInputStateCreateInfo::default();
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewport = vk::Viewport {
            x: 0.0, y: 0.0,
            width: width as f32, height: height as f32,
            min_depth: 0.0, max_depth: 1.0,
        };
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width, height },
        };
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(std::slice::from_ref(&viewport))
            .scissors(std::slice::from_ref(&scissor));

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);

        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false);

        let blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&blend_attachment));

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisample)
            .color_blend_state(&blend_state)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);

        let graphics_pipeline = device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&pipeline_info),
                None,
            )
            .map_err(|(_, e)| log::error!("create_graphics_pipelines: {:?}", e))
            .ok()?[0];

        device.destroy_shader_module(vert_module, None);
        device.destroy_shader_module(frag_module, None);

        // ── 10. Descriptor Pool ──────────────────────────────────────────────
        // max_sets=1: всегда ровно один активный дескриптор
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)];

        let descriptor_pool = device
            .create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .pool_sizes(&pool_sizes)
                    .max_sets(1)
                    .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET),
                None,
            )
            .map_err(|e| log::error!("create_descriptor_pool: {:?}", e))
            .ok()?;

        Some(Self {
            ycbcr_conversion,
            ycbcr_sampler,
            descriptor_set_layout,
            pipeline_layout,
            render_pass,
            graphics_pipeline,
            descriptor_pool,
            output_image,
            output_image_view,
            output_memory,
            output_framebuffer,
            slot: None,
            width,
            height,
            fn_get_ahb_props: fn_get,
        })
    }

    // ── Вспомогательные ─────────────────────────────────────────────────────

    unsafe fn alloc_image_memory(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
        image: vk::Image,
        required_flags: vk::MemoryPropertyFlags,
    ) -> Option<vk::DeviceMemory> {
        let mem_reqs = device.get_image_memory_requirements(image);
        let mem_props = instance.get_physical_device_memory_properties(physical_device);

        let memory_type_index = (0..mem_props.memory_type_count).find(|&i| {
            let bit      = 1u32 << i;
            let suitable = (mem_reqs.memory_type_bits & bit) != 0;
            let flags_ok = mem_props.memory_types[i as usize]
                .property_flags
                .contains(required_flags);
            suitable && flags_ok
        })?;

        device
            .allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(mem_reqs.size)
                    .memory_type_index(memory_type_index),
                None,
            )
            .map_err(|e| log::error!("allocate_memory: {:?}", e))
            .ok()
    }

    /// Освобождаем текущий слот.
    /// ВАЖНО: вызывать только когда GPU гарантированно закончил
    /// (после wait_for_fences в YcbcrCommandResources::begin_frame).
    unsafe fn drop_slot(&mut self, device: &ash::Device) {
        if let Some(old) = self.slot.take() {
            device
                .free_descriptor_sets(self.descriptor_pool, &[old.descriptor_set])
                .ok();
            device.destroy_image_view(old.image_view, None);
            device.destroy_image(old.image, None);
            device.free_memory(old.memory, None);
        }
    }

    /// Создаём VkImage + VkDeviceMemory (import AHB) + VkImageView + DescriptorSet
    /// под конкретный AHardwareBuffer*.
    unsafe fn create_slot(
        &self,
        device: &ash::Device,
        buffer_ptr: *mut std::ffi::c_void,
        external_format: u64,
        ahb_props: &AhbProps,
    ) -> Option<AhbSlot> {
        // VkImage
        let mut ext_mem_info = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID);
        let mut ext_format_info =
            vk::ExternalFormatANDROID::default().external_format(external_format);

        let image = device
            .create_image(
                &vk::ImageCreateInfo::default()
                    .push_next(&mut ext_mem_info)
                    .push_next(&mut ext_format_info)
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(vk::Format::UNDEFINED)
                    .extent(vk::Extent3D { width: self.width, height: self.height, depth: 1 })
                    .mip_levels(1)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(vk::ImageUsageFlags::SAMPLED)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .initial_layout(vk::ImageLayout::UNDEFINED),
                None,
            )
            .map_err(|e| log::error!("create_slot: create_image: {:?}", e))
            .ok()?;

        // VkDeviceMemory — импортируем конкретный AHardwareBuffer*
        let mut import_info =
            vk::ImportAndroidHardwareBufferInfoANDROID::default().buffer(buffer_ptr as _);
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
        let memory_type   = ahb_props.memory_type_bits.trailing_zeros();

        let memory = device
            .allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .push_next(&mut dedicated)
                    .push_next(&mut import_info)
                    .allocation_size(ahb_props.allocation_size)
                    .memory_type_index(memory_type),
                None,
            )
            .map_err(|e| {
                // Откатываем image если memory не удалась
                device.destroy_image(image, None);
                log::error!("create_slot: allocate_memory: {:?}", e);
            })
            .ok()?;

        device
            .bind_image_memory(image, memory, 0)
            .map_err(|e| {
                device.free_memory(memory, None);
                device.destroy_image(image, None);
                log::error!("create_slot: bind_image_memory: {:?}", e);
            })
            .ok()?;

        // VkImageView с YcbcrConversionInfo в pNext
        let mut ycbcr_info =
            vk::SamplerYcbcrConversionInfo::default().conversion(self.ycbcr_conversion);

        let image_view = device
            .create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .push_next(&mut ycbcr_info)
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::UNDEFINED)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    }),
                None,
            )
            .map_err(|e| {
                device.free_memory(memory, None);
                device.destroy_image(image, None);
                log::error!("create_slot: create_image_view: {:?}", e);
            })
            .ok()?;

        // DescriptorSet
        let set_layouts = [self.descriptor_set_layout];
        let descriptor_set = device
            .allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(self.descriptor_pool)
                    .set_layouts(&set_layouts),
            )
            .map_err(|e| {
                device.destroy_image_view(image_view, None);
                device.free_memory(memory, None);
                device.destroy_image(image, None);
                log::error!("create_slot: allocate_descriptor_sets: {:?}", e);
            })
            .ok()?[0];

        // Обновляем дескриптор
        let image_infos = [vk::DescriptorImageInfo::default()
            .sampler(vk::Sampler::null()) // immutable — не нужен здесь
            .image_view(image_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
        let writes = [vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_infos)];
        device.update_descriptor_sets(&writes, &[]);

        Some(AhbSlot { image, image_view, memory, descriptor_set, external_format })
    }

    /// Записываем в command_buffer: barrier + render pass.
    unsafe fn record_commands(
        &self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        slot: &AhbSlot,
    ) {
        // UNDEFINED → SHADER_READ_ONLY_OPTIMAL
        // Каждый кадр image новый — layout всегда UNDEFINED
        let barrier = vk::ImageMemoryBarrier::default()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(slot.image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::SHADER_READ);

        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[], &[],
            std::slice::from_ref(&barrier),
        );

        // Render pass: YCbCr AHB → output_image (Rgba8Unorm)
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] },
        }];

        device.cmd_begin_render_pass(
            command_buffer,
            &vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.output_framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D { width: self.width, height: self.height },
                })
                .clear_values(&clear_values),
            vk::SubpassContents::INLINE,
        );

        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.graphics_pipeline,
        );

        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_layout,
            0,
            &[slot.descriptor_set],
            &[],
        );

        device.cmd_draw(command_buffer, 3, 1, 0, 0);
        device.cmd_end_render_pass(command_buffer);
    }

    // ── Публичный API ────────────────────────────────────────────────────────

    pub unsafe fn query_ahb_properties_struct(
        &self,
        raw_device: &ash::Device,
        buffer_ptr: *mut std::ffi::c_void,
    ) -> Option<(u64, AhbProps)> {
        let mut format_props = vk::AndroidHardwareBufferFormatPropertiesANDROID::default();
        let mut props = vk::AndroidHardwareBufferPropertiesANDROID::default()
            .push_next(&mut format_props);

        let result = (self.fn_get_ahb_props)(raw_device.handle(), buffer_ptr, &mut props);
        if result != vk::Result::SUCCESS {
            log::error!("vkGetAndroidHardwareBufferPropertiesANDROID: {:?}", result);
            return None;
        }

        let allocation_size  = props.allocation_size;
        let memory_type_bits = props.memory_type_bits;
        let external_format  = format_props.external_format;
        let vk_format = if external_format == 0 {
            format_props.format
        } else {
            vk::Format::UNDEFINED
        };

        Some((
            external_format,
            AhbProps { allocation_size, memory_type_bits, external_format, vk_format },
        ))
    }

    /// Главный метод — вызывается каждый кадр.
    ///
    /// Контракт вызывающей стороны:
    /// 1. Перед вызовом GPU должен быть синхронизирован
    ///    (YcbcrCommandResources::begin_frame уже вызвал wait_for_fences).
    /// 2. После вызова — submit с fence.
    pub unsafe fn process_ahb_frame(
        &mut self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        buffer_ptr: *mut std::ffi::c_void,
        external_format: u64,
        ahb_props: &AhbProps,
    ) -> bool {
        // GPU закончил прошлый кадр (begin_frame уже подождал) —
        // безопасно уничтожаем старый слот.
        self.drop_slot(device);

        // Создаём новый слот под текущий AHardwareBuffer*.
        let slot = match self.create_slot(device, buffer_ptr, external_format, ahb_props) {
            Some(s) => s,
            None => {
                log::error!("process_ahb_frame: create_slot failed");
                return false;
            }
        };

        // Записываем команды.
        self.record_commands(device, command_buffer, &slot);

        // Сохраняем слот — GPU будет работать с ним до следующего begin_frame.
        self.slot = Some(slot);
        true
    }

    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        // Ждём GPU перед уничтожением (на случай если вызывается вне begin_frame)
        // Caller должен гарантировать что fence просигналён, но подстрахуемся логом.
        // Реальный wait должен быть снаружи через YcbcrCommandResources::destroy.
        self.drop_slot(device);

        device.destroy_descriptor_pool(self.descriptor_pool, None);
        device.destroy_framebuffer(self.output_framebuffer, None);
        device.destroy_image_view(self.output_image_view, None);
        device.destroy_image(self.output_image, None);
        device.free_memory(self.output_memory, None);
        device.destroy_render_pass(self.render_pass, None);
        device.destroy_pipeline(self.graphics_pipeline, None);
        device.destroy_pipeline_layout(self.pipeline_layout, None);
        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        device.destroy_sampler(self.ycbcr_sampler, None);
        device.destroy_sampler_ycbcr_conversion(self.ycbcr_conversion, None);
    }
}

// ── SPIR-V ───────────────────────────────────────────────────────────────────

fn vert_spirv_words() -> &'static [u32] {
    use std::sync::OnceLock;
    static DATA: OnceLock<Vec<u32>> = OnceLock::new();
    DATA.get_or_init(|| {
        read_spv(&mut Cursor::new(include_bytes!("../../shaders/ycbcr_blit.vert.spv")))
            .expect("Failed to parse vertex shader SPIR-V")
    })
    .as_slice()
}

fn frag_spirv_words() -> &'static [u32] {
    use std::sync::OnceLock;
    static DATA: OnceLock<Vec<u32>> = OnceLock::new();
    DATA.get_or_init(|| {
        read_spv(&mut Cursor::new(include_bytes!("../../shaders/ycbcr_blit.frag.spv")))
            .expect("Failed to parse fragment shader SPIR-V")
    })
    .as_slice()
}