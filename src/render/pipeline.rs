use crate::ecs::components::*;
use crate::ecs::world::World;
use crate::font::FontAtlas;
use crate::input::InputState;
use crate::render::geometry::{Vertex, build_background_quad, build_glass_quad, build_shadow_quad};
use crate::video::{VideoPlayer, VideoFrame};
use glyphon::{
    Attrs, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextAtlas, TextBounds,
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

pub struct RenderPipeline {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
    pub sampler: wgpu::Sampler,
    pub font_texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub background_texture: wgpu::Texture,
    pub background_texture_view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub font_atlas: FontAtlas,
    pub max_vertices: usize,
    pub scale_factor: f32,

    pub blur_pipeline: wgpu::RenderPipeline,
    pub blur_params_buffer_h: wgpu::Buffer,
    pub blur_params_buffer_v: wgpu::Buffer,
    pub blurred_texture: wgpu::Texture,
    pub blurred_texture_view: wgpu::TextureView,
    pub temp_texture: wgpu::Texture,
    pub temp_texture_view: wgpu::TextureView,

    pub icon_texture: wgpu::Texture,
    pub icon_texture_view: wgpu::TextureView,

    pub cover_texture: wgpu::Texture,
    pub cover_texture_view: wgpu::TextureView,

    pub vector_pipeline: wgpu::RenderPipeline,
    pub vector_vertex_buffer: wgpu::Buffer,
    pub vector_vertex_count: u32,

    // --- GLYPHON ---
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub text_atlas: TextAtlas,
    pub text_renderer: TextRenderer,
    pub text_viewport: Viewport,
    pub text_buffer_pool: Vec<ManagedBuffer>,
    pub text_requests: Vec<(String, f32, f32, f32, [f32; 4])>,

    pub blur_bg_layout: wgpu::BindGroupLayout,
    pub bg_layout: wgpu::BindGroupLayout,
    pub video_player: Option<VideoPlayer>,
}

impl RenderPipeline {
    pub async fn new(
        window: &winit::window::Window,
        font_data: &[u8],
        atlas_png: &[u8],
        atlas_json: &str,
    ) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
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

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .unwrap();

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
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
            bind_group_layouts: &[&bg_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[crate::render::geometry::Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
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
            multiview: None,
            cache: None,
        });

        let font_atlas = FontAtlas::load(font_data, atlas_png, atlas_json);
        let font_texture = font_atlas.create_texture(&device);
        font_atlas.upload_texture(&font_texture, &device, &queue);
        let font_view = font_texture.create_view(&wgpu::TextureViewDescriptor::default());

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
        println!("Rust: [CRITICAL_LOG] Attempting to open video: {}", video_path);

        let (bg_rgba, bg_w, bg_h, video_player) = if let Some(mut player) = VideoPlayer::new(video_path) {
            println!("Rust: [CRITICAL_LOG] VideoPlayer initialized successfully for {}", video_path);
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
            (image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 255])), 1, 1, None)
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
            wgpu::ImageCopyTexture {
                texture: &bg_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bg_rgba,
            wgpu::ImageDataLayout {
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

        // Icon texture (Stub - no longer used for atlas)
        let icon_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Icon Texture Stub"),
            view_formats: &[],
        });
        let icon_texture_view = icon_texture.create_view(&wgpu::TextureViewDescriptor::default());

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
            wgpu::ImageCopyTexture {
                texture: &cover_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &cover_rgba,
            wgpu::ImageDataLayout {
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
                    resource: wgpu::BindingResource::TextureView(&icon_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&cover_texture_view),
                },
            ],
            label: Some("Bind Group"),
        });

        let max_vertices = 10000;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Vector Icons Pipeline
        let vector_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Vector Pipeline Layout"),
                bind_group_layouts: &[&bg_layout], // Match main layout to avoid mismatch panic
                push_constant_ranges: &[],
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
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &vector_shader,
                entry_point: "fs_main",
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
            multiview: None,
            cache: None,
        });

        // --- GLYPHON INITIALIZATION ---
        let mut font_system = FontSystem::new();
        font_system.db_mut().load_font_data(font_data.to_vec());
        let swash_cache = SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let mut text_atlas = TextAtlas::new(&device, &queue, &cache, config.format);
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
            bind_group_layouts: &[&blur_bg_layout],
            push_constant_ranges: &[],
        });

        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blur Pipeline"),
            layout: Some(&blur_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blur_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blur_shader,
                entry_point: "fs_main",
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
            multiview: None,
            cache: None,
        });

        let vector_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vector Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<Vertex>()) as u64,
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
            font_texture,
            texture_view: font_view,
            background_texture: bg_texture,
            background_texture_view: bg_view,
            bind_group,
            vertex_buffer,
            vertex_count: 0,
            font_atlas,
            max_vertices,
            blur_pipeline,
            blur_params_buffer_h,
            blur_params_buffer_v,
            blurred_texture,
            blurred_texture_view: blurred_view,
            temp_texture,
            temp_texture_view: temp_view,
            icon_texture,
            icon_texture_view,
            cover_texture,
            cover_texture_view,
            vector_pipeline,
            vector_vertex_buffer,
            vector_vertex_count: 0,

            // --- GLYPHON ---
            font_system,
            swash_cache,
            text_atlas,
            text_renderer,
            text_viewport,
            text_buffer_pool: Vec::new(),
            text_requests: Vec::new(),

            blur_bg_layout,
            bg_layout,
            scale_factor,
            video_player,
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

            // Recreate main bind group with correct 5 entries (0-4)
            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 2,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 3,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 4,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                        ],
                        label: Some("Bind Group Layout"),
                    }),
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
                        resource: wgpu::BindingResource::TextureView(&self.icon_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&self.cover_texture_view),
                    },
                ],
                label: Some("Bind Group"),
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
                            wgpu::ImageCopyTexture {
                                texture: &self.background_texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            &rgba,
                            wgpu::ImageDataLayout {
                                offset: 0,
                                bytes_per_row: Some(w * 4),
                                rows_per_image: Some(h),
                            },
                            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                        );
                    },
                    VideoFrame::HardwareBuffer(ptr) => {
                        #[cfg(target_os = "android")]
                        {
                            if let Some(texture) = unsafe {
                                crate::video::android_hw::import_android_buffer(&self.device, &self.queue, ptr, self.config.width, self.config.height)
                            } {
                                self.background_texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                                
                                // Recreate bind group to point to the new texture
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
                                            resource: wgpu::BindingResource::TextureView(&self.icon_texture_view),
                                        },
                                        wgpu::BindGroupEntry {
                                            binding: 4,
                                            resource: wgpu::BindingResource::TextureView(&self.cover_texture_view),
                                        },
                                    ],
                                    label: Some("Dynamic Bind Group"),
                                });
                            }
                            // Освобождаем нашу ссылку на AHardwareBuffer (Vulkan import сделал свою)
                            unsafe { ndk_sys::AHardwareBuffer_release(ptr as *mut _); }
                        }
                    }
                }
            }
        }

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => return,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // --- BLUR PASS 1 (Horizontal) ---
        // background_texture -> temp_texture (Downsampled 4x)
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
                        resource: wgpu::BindingResource::TextureView(&self.background_texture_view),
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
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // --- BLUR PASS 2 (Vertical) ---
        // temp_texture -> blurred_texture
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
                label: Some("Blur BG Vertical"),
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blur Vertical Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.blurred_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // --- MAIN PASS ---
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..self.vertex_count, 0..1);

            if self.vector_vertex_count > 0 {
                pass.set_pipeline(&self.vector_pipeline);
                // No bind group needed for vector pipeline
                pass.set_vertex_buffer(0, self.vector_vertex_buffer.slice(..));
                pass.draw(0..self.vector_vertex_count, 0..1);
            }

            // --- GLYPHON RENDERING ---
            // Подготавливаем вьюпорт
            self.text_viewport.update(
                &self.queue,
                Resolution {
                    width: self.config.width,
                    height: self.config.height,
                },
            );

            // Наполняем буферы из пула
            while self.text_buffer_pool.len() < self.text_requests.len() {
                self.text_buffer_pool.push(ManagedBuffer {
                    buffer: glyphon::Buffer::new(&mut self.font_system, Metrics::new(32.0, 42.0)),
                    last_text: String::new(),
                    last_scale: 0.0,
                });
            }

            let mut text_areas = Vec::new();
            let font_system = &mut self.font_system;

            for ((text, x, y, scale, color), managed) in self
                .text_requests
                .iter()
                .zip(self.text_buffer_pool.iter_mut())
            {
                // Шейпим ТОЛЬКО если текст или масштаб изменились
                // ВАЖНО: Metrics должны быть в логических пикселях,
                // так как glyphon::TextArea::scale применит scale_factor при растеризации.
                // ВАЖНО: Теперь scale — это и есть размер шрифта в логических пикселях.
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
                    // Используем Thin Space (\u2009) для сужения пробелов (примерно 0.8 от обычного)
                    let adjusted_text = text.replace(' ', "\u{2009}");
                    managed.buffer.set_text(
                        font_system,
                        &adjusted_text,
                        Attrs::new().family(Family::SansSerif),
                        Shaping::Basic,
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
                        left: 0,
                        top: 0,
                        right: self.config.width as i32,
                        bottom: self.config.height as i32,
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

            // Важно: text_areas содержит ссылки на буферы, поэтому мы должны использовать их
            // до того, как цикл завершится или буферы будут перемещены.
            // Но в glyphon::TextArea время жизни привязано к буферу.
            // Нам нужно вызвать prepare ПРЯМО ЗДЕСЬ, пока ссылки живы.

            self.text_renderer
                .prepare(
                    &self.device,
                    &self.queue,
                    &mut self.font_system,
                    &mut self.text_atlas,
                    &self.text_viewport,
                    text_areas,
                    &mut self.swash_cache,
                )
                .unwrap();

            self.text_renderer
                .render(&self.text_atlas, &self.text_viewport, &mut pass)
                .unwrap();

            /* --- OLD MSDF RENDERING (Commented out) ---
            if !self.text_instances.is_empty() {
                // Загружаем инстансы на GPU
                self.queue.write_buffer(&self.text_instance_buffer, 0, bytemuck::cast_slice(&self.text_instances));

                // Настраиваем камеру (ортографическая проекция 2D)
                let proj = [
                    2.0 / self.config.width as f32, 0.0, 0.0, 0.0,
                    0.0, -2.0 / self.config.height as f32, 0.0, 0.0,
                    0.0, 0.0, 1.0, 0.0,
                    -1.0, 1.0, 0.0, 1.0,
                ];
                self.queue.write_buffer(&self.text_camera_buffer, 0, bytemuck::cast_slice(&proj));

                // Настраиваем параметры текста (по умолчанию)
                let params = TextParams {
                    transform: [
                        1.0, 0.0, 0.0, 0.0,
                        0.0, 1.0, 0.0, 0.0,
                        0.0, 0.0, 1.0, 0.0,
                        0.0, 0.0, 0.0, 1.0,
                    ],
                    color: [1.0, 1.0, 1.0, 1.0],
                    scale: 1.0,
                    _padding: [0.0; 3],
                };
                self.queue.write_buffer(&self.text_params_buffer, 0, bytemuck::cast_slice(&[params]));

                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.text_bind_group_0, &[]);
                pass.set_bind_group(1, &self.text_bind_group_1, &[]);
                pass.draw(0..4, 0..self.text_instances.len() as u32);
            }
            */
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}
