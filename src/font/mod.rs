use fontdue::Font;
use std::collections::HashMap;

/// GPU-совместимая структура для хранения метрик символа.
/// Должна точно соответствовать struct Char в msdfText.wgsl.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GPUChar {
    pub tex_offset: [f32; 2],
    pub tex_extent: [f32; 2],
    pub size: [f32; 2],
    pub offset: [f32; 2],
}

pub struct AtlasGlyph {
    pub gpu_index: u32,
    pub advance: f32,
}

pub struct FontAtlas {
    pub font: Font,
    pub atlas_rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub glyphs: Vec<GPUChar>,
    pub char_to_glyph: HashMap<u32, AtlasGlyph>,
    pub line_height: f32,
}

impl FontAtlas {
    pub fn load(font_data: &[u8], atlas_png: &[u8], atlas_json: &str) -> Self {
        let font = Font::from_bytes(font_data, Default::default()).unwrap();
        
        // Загружаем изображение атласа
        let img = image::load_from_memory(atlas_png).unwrap().to_rgba8();
        let (width, height) = img.dimensions();
        let atlas_rgba = img.into_raw();

        // Парсим JSON (очень упрощенно, так как msdf-atlas-gen выдает стабильный формат)
        let mut char_to_glyph = HashMap::new();
        let mut gpu_chars = Vec::new();

        if let Some(glyphs_start) = atlas_json.find("\"glyphs\":[") {
            let glyphs_content = &atlas_json[glyphs_start + 9..];
            let mut current = glyphs_content;
            
            let mut gpu_idx = 0;
            while let Some(obj_start) = current.find('{') {
                if let Some(obj_end) = current.find('}') {
                    if obj_end < obj_start {
                        // Если нашли закрывающую скобку до открывающей, пропускаем её
                        current = &current[obj_end + 1..];
                        continue;
                    }
                    let obj = &current[obj_start..obj_end + 1];
                    
                    let index = find_json_u32(obj, "index").unwrap_or(0);
                    let advance = find_json_f32(obj, "advance").unwrap_or(0.0);
                    
                    let l = find_json_f32_nested(obj, "atlasBounds", "left").unwrap_or(0.0);
                    let b = find_json_f32_nested(obj, "atlasBounds", "bottom").unwrap_or(0.0);
                    let r = find_json_f32_nested(obj, "atlasBounds", "right").unwrap_or(0.0);
                    let t = find_json_f32_nested(obj, "atlasBounds", "top").unwrap_or(0.0);

                    let pl = find_json_f32_nested(obj, "planeBounds", "left").unwrap_or(0.0);
                    let pb = find_json_f32_nested(obj, "planeBounds", "bottom").unwrap_or(0.0);
                    let pr = find_json_f32_nested(obj, "planeBounds", "right").unwrap_or(0.0);
                    let pt = find_json_f32_nested(obj, "planeBounds", "top").unwrap_or(0.0);

                    gpu_chars.push(GPUChar {
                        tex_offset: [l / width as f32, 1.0 - t / height as f32],
                        tex_extent: [(r - l) / width as f32, (t - b) / height as f32],
                        size: [pr - pl, pt - pb],
                        offset: [pl, pb],
                    });
                    
                    let last = gpu_chars.last_mut().unwrap();
                    last.tex_offset[1] = (height as f32 - t) / height as f32;

                    char_to_glyph.insert(index, AtlasGlyph {
                        gpu_index: gpu_idx,
                        advance,
                    });

                    gpu_idx += 1;
                    current = &current[obj_end + 1..];
                } else {
                    break;
                }
                if current.trim_start().starts_with(']') { break; }
            }
        }

        FontAtlas {
            font,
            atlas_rgba,
            width,
            height,
            glyphs: gpu_chars,
            char_to_glyph,
            line_height: 1.2,
        }
    }

    pub fn get_glyph(&self, codepoint: u32) -> Option<&AtlasGlyph> {
        let glyph_index = self.font.lookup_glyph_index(char::from_u32(codepoint).unwrap_or(' '));
        self.char_to_glyph.get(&(glyph_index as u32))
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
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    pub fn upload_texture(&self, texture: &wgpu::Texture, device: &wgpu::Device, queue: &wgpu::Queue) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.atlas_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.width * 4),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }
}

fn find_json_u32(json: &str, key: &str) -> Option<u32> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let start = pos + pattern.len();
        let end = json[start..].find(|c: char| !c.is_ascii_digit()).unwrap_or(json.len() - start);
        return json[start..start + end].trim().parse().ok();
    }
    None
}

fn find_json_f32(json: &str, key: &str) -> Option<f32> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let start = pos + pattern.len();
        let end = json[start..].find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').unwrap_or(json.len() - start);
        return json[start..start + end].trim().parse().ok();
    }
    None
}

fn find_json_f32_nested(json: &str, parent: &str, key: &str) -> Option<f32> {
    let pattern = format!("\"{}\":{{", parent);
    if let Some(pos) = json.find(&pattern) {
        let content = &json[pos + pattern.len()..];
        let end = content.find('}').unwrap_or(content.len());
        return find_json_f32(&content[..end], key);
    }
    None
}
