use ash::vk;
use wgpu_hal as hal;

/// Загружаем vkGetAndroidHardwareBufferPropertiesANDROID напрямую из libvulkan.so
/// через dlsym. Это обходит ограничение wgpu 22 который не включает
/// VK_ANDROID_external_memory_android_hardware_buffer при создании VkDevice.
/// На Android драйвер реализует эту функцию безусловно.
unsafe fn load_get_ahb_props_fn() -> Option<
    extern "system" fn(
        vk::Device,
        *const std::ffi::c_void,
        *mut vk::AndroidHardwareBufferPropertiesANDROID,
    ) -> vk::Result,
> {
    let lib = libc::dlopen(
        c"libvulkan.so".as_ptr(),
        libc::RTLD_NOW | libc::RTLD_NOLOAD,
    );
    if lib.is_null() {
        log::error!("vulkan_import: dlopen libvulkan.so failed");
        return None;
    }

    let sym = libc::dlsym(
        lib,
        c"vkGetAndroidHardwareBufferPropertiesANDROID".as_ptr(),
    );
    libc::dlclose(lib);

    if sym.is_null() {
        log::error!("vulkan_import: dlsym vkGetAndroidHardwareBufferPropertiesANDROID returned null");
        return None;
    }

    Some(std::mem::transmute(sym))
}

pub unsafe fn import_android_buffer(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    buffer_ptr: *mut std::ffi::c_void, // AHardwareBuffer*
    width: u32,
    height: u32,
) -> Option<wgpu::Texture> {
    device
        .as_hal::<hal::api::Vulkan, _, Option<wgpu::Texture>>(|hal_device| {
            let hal_device = match hal_device {
                Some(d) => d,
                None => {
                    log::error!("vulkan_import: as_hal returned None (no Vulkan device)");
                    return None;
                }
            };
            let raw_device = hal_device.raw_device();

            // 1. Загружаем функцию через dlsym
            let vk_get_ahb_props = match load_get_ahb_props_fn() {
                Some(f) => f,
                None => return None,
            };

            // 2. Получаем свойства AHardwareBuffer
            let mut format_props = vk::AndroidHardwareBufferFormatPropertiesANDROID::default();
            let mut props =
                vk::AndroidHardwareBufferPropertiesANDROID::default().push_next(&mut format_props);

            let result = vk_get_ahb_props(raw_device.handle(), buffer_ptr, &mut props);
            if result != vk::Result::SUCCESS {
                log::error!("vkGetAndroidHardwareBufferPropertiesANDROID failed: {:?}", result);
                return None;
            }

            // Читаем поля props первыми (props держит &mut format_props через push_next)
            let allocation_size = props.allocation_size;
            let memory_type_bits = props.memory_type_bits;
            drop(props); // Освобождаем мутабельную ссылку на format_props

            // Теперь можно читать format_props
            let external_format_val = format_props.external_format;

            log::error!(
                "AHB props: alloc_size={}, memory_type_bits={:#x}, external_format={}",
                allocation_size,
                memory_type_bits,
                external_format_val
            );

            // 3. Создаём VkImage с external memory
            let mut external_image_info = vk::ExternalMemoryImageCreateInfo::default()
                .handle_types(vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID);

            let mut external_format =
                vk::ExternalFormatANDROID::default().external_format(external_format_val);

            let image_create_info = vk::ImageCreateInfo::default()
                .push_next(&mut external_image_info)
                .push_next(&mut external_format)
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::UNDEFINED)
                .extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED);

            let vk_image = match raw_device.create_image(&image_create_info, None) {
                Ok(img) => img,
                Err(e) => {
                    log::error!("create_image failed: {:?}", e);
                    return None;
                }
            };

            // 4. Импорт памяти из AHardwareBuffer
            let mut import_info =
                vk::ImportAndroidHardwareBufferInfoANDROID::default().buffer(buffer_ptr as _);

            let alloc_info = vk::MemoryAllocateInfo::default()
                .allocation_size(allocation_size)
                .memory_type_index(memory_type_bits.trailing_zeros())
                .push_next(&mut import_info);

            let memory = match raw_device.allocate_memory(&alloc_info, None) {
                Ok(mem) => mem,
                Err(e) => {
                    raw_device.destroy_image(vk_image, None);
                    log::error!("allocate_memory failed: {:?}", e);
                    return None;
                }
            };

            if let Err(e) = raw_device.bind_image_memory(vk_image, memory, 0) {
                raw_device.free_memory(memory, None);
                raw_device.destroy_image(vk_image, None);
                log::error!("bind_image_memory failed: {:?}", e);
                return None;
            }

            // 5. Оборачиваем в wgpu_hal::vulkan::Texture
            let hal_desc = hal::TextureDescriptor {
                label: Some("Android Video Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: hal::TextureUses::RESOURCE,
                memory_flags: hal::MemoryFlags::empty(),
                view_formats: vec![],
            };

            let hal_texture = wgpu_hal::vulkan::Device::texture_from_raw(
                vk_image,
                &hal_desc,
                Some(Box::new(memory)),
            );

            let texture_desc = wgpu::TextureDescriptor {
                label: Some("Android Video Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            };

            log::error!("vulkan import SUCCESS!");
            Some(device.create_texture_from_hal::<hal::api::Vulkan>(hal_texture, &texture_desc))
        })
        .flatten()
}
