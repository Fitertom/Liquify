use crate::ecs::components::*;
use crate::ecs::world::World;

use crate::input::InputState;
use crate::render::geometry::{Vertex, build_background_quad, build_glass_quad, build_shadow_quad};
use crate::video::{VideoFrame, VideoPlayer};
use glyphon::{
    Attrs, Buffer, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurParams {
    texel_size: [f32; 2],
    radius: f32,
    direction: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextInstance {
    pub x: f32,
    pub y: f32,
    pub glyph_index: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextParams {
    pub transform: [f32; 16],
    pub color: [f32; 4],
    pub scale: f32,
    pub _padding: [f32; 3],
}

struct ManagedBuffer {
    buffer: glyphon::Buffer,
    last_text: String,
    last_scale: f32,
}

#[derive(Copy, Clone)]
struct ScissorRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

pub struct RenderPipeline {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
    pub sampler: wgpu::Sampler,
    pub background_texture: wgpu::Texture,
    pub background_texture_view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    pub scene_texture: wgpu::Texture,
    pub scene_texture_view: wgpu::TextureView,
    pub scene_bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub content_vertex_count: u32,
    pub ui_vertex_count: u32,
    pub scene_blur_ui_vertex_count: u32,

    // Separate vertex buffers for UI layer
    pub ui_vertex_buffer: wgpu::Buffer,
    pub scene_blur_ui_vertex_buffer: wgpu::Buffer,

    pub scale_factor: f32,

    pub blur_pipeline: wgpu::RenderPipeline,
    pub blur_params_buffer_h: wgpu::Buffer,
    pub blur_params_buffer_v: wgpu::Buffer,
    pub blurred_texture: wgpu::Texture,
    pub blurred_texture_view: wgpu::TextureView,
    pub scene_blurred_texture: wgpu::Texture,
    pub scene_blurred_texture_view: wgpu::TextureView,
    pub temp_texture: wgpu::Texture,
    pub temp_texture_view: wgpu::TextureView,

    pub _cover_texture: wgpu::Texture,
    pub cover_texture_view: wgpu::TextureView,

    pub vector_pipeline: wgpu::RenderPipeline,
    pub vector_vertex_buffer: wgpu::Buffer,
    pub vector_vertex_count: u32,
    pub content_vector_vertex_count: u32,
    pub ui_vector_vertex_count: u32,

    // --- GLYPHON ---
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub text_atlas: TextAtlas,
    pub content_text_renderer: TextRenderer,
    pub text_renderer: TextRenderer,
    pub text_viewport: Viewport,
    pub content_text_buffer_pool: Vec<ManagedBuffer>,
    pub text_buffer_pool: Vec<ManagedBuffer>,
    // Content text (cards, headers, sections) - rendered before UI glass
    pub content_text_requests: Vec<(String, f32, f32, f32, [f32; 4])>,
    // UI text (navbar, miniplayer, fullscreen player) - rendered on top
    pub text_requests: Vec<(String, f32, f32, f32, [f32; 4])>,
    pub gradient_text_requests: Vec<(String, f32, f32, f32)>,
    pub gradient_text_buffer: glyphon::Buffer,

    // Temporary accumulation buffers (not sent to GPU yet)
    pub content_vertices: Vec<Vertex>,
    pub ui_vertices: Vec<Vertex>,
    pub scene_blur_ui_vertices: Vec<Vertex>,
    pub content_vector_vertices: Vec<Vertex>,
    pub ui_vector_vertices: Vec<Vertex>,

    pub bg_layout: wgpu::BindGroupLayout,
    pub video_player: Option<VideoPlayer>,

    // Gradient effect resources
    pub gradient_mask_texture: wgpu::Texture,
    pub gradient_mask_view: wgpu::TextureView,
    pub gradient_composite_pipeline: wgpu::RenderPipeline,
    pub gradient_composite_bind_group: wgpu::BindGroup,
    pub gradient_uniform_buffer: wgpu::Buffer,

    #[cfg(target_os = "android")]
    pub ycbcr_pipeline: Option<crate::video::raw_vulkan_ycbcr::RawYcbcrPipeline>,

    #[cfg(target_os = "android")]
    pub ycbcr_cmd_resources: Option<crate::video::raw_vulkan_ycbcr::YcbcrCommandResources>,

    #[cfg(target_os = "android")]
    pub ycbcr_output_texture: Option<wgpu::Texture>,

    #[cfg(target_os = "android")]
    pub ycbcr_output_view: Option<wgpu::TextureView>,

    #[cfg(target_os = "android")]
    #[cfg(target_os = "android")]
    pub ycbcr_ahb_props: Option<crate::video::vulkan_import::AhbProps>,
}

impl RenderPipeline {
    pub async fn new(window: &winit::window::Window, font_data: &[u8]) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance.create_surface(window).unwrap();
        let surface: wgpu::Surface<'static> = unsafe { std::mem::transmute(surface) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("Failed to find adapter");

        #[cfg(not(target_os = "android"))]
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        // На Android создаём device через HAL с включением
        // VK_ANDROID_external_memory_android_hardware_buffer
        #[cfg(target_os = "android")]
        let (device, queue) = {
            use wgpu_hal::api::Vulkan;

            let ahb_ext_name = c"VK_ANDROID_external_memory_android_hardware_buffer";

            // В wgpu 29 as_hal принимает только тип, без замыкания
            let open_device_result = unsafe {
                adapter.as_hal::<Vulkan>().and_then(|hal_adapter| {
                    hal_adapter
                        .open_with_callback(
                            wgpu::Features::empty(),
                            &wgpu::Limits::downlevel_webgl2_defaults()
                                .using_resolution(adapter.limits()),
                            &wgpu::MemoryHints::default(),
                            Some(Box::new(move |args| {
                                args.extensions.push(ahb_ext_name);
                            })),
                        )
                        .ok()
                })
            };

            if let Some(open_device) = open_device_result {
                log::error!("open_with_callback SUCCESS - AHB extension enabled");
                unsafe {
                    adapter
                        .create_device_from_hal(
                            open_device,
                            &wgpu::DeviceDescriptor {
                                label: None,
                                required_features: wgpu::Features::empty(),
                                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                                    .using_resolution(adapter.limits()),
                                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                                memory_hints: wgpu::MemoryHints::default(),
                                trace: wgpu::Trace::Off,
                            },
                        )
                        .unwrap()
                }
            } else {
                log::error!("open_with_callback failed, falling back to request_device");
                adapter
                    .request_device(&wgpu::DeviceDescriptor {
                        label: None,
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits()),
                        experimental_features: wgpu::ExperimentalFeatures::disabled(),
                        memory_hints: wgpu::MemoryHints::default(),
                        trace: wgpu::Trace::Off,
                    })
                    .await
                    .unwrap()
            }
        };

        let caps = surface.get_capabilities(&adapter);
        let surface_format = *caps
            .formats
            .first()
            .unwrap_or(&wgpu::TextureFormat::Bgra8UnormSrgb);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        let shader_source = include_str!("../../shaders/render.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
            label: Some("Bind Group Layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[Some(&bg_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[crate::render::geometry::Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        println!("Rust: [CRITICAL_LOG] Entering RenderPipeline::new");

        // На Android ищем видео в ассетах
        let video_path = "background.mp4";
        println!(
            "Rust: [CRITICAL_LOG] Attempting to open video: {}",
            video_path
        );

        let (bg_rgba, bg_w, bg_h, video_player) =
            if let Some(mut player) = VideoPlayer::new(video_path) {
                println!(
                    "Rust: [CRITICAL_LOG] VideoPlayer initialized successfully for {}",
                    video_path
                );
                let (_bg_w, _bg_h) = player.dimensions();
                (image::RgbaImage::new(1, 1), 1u32, 1u32, Some(player))
            } else if let Ok(bg_data) = std::fs::read("background.png") {
                println!("Loading background.png from filesystem...");
                if let Ok(bg_image) = image::load_from_memory(&bg_data) {
                    let bg_rgba = bg_image.to_rgba8();
                    let (bg_w, bg_h) = bg_rgba.dimensions();
                    (bg_rgba, bg_w, bg_h, None)
                } else {
                    (image::RgbaImage::new(1, 1), 1, 1, None)
                }
            } else {
                println!("No background found, using black fallback.");
                (
                    image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 255])),
                    1,
                    1,
                    None,
                )
            };

        let bg_size = wgpu::Extent3d {
            width: bg_w,
            height: bg_h,
            depth_or_array_layers: 1,
        };
        let bg_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: bg_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Background Texture"),
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &bg_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bg_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * bg_w),
                rows_per_image: Some(bg_h),
            },
            bg_size,
        );
        let bg_view = bg_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let blurred_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Blurred Texture"),
            size: wgpu::Extent3d {
                width: (config.width / 4).max(1),
                height: (config.height / 4).max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let blurred_view = blurred_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let scene_blurred_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Scene Blurred Texture"),
            size: wgpu::Extent3d {
                width: (config.width / 4).max(1),
                height: (config.height / 4).max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let scene_blurred_view = scene_blurred_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let scene_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Scene Texture"),
            size: wgpu::Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let scene_texture_view = scene_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // --- LOAD COVER TEXTURE (temp_icon.png) ---
        let cover_data = std::fs::read("assets/temp_icon.png")
            .unwrap_or_else(|_| include_bytes!("../../assets/temp_icon.png").to_vec());
        let cover_image =
            image::load_from_memory(&cover_data).expect("Failed to load temp_icon.png");
        let cover_rgba = cover_image.to_rgba8();
        let (cover_w, cover_h) = cover_rgba.dimensions();

        let cover_size = wgpu::Extent3d {
            width: cover_w,
            height: cover_h,
            depth_or_array_layers: 1,
        };
        let cover_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: cover_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Cover Texture"),
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &cover_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &cover_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * cover_w),
                rows_per_image: Some(cover_h),
            },
            cover_size,
        );
        let cover_texture_view = cover_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&bg_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&blurred_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&cover_texture_view),
                },
            ],
            label: Some("Bind Group"),
        });

        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&scene_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&scene_blurred_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&cover_texture_view),
                },
            ],
            label: Some("Scene Bind Group"),
        });

        let max_vertices = 10000;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (10000 * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Separate UI vertex buffer
        let ui_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Vertex Buffer"),
            size: (10000 * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_blur_ui_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Scene Blur UI Vertex Buffer"),
            size: (10000 * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Vector Icons Pipeline
        let vector_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Vector Pipeline Layout"),
                bind_group_layouts: &[Some(&bg_layout)], // Match main layout to avoid mismatch panic
                immediate_size: 0,
            });

        let vector_shader_source = include_str!("../../shaders/vector_icons.wgsl");
        let vector_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vector Shader"),
            source: wgpu::ShaderSource::Wgsl(vector_shader_source.into()),
        });

        let vector_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Vector Pipeline"),
            layout: Some(&vector_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vector_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &vector_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // --- GRADIENT TEXT SETUP ---
        let gradient_mask_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Gradient Mask Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let gradient_mask_view = gradient_mask_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Uniform buffer for screen size
        let gradient_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gradient Uniform Buffer"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let gradient_shader_source = include_str!("../../shaders/gradient_text.wgsl");
        let gradient_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gradient Text Shader"),
            source: wgpu::ShaderSource::Wgsl(gradient_shader_source.into()),
        });

        let gradient_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Gradient Bind Group Layout"),
        });

        let gradient_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gradient Pipeline Layout"),
            bind_group_layouts: &[Some(&gradient_bind_group_layout)],
            immediate_size: 0,
        });

        let gradient_composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gradient Composite Pipeline"),
            layout: Some(&gradient_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &gradient_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &gradient_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let gradient_composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &gradient_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&gradient_mask_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: gradient_uniform_buffer.as_entire_binding(),
                },
            ],
            label: Some("Gradient Composite Bind Group"),
        });

        // --- GLYPHON INITIALIZATION ---
        let mut font_system = FontSystem::new();
        // Load title font (Plus Jakarta Sans ExtraBold)
        font_system.db_mut().load_font_data(font_data.to_vec());
        // Load UI font (Roboto Bold)
        let roboto_data = include_bytes!("../../fonts/Roboto-Bold.ttf");
        font_system.db_mut().load_font_data(roboto_data.to_vec());
        // Buffer for gradient text (Liquify title)
        let mut gradient_text_buffer = glyphon::Buffer::new(&mut font_system, glyphon::Metrics::new(32.0, 42.0));
        let swash_cache = SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let mut text_atlas = TextAtlas::new(&device, &queue, &cache, config.format);
        let content_text_renderer = TextRenderer::new(
            &mut text_atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );
        let text_viewport = Viewport::new(&device, &cache);

        // Blur Pipeline
        let blur_shader_source = include_str!("../../shaders/Sgausblur.wgsl");
        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blur Shader"),
            source: wgpu::ShaderSource::Wgsl(blur_shader_source.into()),
        });

        let blur_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Blur Bind Group Layout"),
        });

        let blur_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blur Pipeline Layout"),
            bind_group_layouts: &[Some(&blur_bg_layout)],
            immediate_size: 0,
        });

        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blur Pipeline"),
            layout: Some(&blur_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blur_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blur_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vector_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vector Vertex Buffer"),
            size: (10000 * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let blur_params_buffer_h = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Blur Params Buffer H"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let blur_params_buffer_v = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Blur Params Buffer V"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let temp_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Temp Texture"),
            size: wgpu::Extent3d {
                width: (config.width / 4).max(1),
                height: (config.height / 4).max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let temp_view = temp_texture.create_view(&wgpu::TextureViewDescriptor::default());

        RenderPipeline {
            surface,
            device,
            queue,
            config,
            pipeline,
            sampler,
            background_texture: bg_texture,
            background_texture_view: bg_view,
            bind_group,
            scene_texture,
            scene_texture_view,
            scene_bind_group,
            vertex_buffer,
            vertex_count: 0,
            content_vertex_count: 0,
            ui_vertex_count: 0,
            scene_blur_ui_vertex_count: 0,
            ui_vertex_buffer,
            scene_blur_ui_vertex_buffer,
            scale_factor,

            blur_pipeline,
            blur_params_buffer_h,
            blur_params_buffer_v,
            blurred_texture,
            blurred_texture_view: blurred_view,
            scene_blurred_texture,
            scene_blurred_texture_view: scene_blurred_view,
            temp_texture,
            temp_texture_view: temp_view,

            _cover_texture: cover_texture,
            cover_texture_view,

            vector_pipeline,
            vector_vertex_buffer,
            vector_vertex_count: 0,
            content_vector_vertex_count: 0,
            ui_vector_vertex_count: 0,

            // --- GLYPHON ---
            font_system,
            swash_cache,
            text_atlas,
            content_text_renderer,
            text_renderer,
            text_viewport,
            content_text_buffer_pool: Vec::new(),
            text_buffer_pool: Vec::new(),
            content_text_requests: Vec::new(),
            text_requests: Vec::new(),
            gradient_text_requests: Vec::new(),
            gradient_text_buffer: gradient_text_buffer,

            // Temp vertex accumulation buffers
            content_vertices: Vec::new(),
            ui_vertices: Vec::new(),
            scene_blur_ui_vertices: Vec::new(),
            content_vector_vertices: Vec::new(),
            ui_vector_vertices: Vec::new(),

            bg_layout,

            video_player,
            gradient_mask_texture,
            gradient_mask_view,
            gradient_composite_pipeline,
            gradient_composite_bind_group,
            gradient_uniform_buffer,

            #[cfg(target_os = "android")]
            ycbcr_pipeline: None,
            #[cfg(target_os = "android")]
            ycbcr_cmd_resources: None,
            #[cfg(target_os = "android")]
            ycbcr_output_texture: None,
            #[cfg(target_os = "android")]
            ycbcr_output_view: None,
            #[cfg(target_os = "android")]
            #[cfg(target_os = "android")]
            ycbcr_ahb_props: None,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.scale_factor = scale_factor;
            self.surface.configure(&self.device, &self.config);

            // Recreate blur textures (Downsampled 4x for performance)
            let blurred_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Blurred Texture"),
                size: wgpu::Extent3d {
                    width: (self.config.width / 4).max(1),
                    height: (self.config.height / 4).max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let blurred_view = blurred_texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.blurred_texture = blurred_texture;
            self.blurred_texture_view = blurred_view;

            let scene_blurred_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Scene Blurred Texture"),
                size: wgpu::Extent3d {
                    width: (self.config.width / 4).max(1),
                    height: (self.config.height / 4).max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let scene_blurred_view = scene_blurred_texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.scene_blurred_texture = scene_blurred_texture;
            self.scene_blurred_texture_view = scene_blurred_view;

            let temp_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Temp Texture"),
                size: wgpu::Extent3d {
                    width: (self.config.width / 4).max(1),
                    height: (self.config.height / 4).max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let temp_view = temp_texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.temp_texture = temp_texture;
            self.temp_texture_view = temp_view;

            let scene_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Scene Texture"),
                size: wgpu::Extent3d {
                    width: self.config.width.max(1),
                    height: self.config.height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let scene_texture_view = scene_texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.scene_texture = scene_texture;
            self.scene_texture_view = scene_texture_view;

            // Recreate gradient mask texture with correct format
            let gradient_mask_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Gradient Mask Texture"),
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let gradient_mask_view = gradient_mask_texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.gradient_mask_texture = gradient_mask_texture;
            self.gradient_mask_view = gradient_mask_view;

            // Recreate gradient composite bind group with new view
            self.gradient_composite_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.gradient_composite_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.gradient_mask_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.gradient_uniform_buffer.as_entire_binding(),
                    },
                ],
                label: Some("Gradient Composite Bind Group"),
            });

            // Recreate main bind group (existing code)
            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bg_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.background_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.blurred_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.cover_texture_view),
                    },
                ],
                label: Some("Bind Group"),
            });

            self.scene_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bg_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.scene_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.scene_blurred_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.cover_texture_view),
                    },
                ],
                label: Some("Scene Bind Group"),
            });
        }
    }

    pub fn draw(&mut self) {
        // --- VIDEO BACKGROUND UPDATE ---
        if let Some(ref mut player) = self.video_player {
            if let Some(frame) = player.next_frame() {
                match frame {
                    VideoFrame::Rgba(rgba, w, h) => {
                        self.queue.write_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture: &self.background_texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            &rgba,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(w * 4),
                                rows_per_image: Some(h),
                            },
                            wgpu::Extent3d {
                                width: w,
                                height: h,
                                depth_or_array_layers: 1,
                            },
                        );
                    }
                    #[cfg(target_os = "android")]
                    VideoFrame::HardwareBuffer(ptr) => {
                        #[cfg(target_os = "android")]
                        {
                            use wgpu_hal as hal;

                            let (width, height) = {
                                let player = self.video_player.as_ref().unwrap();
                                player.dimensions()
                            };

                             // Ленивая инициализация YCbCr пайплайна
                             // (нужны данные первого кадра для external_format)
                             #[cfg(target_os = "android")]
                             if self.ycbcr_pipeline.is_none() {
                                unsafe {
                                    // Получить properties первого кадра
                                    let Some(hal_device) = self.device.as_hal::<hal::api::Vulkan>()
                                    else {
                                        return;
                                    };
                                    let raw_device = hal_device.raw_device();
                                    let raw_instance = hal_device.shared_instance().raw_instance();
                                    let pd = hal_device.raw_physical_device();

                                    // Запросить properties первого AHB
                                    if let Some((ext_format, format_props)) =
                                        crate::video::vulkan_import::query_ahb_properties(
                                            raw_device, ptr,
                                        )
                                    {
                                        if let Some(pipeline) =
                                            crate::video::raw_vulkan_ycbcr::RawYcbcrPipeline::new(
                                                raw_instance,
                                                pd,
                                                raw_device,
                                                width,
                                                height,
                                                ext_format,
                                                &format_props,
                                            )
                                        {
                                            // Создаём output texture ОДИН РАЗ здесь
                                            // output_image уже создан внутри pipeline
                                            // Оборачиваем его в wgpu texture
                                            let output_tex = {
                                                let hal_tex = hal_device.texture_from_raw(
                                                    pipeline.output_image,
                                                    &hal::TextureDescriptor {
                                                        label: Some("YCbCr Output"),
                                                        size: wgpu::Extent3d {
                                                            width,
                                                            height,
                                                            depth_or_array_layers: 1,
                                                        },
                                                        mip_level_count: 1,
                                                        sample_count: 1,
                                                        dimension: wgpu::TextureDimension::D2,
                                                        format: wgpu::TextureFormat::Rgba8Unorm,
                                                        usage: wgpu::TextureUses::RESOURCE,
                                                        memory_flags: hal::MemoryFlags::empty(),
                                                        view_formats: vec![],
                                                    },
                                                    // None = wgpu не владеет памятью (мы управляем сами)
                                                    None,
                                                    // Передаём None для памяти — pipeline владеет output_memory
                                                    hal::vulkan::TextureMemory::External,
                                                );
                                                self.device.create_texture_from_hal::<hal::api::Vulkan>(
                                                    hal_tex,
                                                    &wgpu::TextureDescriptor {
                                                        label: Some("YCbCr Output"),
                                                        size: wgpu::Extent3d {
                                                            width, height, depth_or_array_layers: 1,
                                                        },
                                                        mip_level_count: 1,
                                                        sample_count: 1,
                                                        dimension: wgpu::TextureDimension::D2,
                                                        format: wgpu::TextureFormat::Rgba8Unorm,
                                                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                                                        view_formats: &[],
                                                    },
                                                )
                                            };

                                            let output_view = output_tex.create_view(
                                                &wgpu::TextureViewDescriptor::default(),
                                            );

                                            self.ycbcr_output_texture = Some(output_tex);
                                            self.ycbcr_output_view = Some(output_view);
                                            self.ycbcr_pipeline = Some(pipeline);

                                            // Создаём command resources для YCbCr
                                            let queue_family_index =
                                                hal_device.queue_family_index();
                                            let raw_queue = hal_device.raw_queue();
                                            if let Some(cmd_res) = crate::video::raw_vulkan_ycbcr::YcbcrCommandResources::new(
                                                    raw_device,
                                                    queue_family_index,
                                                    raw_queue,
                                                ) {
                                                self.ycbcr_cmd_resources = Some(cmd_res);
                                            }

                                            // Обновляем bind group один раз.
                                            // bg_layout требует все 5 биндингов (0-4).
                                            self.bind_group =
                                                self.device
                                                    .create_bind_group(&wgpu::BindGroupDescriptor {
                                                    layout: &self.bg_layout,
                                                    entries: &[
                                                        wgpu::BindGroupEntry {
                                                            binding: 0,
                                                            resource:
                                                                wgpu::BindingResource::Sampler(
                                                                    &self.sampler,
                                                                ),
                                                        },
                                                        wgpu::BindGroupEntry {
                                                            binding: 1,
                                                            resource:
                                                                wgpu::BindingResource::TextureView(
                                                                    self.ycbcr_output_view
                                                                        .as_ref()
                                                                        .unwrap(),
                                                                ),
                                                        },
                                                        wgpu::BindGroupEntry {
                                                            binding: 2,
                                                            resource:
                                                                wgpu::BindingResource::TextureView(
                                                                    &self.blurred_texture_view,
                                                                ),
                                                        },
                                                        wgpu::BindGroupEntry {
                                                            binding: 3,
                                                                resource:
                                                                    wgpu::BindingResource::TextureView(
                                                                        &self.cover_texture_view,
                                                                    ),
                                                                },
                                                            ],
                                                            label: Some("YCbCr Bind Group"),
                                                });

                                            log::info!("YCbCr pipeline initialized OK");
                                        }
                                    }
                                }
                            }

                            // Обработка с YCbCr пайплайном (если инициализирована)
                            // ВАЖНО: сначала разрешаем ahb_props (нужен immutable borrow на
                            // ycbcr_pipeline), и только потом берём мутабельный borrow.
                            //
                            // LIFECYCLE NOTE: ptr — это *borrowed* указатель из AndroidHwBackend.
                            // AndroidHwBackend::next_frame() держит AHB через current_hb и
                             // освободит его сам при следующем вызове next_frame().
                             // НЕ вызываем AHardwareBuffer_release здесь — это вызовет double-free.
                             #[cfg(target_os = "android")]
                             if self.ycbcr_pipeline.is_some() && self.ycbcr_cmd_resources.is_some() {
                                unsafe {
                                    let Some(hal_device) = self.device.as_hal::<hal::api::Vulkan>()
                                    else {
                                        // Не освобождаем ptr — AndroidHwBackend владеет им
                                        return;
                                    };
                                    let raw_device = hal_device.raw_device();

                                    // Шаг 1: разрешаем ahb_props — только immutable borrow.
                                    // Если ещё не закэшированы, запрашиваем через ycbcr_pipeline.
                                    if self.ycbcr_ahb_props.is_none() {
                                        let query_result = self
                                            .ycbcr_pipeline
                                            .as_ref()
                                            .unwrap()
                                            .query_ahb_properties_struct(raw_device, ptr);
                                        match query_result {
                                            Some((_ext_fmt, props)) => {
                                                self.ycbcr_ahb_props = Some(props);
                                            }
                                            None => {
                                                log::error!(
                                                    "query_ahb_properties_struct returned None, skipping frame"
                                                );
                                                // Не освобождаем ptr — AndroidHwBackend владеет им
                                                return;
                                            }
                                        }
                                    }

                                    // Шаг 2: копируем значения из кэша, чтобы не держать
                                    // ссылку на self при мутабельном borrow ниже.
                                    let ahb_props_copy =
                                        self.ycbcr_ahb_props.as_ref().unwrap().clone();
                                    let ext_format = ahb_props_copy.external_format;

                                    // Шаг 3: теперь можно безопасно взять мутабельные ссылки.
                                    let ycbcr = self.ycbcr_pipeline.as_mut().unwrap();
                                    let cmd_res = self.ycbcr_cmd_resources.as_ref().unwrap();

                                    // begin_frame ждёт fence прошлого кадра (GPU finished)
                                    cmd_res.begin_frame(raw_device);

                                    // Записываем YCbCr AHB → output_image (RGBA)
                                    ycbcr.process_ahb_frame(
                                        raw_device,
                                        cmd_res.command_buffer,
                                        ptr,
                                        ext_format,
                                        &ahb_props_copy,
                                    );

                                    // Submit на GPU; fence сигналится когда blit готов.
                                    // wgpu blur/main pass читают output_image ПОСЛЕ этого submit'а —
                                    // синхронизация через output_image layout
                                    // (UNDEFINED→SHADER_READ_ONLY в record_commands).
                                    cmd_res.submit(raw_device);

                                    // НЕ вызываем AHardwareBuffer_release —
                                    // AndroidHwBackend::current_hb освободит его при следующем next_frame()
                                }
                            } else {
                                // Fallback: импортируем AHB напрямую в wgpu текстуру.
                                // import_android_buffer сам делает acquire внутри —
                                // нам release тоже не нужен здесь, AndroidHwBackend освободит.
                                if let Some(texture) = unsafe {
                                    crate::video::android_hw::import_android_buffer(
                                        &self.device,
                                        &self.queue,
                                        ptr,
                                        self.config.width,
                                        self.config.height,
                                    )
                                } {
                                    self.background_texture = texture;
                                    self.background_texture_view = self
                                        .background_texture
                                        .create_view(&wgpu::TextureViewDescriptor::default());

                                    self.bind_group =
                                        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                            layout: &self.bg_layout,
                                            entries: &[
                                                wgpu::BindGroupEntry {
                                                    binding: 0,
                                                    resource: wgpu::BindingResource::Sampler(
                                                        &self.sampler,
                                                    ),
                                                },
                                                wgpu::BindGroupEntry {
                                                    binding: 1,
                                                    resource: wgpu::BindingResource::TextureView(
                                                        &self.background_texture_view,
                                                    ),
                                                },
                                                wgpu::BindGroupEntry {
                                                    binding: 2,
                                                    resource: wgpu::BindingResource::TextureView(
                                                        &self.blurred_texture_view,
                                                    ),
                                                },
                                                wgpu::BindGroupEntry {
                                                    binding: 3,
                                                    resource: wgpu::BindingResource::TextureView(
                                                        &self.cover_texture_view,
                                                    ),
                                                },
                                            ],
                                            label: Some("Fallback Bind Group"),
                                        });
                                }
                                // НЕ вызываем AHardwareBuffer_release —
                                // AndroidHwBackend::current_hb освободит его при следующем next_frame()
                            }
                        }
                    }
                }
            }
        }

        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            _ => return,
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.text_viewport.update(
            &self.queue,
            Resolution {
                width: self.config.width,
                height: self.config.height,
            },
        );

        // Plain background blur is used by normal glass elements.
        self.do_background_blur_passes(&mut encoder);

        // === CONTENT LAYER PASS ===
        self.render_content_layer(&mut encoder);

        // Blur the fully rendered page so fixed UI glass sees content under it.
        self.do_scene_blur_passes(&mut encoder);

        self.render_scene_to_surface(&mut encoder, &view);

        // === UI LAYER PASS ===
        self.render_ui_layer(&mut encoder, &view);

        // --- GRADIENT TEXT MASK + COMPOSITE (ON TOP) ---
        // Gradient pass is currently disabled until the compositor method is restored.

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    fn do_background_blur_passes(&mut self, encoder: &mut wgpu::CommandEncoder) {
        {
            let blur_w = (self.config.width / 4).max(1);
            let blur_h = (self.config.height / 4).max(1);

            let blur_params = BlurParams {
                texel_size: [1.0 / blur_w as f32, 1.0 / blur_h as f32],
                radius: 10.0,
                direction: 0.0,
            };
            self.queue.write_buffer(
                &self.blur_params_buffer_h,
                0,
                bytemuck::bytes_of(&blur_params),
            );

            let blur_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.blur_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView({
                            #[cfg(target_os = "android")]
                            let v = self.ycbcr_output_view.as_ref().unwrap_or(&self.background_texture_view);
                            #[cfg(not(target_os = "android"))]
                            let v = &self.background_texture_view;
                            v
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.blur_params_buffer_h.as_entire_binding(),
                    },
                ],
                label: Some("Background Blur Horizontal"),
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Background Blur Horizontal Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.temp_texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        {
            let blur_w = (self.config.width / 4).max(1);
            let blur_h = (self.config.height / 4).max(1);

            let blur_params = BlurParams {
                texel_size: [1.0 / blur_w as f32, 1.0 / blur_h as f32],
                radius: 10.0,
                direction: 1.0,
            };
            self.queue.write_buffer(
                &self.blur_params_buffer_v,
                0,
                bytemuck::bytes_of(&blur_params),
            );

            let blur_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.blur_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.temp_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.blur_params_buffer_v.as_entire_binding(),
                    },
                ],
                label: Some("Background Blur Vertical"),
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Background Blur Vertical Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.blurred_texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_bg, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    // Helper: perform horizontal+vertical blur for the rendered page.
    fn do_scene_blur_passes(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let blur_w = (self.config.width / 4).max(1);
        let blur_h = (self.config.height / 4).max(1);
        let scissor_rects = self.scene_blur_scissor_rects(blur_w, blur_h);
        if scissor_rects.is_empty() {
            return;
        }

        // --- BLUR PASS 1 (Horizontal) ---
        {
            let blur_params = BlurParams {
                texel_size: [1.0 / blur_w as f32, 1.0 / blur_h as f32],
                radius: 10.0,
                direction: 0.0,
            };
            self.queue.write_buffer(
                &self.blur_params_buffer_h,
                0,
                bytemuck::bytes_of(&blur_params),
            );

            let blur_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.blur_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.scene_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.blur_params_buffer_h.as_entire_binding(),
                    },
                ],
                label: Some("Blur BG Horizontal"),
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blur Horizontal Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.temp_texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_bg, &[]);
            for rect in &scissor_rects {
                pass.set_scissor_rect(rect.x, rect.y, rect.w, rect.h);
                pass.draw(0..3, 0..1);
            }
        }

        // --- BLUR PASS 2 (Vertical) ---
        {
            let blur_params = BlurParams {
                texel_size: [1.0 / blur_w as f32, 1.0 / blur_h as f32],
                radius: 10.0,
                direction: 1.0,
            };
            self.queue.write_buffer(
                &self.blur_params_buffer_v,
                0,
                bytemuck::bytes_of(&blur_params),
            );

            let blur_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.blur_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.temp_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.blur_params_buffer_v.as_entire_binding(),
                    },
                ],
                label: Some("Blur BG Vertical"),
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blur Vertical Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_blurred_texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_bg, &[]);
            for rect in &scissor_rects {
                pass.set_scissor_rect(rect.x, rect.y, rect.w, rect.h);
                pass.draw(0..3, 0..1);
            }
        }
    }

    fn scene_blur_scissor_rects(&self, blur_w: u32, blur_h: u32) -> Vec<ScissorRect> {
        let mut rects = Vec::new();
        let margin = 24.0;

        for quad in self.scene_blur_ui_vertices.chunks(6) {
            if quad.len() < 6 {
                continue;
            }

            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;

            for v in quad {
                let x = ((v.position[0] + 1.0) * 0.5) * blur_w as f32;
                let y = ((1.0 - v.position[1]) * 0.5) * blur_h as f32;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }

            let x0 = (min_x - margin).floor().max(0.0) as u32;
            let y0 = (min_y - margin).floor().max(0.0) as u32;
            let x1 = (max_x + margin).ceil().min(blur_w as f32) as u32;
            let y1 = (max_y + margin).ceil().min(blur_h as f32) as u32;

            if x1 > x0 && y1 > y0 {
                rects.push(ScissorRect {
                    x: x0,
                    y: y0,
                    w: x1 - x0,
                    h: y1 - y0,
                });
            }
        }

        rects
    }

    fn render_scene_to_surface(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let scene_quad = [
            Vertex { position: [-1.0,  1.0], tex_coord: [0.0, 0.0], screen_uv: [0.0, 0.0], color: [1.0, 1.0, 1.0, -1.0], size: [self.config.width as f32, self.config.height as f32], radii: [0.0; 4] },
            Vertex { position: [ 1.0,  1.0], tex_coord: [1.0, 0.0], screen_uv: [1.0, 0.0], color: [1.0, 1.0, 1.0, -1.0], size: [self.config.width as f32, self.config.height as f32], radii: [0.0; 4] },
            Vertex { position: [ 1.0, -1.0], tex_coord: [1.0, 1.0], screen_uv: [1.0, 1.0], color: [1.0, 1.0, 1.0, -1.0], size: [self.config.width as f32, self.config.height as f32], radii: [0.0; 4] },
            Vertex { position: [-1.0,  1.0], tex_coord: [0.0, 0.0], screen_uv: [0.0, 0.0], color: [1.0, 1.0, 1.0, -1.0], size: [self.config.width as f32, self.config.height as f32], radii: [0.0; 4] },
            Vertex { position: [ 1.0, -1.0], tex_coord: [1.0, 1.0], screen_uv: [1.0, 1.0], color: [1.0, 1.0, 1.0, -1.0], size: [self.config.width as f32, self.config.height as f32], radii: [0.0; 4] },
            Vertex { position: [-1.0, -1.0], tex_coord: [0.0, 1.0], screen_uv: [0.0, 1.0], color: [1.0, 1.0, 1.0, -1.0], size: [self.config.width as f32, self.config.height as f32], radii: [0.0; 4] },
        ];
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&scene_quad));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Scene Composite Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.scene_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..6, 0..1);
    }

    // Helper: render content layer (everything below UI glass)
    fn render_content_layer(&mut self, encoder: &mut wgpu::CommandEncoder) {
        // Write content geometry buffers (already filled by build_frame_ecs)
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.content_vertices[..self.content_vertex_count as usize]));
        self.queue.write_buffer(&self.vector_vertex_buffer, 0, bytemuck::cast_slice(&self.content_vector_vertices[..self.content_vector_vertex_count as usize]));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Content Layer Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.scene_texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.content_vertex_count, 0..1);

        if self.content_vector_vertex_count > 0 {
            pass.set_pipeline(&self.vector_pipeline);
            pass.set_vertex_buffer(0, self.vector_vertex_buffer.slice(..));
            pass.draw(0..self.content_vector_vertex_count, 0..1);
        }

        if self.content_text_requests.is_empty() {
            return;
        }

        // Render content text in content pass, so UI layer always stays above it.
        while self.content_text_buffer_pool.len() < self.content_text_requests.len() {
            self.content_text_buffer_pool.push(ManagedBuffer {
                buffer: Buffer::new(&mut self.font_system, Metrics::new(16.0, 22.0)),
                last_text: String::new(),
                last_scale: 0.0,
            });
        }

        let mut text_areas = Vec::new();
        let font_system = &mut self.font_system;
        for ((text, x, y, scale, color), managed) in self
            .content_text_requests
            .iter()
            .zip(self.content_text_buffer_pool.iter_mut())
        {
            let logical_size = *scale;
            if managed.last_text != *text || managed.last_scale != logical_size {
                managed
                    .buffer
                    .set_metrics(font_system, Metrics::new(logical_size, logical_size * 1.35));
                managed.buffer.set_size(
                    font_system,
                    Some(self.config.width as f32 / self.scale_factor),
                    Some(self.config.height as f32 / self.scale_factor),
                );
                let adjusted_text = text.replace(' ', "\u{2009}");
                managed.buffer.set_text(
                    font_system,
                    &adjusted_text,
                    &Attrs::new().family(Family::Name("Roboto")),
                    Shaping::Basic,
                    None,
                );
                managed.buffer.shape_until_scroll(font_system, false);
                managed.last_text = text.clone();
                managed.last_scale = logical_size;
            }
            text_areas.push(glyphon::TextArea {
                buffer: &managed.buffer,
                left: *x,
                top: *y,
                scale: self.scale_factor,
                bounds: TextBounds {
                    left: -4096,
                    top: -4096,
                    right: self.config.width as i32 + 4096,
                    bottom: self.config.height as i32 + 4096,
                },
                default_color: glyphon::Color::rgba(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                    (color[3] * 255.0) as u8,
                ),
                custom_glyphs: &[],
            });
        }
        self.content_text_renderer
            .prepare(
                &self.device,
                &self.queue,
                font_system,
                &mut self.text_atlas,
                &self.text_viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();
        self.content_text_renderer
            .render(&self.text_atlas, &self.text_viewport, &mut pass)
            .unwrap();
    }

    fn render_ui_layer(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        // Prepare text for UI layer
        while self.text_buffer_pool.len() < self.text_requests.len() {
            self.text_buffer_pool.push(ManagedBuffer {
                buffer: Buffer::new(&mut self.font_system, Metrics::new(16.0, 22.0)),
                last_text: String::new(),
                last_scale: 0.0,
            });
        }

        let mut text_areas = Vec::new();
        let font_system = &mut self.font_system;

        for ((text, x, y, scale, color), managed) in self.text_requests.iter().zip(self.text_buffer_pool.iter_mut()) {
            let logical_size = *scale;
            if managed.last_text != *text || managed.last_scale != logical_size {
                managed.buffer.set_metrics(font_system, Metrics::new(logical_size, logical_size * 1.35));
                managed.buffer.set_size(
                    font_system,
                    Some(self.config.width as f32 / self.scale_factor),
                    Some(self.config.height as f32 / self.scale_factor),
                );
                let adjusted_text = text.replace(' ', "\u{2009}");
                managed.buffer.set_text(
                    font_system,
                    &adjusted_text,
                    &Attrs::new().family(Family::Name("Roboto")),
                    Shaping::Basic,
                    None,
                );
                managed.buffer.shape_until_scroll(font_system, false);
                managed.last_text = text.clone();
                managed.last_scale = logical_size;
            }
            text_areas.push(glyphon::TextArea {
                buffer: &managed.buffer,
                left: *x,
                top: *y,
                scale: self.scale_factor,
                bounds: TextBounds {
                    left: -4096,
                    top: -4096,
                    right: self.config.width as i32 + 4096,
                    bottom: self.config.height as i32 + 4096,
                },
                default_color: glyphon::Color::rgba(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                    (color[3] * 255.0) as u8,
                ),
                custom_glyphs: &[],
            });
        }

        // Upload UI geometry to dedicated UI vertex buffer
        self.queue.write_buffer(&self.ui_vertex_buffer, 0, bytemuck::cast_slice(&self.ui_vertices[..self.ui_vertex_count as usize]));
        self.queue.write_buffer(&self.scene_blur_ui_vertex_buffer, 0, bytemuck::cast_slice(&self.scene_blur_ui_vertices[..self.scene_blur_ui_vertex_count as usize]));
        // Upload UI vector vertices to shared vector buffer (overwrite content vectors)
        self.queue.write_buffer(&self.vector_vertex_buffer, 0, bytemuck::cast_slice(&self.ui_vector_vertices[..self.ui_vector_vertex_count as usize]));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UI Layer Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        // Draw UI glass + icons first
        pass.set_pipeline(&self.pipeline);
        if self.scene_blur_ui_vertex_count > 0 {
            pass.set_bind_group(0, &self.scene_bind_group, &[]);
            pass.set_vertex_buffer(0, self.scene_blur_ui_vertex_buffer.slice(..));
            pass.draw(0..self.scene_blur_ui_vertex_count, 0..1);
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.ui_vertex_buffer.slice(..));
        pass.draw(0..self.ui_vertex_count, 0..1);

        if self.ui_vector_vertex_count > 0 {
            pass.set_pipeline(&self.vector_pipeline);
            pass.set_vertex_buffer(0, self.vector_vertex_buffer.slice(..));
            pass.draw(0..self.ui_vector_vertex_count, 0..1);
        }

        // Then render UI text on top
        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                font_system,
                &mut self.text_atlas,
                &self.text_viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();
        self.text_renderer
            .render(&self.text_atlas, &self.text_viewport, &mut pass)
            .unwrap();
    }
}
