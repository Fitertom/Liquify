use crate::ecs::components::*;
use crate::ecs::world::World;
use crate::font::FontAtlas;
use crate::input::InputState;
use crate::render::geometry::{Vertex, build_background_quad, build_glass_quad, build_shadow_quad};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurParams {
    texel_size: [f32; 2],
    radius: f32,
    direction: f32,
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
    max_vertices: usize,

    pub blur_pipeline: wgpu::RenderPipeline,
    pub blur_params_buffer_h: wgpu::Buffer,
    pub blur_params_buffer_v: wgpu::Buffer,
    pub blurred_texture: wgpu::Texture,
    pub blurred_texture_view: wgpu::TextureView,
    pub temp_texture: wgpu::Texture,
    pub temp_texture_view: wgpu::TextureView,
}

impl RenderPipeline {
    pub async fn new(window: &winit::window::Window, font_data: &[u8]) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: wgpu::InstanceFlags::default(),
            dx12_shader_compiler: Default::default(),
            gles_minor_version: Default::default(),
        });

        let surface = instance.create_surface(window).unwrap();
        let surface: wgpu::Surface<'static> = unsafe { std::mem::transmute(surface) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Vulkan adapter not found. Install/update a GPU driver with Vulkan support.");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
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

        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
            wgpu::PresentMode::Immediate
        } else if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
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
            bind_group_layouts: &[&bg_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
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
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let font_atlas = FontAtlas::new(font_data, 32, 512);
        let font_texture = font_atlas.create_texture(&device);
        font_atlas.upload_texture(&font_texture, &device, &queue);

        let font_view = font_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bg_data = include_bytes!("../../background.png");
        let bg_image = image::load_from_memory(bg_data).expect("Failed to load background.png");
        let bg_rgba = bg_image.to_rgba8();
        let (bg_w, bg_h) = bg_rgba.dimensions();

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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&font_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&bg_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&blurred_view),
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
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
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

            // Recreate main bind group
            let bg_layout =
                self.device
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
                        ],
                        label: Some("Bind Group Layout"),
                    });

            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bg_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.background_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.blurred_texture_view),
                    },
                ],
                label: Some("Bind Group"),
            });
        }
    }

    pub fn render_text_scaled(
        &mut self,
        text: &str,
        mut x: f32,
        y: f32,
        scale: f32,
        color: [f32; 4],
        vertices: &mut Vec<Vertex>,
        scale_factor: f32,
    ) {
        let actual_scale = scale * scale_factor;
        for ch in text.chars() {
            let glyph = self.font_atlas.get_glyph(ch as u32);
            let (gx, gy, gw, gh) = (glyph.rect[0], glyph.rect[1], glyph.rect[2], glyph.rect[3]);
            let atlas_w = self.font_atlas.width as f32;
            let atlas_h = self.font_atlas.height as f32;
            let u0 = gx as f32 / atlas_w;
            let v0 = gy as f32 / atlas_h;
            let u1 = (gx + gw) as f32 / atlas_w;
            let v1 = (gy + gh) as f32 / atlas_h;
            let px = x + glyph.offset[0] * actual_scale;
            let py = y + glyph.offset[1] * actual_scale;
            let gw_f = gw as f32 * actual_scale;
            let gh_f = gh as f32 * actual_scale;

            vertices.extend([
                Vertex {
                    position: [px, py],
                    tex_coord: [u0, v0],
                    screen_uv: [0.0, 0.0],
                    color,
                    quad_size: [gw_f, gh_f],
                },
                Vertex {
                    position: [px + gw_f, py],
                    tex_coord: [u1, v0],
                    screen_uv: [0.0, 0.0],
                    color,
                    quad_size: [gw_f, gh_f],
                },
                Vertex {
                    position: [px + gw_f, py + gh_f],
                    tex_coord: [u1, v1],
                    screen_uv: [0.0, 0.0],
                    color,
                    quad_size: [gw_f, gh_f],
                },
                Vertex {
                    position: [px, py],
                    tex_coord: [u0, v0],
                    screen_uv: [0.0, 0.0],
                    color,
                    quad_size: [gw_f, gh_f],
                },
                Vertex {
                    position: [px + gw_f, py + gh_f],
                    tex_coord: [u1, v1],
                    screen_uv: [0.0, 0.0],
                    color,
                    quad_size: [gw_f, gh_f],
                },
                Vertex {
                    position: [px, py + gh_f],
                    tex_coord: [u0, v1],
                    screen_uv: [0.0, 0.0],
                    color,
                    quad_size: [gw_f, gh_f],
                },
            ]);

            x += glyph.advance * actual_scale;
        }
    }

    pub fn render_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        color: [f32; 4],
        vertices: &mut Vec<Vertex>,
    ) {
        self.render_text_scaled(text, x, y, 1.0, color, vertices, 1.0);
    }

    pub fn measure_text(&mut self, text: &str) -> f32 {
        let mut width = 0.0;
        let scale = 1.0;
        for ch in text.chars() {
            let g = self.font_atlas.get_glyph(ch as u32);
            width += g.advance * scale;
        }
        width
    }

    fn render_card(
        &mut self,
        verts: &mut Vec<Vertex>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        win_w: f32,
        win_h: f32,
        tint: [f32; 3],
        strength: f32,
        title: &str,
        subtitle: Option<&str>,
        _icon: &str,
        scale_factor: f32,
    ) {
        verts.extend(build_shadow_quad(x, y, w, h, 8.0 * scale_factor, 0.18));
        verts.extend(build_glass_quad(x, y, w, h, win_w, win_h, tint, strength));

        // Render title
        let text_color = [1.0, 1.0, 1.0, 1.0];
        let mut text_y = y + (h - 24.0 * scale_factor) * 0.5; // Default center
        let text_x_offset = 65.0 * scale_factor;

        if let Some(sub) = subtitle {
            text_y = y + 25.0 * scale_factor;
            self.render_text_scaled(
                title,
                x + text_x_offset,
                text_y,
                0.45,
                text_color,
                verts,
                scale_factor,
            );
            self.render_text_scaled(
                sub,
                x + text_x_offset,
                text_y + 28.0 * scale_factor,
                0.35,
                [1.0, 1.0, 1.0, 0.5],
                verts,
                scale_factor,
            );
        } else {
            // For small cards (quick grid)
            let tx = x + text_x_offset; // Margin for icon
            self.render_text_scaled(
                title,
                tx,
                text_y + 8.0 * scale_factor,
                0.4,
                text_color,
                verts,
                scale_factor,
            );
        }

        // Render icon placeholder (a simple glass box for now)
        verts.extend(build_glass_quad(
            x,
            y,
            h,
            h,
            win_w,
            win_h,
            tint,
            strength * 1.5,
        ));
    }

    pub fn build_frame_ecs(
        &mut self,
        world: &mut World,
        input: &InputState,
        fps_text: &str,
        win_w: f32,
        win_h: f32,
        scale_factor: f32,
    ) {
        let mut verts: Vec<Vertex> = Vec::new();
        let scroll_y = input.scroll.render_offset;

        verts.extend(build_background_quad(win_w, win_h));

        // Render Header
        for eid in world.query_with_mut::<UIHeader>() {
            if let (Some(header), Some(pos)) = (
                world.get_component::<UIHeader>(eid),
                world.get_component::<Position>(eid),
            ) {
                let sy = pos.y - scroll_y;
                if sy > -100.0 && sy < win_h + 100.0 {
                    self.render_text_scaled(
                        &header.title,
                        pos.x,
                        sy,
                        0.8,
                        [1.0, 1.0, 1.0, 1.0],
                        &mut verts,
                        scale_factor,
                    );
                    self.render_text_scaled(
                        &header.greeting,
                        pos.x,
                        sy + 60.0 * scale_factor,
                        0.7,
                        [1.0, 1.0, 1.0, 1.0],
                        &mut verts,
                        scale_factor,
                    );
                }
            }
        }

        // Render Sections (Titles)
        let mut rendered_sections = std::collections::HashSet::new();
        let entities: Vec<u32> = world.entities.all_entities().iter().map(|e| e.id).collect();
        for eid in entities {
            if let Some(section) = world.get_component::<UISection>(eid) {
                if section.title == "Recommended" && !rendered_sections.contains(&section.title) {
                    if let Some(pos) = world.get_component::<Position>(eid) {
                        let sy = pos.y - scroll_y - 25.0 * scale_factor;
                        if sy > -50.0 && sy < win_h + 50.0 {
                            self.render_text_scaled(
                                "Рекомендуемые",
                                18.0 * scale_factor,
                                sy,
                                0.6,
                                [1.0, 1.0, 1.0, 1.0],
                                &mut verts,
                                scale_factor,
                            );
                        }
                        rendered_sections.insert(section.title.clone());
                    }
                }
                if section.title == "NewRelease" && !rendered_sections.contains(&section.title) {
                    if let Some(pos) = world.get_component::<Position>(eid) {
                        let sy = pos.y - scroll_y - 25.0 * scale_factor;
                        if sy > -50.0 && sy < win_h + 50.0 {
                            self.render_text_scaled(
                                "Новый релиз исполнителя",
                                pos.x,
                                sy,
                                0.35,
                                [1.0, 1.0, 1.0, 0.5],
                                &mut verts,
                                scale_factor,
                            );
                        }
                        rendered_sections.insert(section.title.clone());
                    }
                }
            }

            // Render Cards
            if let Some(card) = world.get_component::<UICard>(eid) {
                if let (Some(pos), Some(size)) = (
                    world.get_component::<Position>(eid),
                    world.get_component::<Size>(eid),
                ) {
                    let sy = pos.y - scroll_y;
                    // Cull off-screen elements
                    if sy > -size.height - 20.0 && sy < win_h + 20.0 && pos.x < win_w {
                        self.render_card(
                            &mut verts,
                            pos.x,
                            sy,
                            size.width,
                            size.height,
                            win_w,
                            win_h,
                            card.tint,
                            if card.is_hovered { 1.1 } else { 0.88 },
                            &card.title,
                            card.subtitle.as_deref(),
                            &card.icon,
                            scale_factor,
                        );
                    }
                }
            }
        }

        // FPS
        self.render_text_scaled(
            fps_text,
            win_w - 70.0 * scale_factor,
            20.0 * scale_factor,
            0.4,
            [0.55, 0.85, 1.0, 0.85],
            &mut verts,
            scale_factor,
        );

        self.vertex_count = verts.len().min(self.max_vertices) as u32;
        let vertex_count = self.vertex_count as usize;

        for vertex in verts.iter_mut().take(vertex_count) {
            vertex.position[0] = (vertex.position[0] / win_w) * 2.0 - 1.0;
            vertex.position[1] = 1.0 - (vertex.position[1] / win_h) * 2.0;
        }

        self.queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&verts[..vertex_count]),
        );
    }

    pub fn draw(&mut self) {
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
            self.queue.write_buffer(&self.blur_params_buffer_h, 0, bytemuck::bytes_of(&blur_params));

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
            self.queue.write_buffer(&self.blur_params_buffer_v, 0, bytemuck::bytes_of(&blur_params));

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
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}
