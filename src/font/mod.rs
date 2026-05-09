use fontdue::Font;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct AtlasGlyph {
    pub rect: [u32; 4],
    pub offset: [f32; 2],
    pub advance: f32,
}

pub struct FontAtlas {
    font: Font,
    atlas: Vec<u8>,
    pub width: u32,
    pub height: u32,
    cursor_x: u32,
    cursor_y: u32,
    line_height: u32,
    glyph_size: u32,
    fast_cache: [Option<AtlasGlyph>; 256],
    fallback_cache: HashMap<u32, AtlasGlyph>,
}

impl FontAtlas {
    pub fn new(font_data: &[u8], size: u32, atlas_size: u32) -> Self {
        let font = Font::from_bytes(font_data, Default::default()).unwrap();
        let area = atlas_size * atlas_size * 4;
        FontAtlas {
            font,
            atlas: vec![0u8; area as usize],
            width: atlas_size,
            height: atlas_size,
            cursor_x: 0,
            cursor_y: 0,
            line_height: size,
            glyph_size: size,
            fast_cache: [None; 256],
            fallback_cache: HashMap::new(),
        }
    }

    pub fn get_glyph(&mut self, codepoint: u32) -> AtlasGlyph {
        if codepoint < 256 {
            if let Some(g) = self.fast_cache[codepoint as usize] {
                return g;
            }
        } else if let Some(g) = self.fallback_cache.get(&codepoint) {
            return *g;
        }

        let ch = char::from_u32(codepoint).unwrap_or('?');
        let (metrics, bitmap) = self.font.rasterize(ch, self.glyph_size as f32);
        if metrics.width == 0 || bitmap.is_empty() {
            let glyph = AtlasGlyph {
                rect: [0, 0, 0, 0],
                offset: [0.0, 0.0],
                advance: metrics.advance_width,
            };
            if codepoint < 256 {
                self.fast_cache[codepoint as usize] = Some(glyph);
            } else {
                self.fallback_cache.insert(codepoint, glyph);
            }
            return glyph;
        }
        
        let gw = metrics.width as u32 + 2;
        let gh = metrics.height as u32 + 2;

        if self.cursor_x + gw > self.width {
            self.cursor_x = 0;
            self.cursor_y += self.line_height;
        }

        let (sx, sy) = (self.cursor_x, self.cursor_y);
        self.cursor_x += gw;

        for (i, &pixel) in bitmap.iter().enumerate() {
            let px = (i % metrics.width as usize) as u32;
            let py = (i / metrics.width as usize) as u32;
            let atlas_x = sx + px + 1;
            let atlas_y = sy + py + 1;
            if atlas_x < self.width && atlas_y < self.height {
                let idx = ((atlas_y * self.width + atlas_x) * 4) as usize;
                self.atlas[idx] = pixel;
                self.atlas[idx + 1] = pixel;
                self.atlas[idx + 2] = pixel;
                self.atlas[idx + 3] = pixel;
            }
        }

        let ox = metrics.xmin as f32 - 1.0;
        let oy = (self.line_height as f32) - (metrics.ymin as f32) - (metrics.height as f32) - 1.0;

        let glyph = AtlasGlyph {
            rect: [sx, sy, gw, gh],
            offset: [ox, oy],
            advance: metrics.advance_width,
        };
        if codepoint < 256 {
            self.fast_cache[codepoint as usize] = Some(glyph);
        } else {
            self.fallback_cache.insert(codepoint, glyph);
        }
        glyph
    }

    pub fn create_texture(&self, device: &wgpu::Device) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Font Atlas Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    pub fn upload_texture(&self, texture: &wgpu::Texture, device: &wgpu::Device, queue: &wgpu::Queue) {
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Font Staging Buffer"),
            size: (self.width * self.height * 4) as u64,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&staging, 0, &self.atlas);
        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Font Upload Encoder"),
            });
            encoder.copy_buffer_to_texture(
                wgpu::ImageCopyBuffer {
                    buffer: &staging,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(self.width * 4),
                        rows_per_image: Some(self.height),
                    },
                },
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(std::iter::once(encoder.finish()));
        }
    }
}
