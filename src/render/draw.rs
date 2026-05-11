use crate::ecs::components::*;
use crate::ecs::world::World;
use crate::input::InputState;
use crate::render::geometry::{Vertex, build_background_quad, build_glass_quad, build_shadow_quad};
use crate::render::pipeline::RenderPipeline;

impl RenderPipeline {
    pub fn render_icon(
        &self,
        icon_id: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [f32; 4],
        vector_verts: &mut Vec<Vertex>,
    ) {
        let id_f = match icon_id {
            "h" => 0.0, "s" => 1.0, "l" => 2.0, "sett" => 3.0, "heart" => 4.0, "play" => 5.0, "pause" => 6.0,
            "prev" => 7.0, "next" => 8.0, "music" => 9.0, "image" => 10.0, "plus" => 11.0, "check" => 12.0,
            "radio" => 13.0, "arrow-left" => 14.0, "more" => 15.0, _ => 0.0,
        };
        let s = [size, size];
        let r = [0.0; 4];
        vector_verts.extend([
            Vertex { position: [x, y], tex_coord: [0.0, 0.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
            Vertex { position: [x + size, y], tex_coord: [1.0, 0.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
            Vertex { position: [x + size, y + size], tex_coord: [1.0, 1.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
            Vertex { position: [x, y], tex_coord: [0.0, 0.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
            Vertex { position: [x + size, y + size], tex_coord: [1.0, 1.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
            Vertex { position: [x, y + size], tex_coord: [0.0, 1.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
        ]);
    }

    pub fn render_text_scaled(&mut self, text: &str, x: f32, y: f32, scale: f32, color: [f32; 4], _scale_factor: f32) {
        self.text_requests.push((text.to_string(), x, y, scale, color));
    }

    pub fn measure_text(&mut self, text: &str) -> f32 {
        let mut width = 0.0;
        let space_scale = 382.0 / 488.0;
        for ch in text.chars() { if ch == ' ' { width += 0.58 * space_scale; } else { width += 0.58; } }
        width
    }

    pub fn render_cover(&self, x: f32, y: f32, size: f32, color: [f32; 4], radii: [f32; 4], vertices: &mut Vec<Vertex>) {
        let (u0, u1, v0, v1) = (0.0, 1.0, 0.0, 1.0);
        let s = [size, size];
        vertices.extend([
            Vertex { position: [x, y], tex_coord: [u0, v0], screen_uv: [-2.0, -1.0], color, size: s, radii },
            Vertex { position: [x + size, y], tex_coord: [u1, v0], screen_uv: [-2.0, -1.0], color, size: s, radii },
            Vertex { position: [x + size, y + size], tex_coord: [u1, v1], screen_uv: [-2.0, -1.0], color, size: s, radii },
            Vertex { position: [x, y], tex_coord: [u0, v0], screen_uv: [-2.0, -1.0], color, size: s, radii },
            Vertex { position: [x + size, y + size], tex_coord: [u1, v1], screen_uv: [-2.0, -1.0], color, size: s, radii },
            Vertex { position: [x, y + size], tex_coord: [0.0, v1], screen_uv: [-2.0, -1.0], color, size: s, radii },
        ]);
    }

    pub fn render_horizontal_card(&mut self, verts: &mut Vec<Vertex>, x: f32, y: f32, w: f32, h: f32, win_w: f32, win_h: f32, tint: [f32; 3], strength: f32, title: &str, subtitle: Option<&str>, scale_factor: f32) {
        let r = 12.0 * scale_factor;
        verts.extend(build_shadow_quad(x, y, w, h, 6.0 * scale_factor, 0.15));
        verts.extend(build_glass_quad(x, y, w, h, win_w, win_h, tint, strength, [r; 4]));
        self.render_cover(x, y, h, [1.0, 1.0, 1.0, 1.0], [r, 0.0, r, 0.0], verts);
        
        let text_x = x + h + 12.0 * scale_factor;
        if let Some(sub) = subtitle {
            let block_h = 32.0 * scale_factor;
            let ty = y + (h - block_h) * 0.5;
            self.render_text_scaled(title, text_x, ty, 13.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
            self.render_text_scaled(sub, text_x, ty + 18.0 * scale_factor, 10.0, [1.0, 1.0, 1.0, 0.5], scale_factor);
        } else {
            self.render_text_scaled(title, text_x, y + (h - 13.0 * scale_factor) * 0.5, 13.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
        }
    }

    pub fn render_vertical_card(&mut self, verts: &mut Vec<Vertex>, x: f32, y: f32, w: f32, h: f32, win_w: f32, win_h: f32, tint: [f32; 3], strength: f32, title: &str, subtitle: Option<&str>, scale_factor: f32) {
        let r = 12.0 * scale_factor;
        verts.extend(build_shadow_quad(x, y, w, h, 8.0 * scale_factor, 0.2));
        verts.extend(build_glass_quad(x, y, w, h, win_w, win_h, tint, strength, [r; 4]));
        self.render_cover(x, y, w, [1.0, 1.0, 1.0, 1.0], [r, r, 0.0, 0.0], verts);
        
        let text_y_base = y + w + 12.0 * scale_factor;
        self.render_text_scaled(title, x + 12.0 * scale_factor, text_y_base, 14.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
        if let Some(sub) = subtitle {
            self.render_text_scaled(sub, x + 12.0 * scale_factor, text_y_base + 20.0 * scale_factor, 11.0, [1.0, 1.0, 1.0, 0.5], scale_factor);
        }
    }

    pub fn build_frame_ecs(&mut self, world: &mut World, input: &InputState, fps_text: &str, win_w: f32, win_h: f32, scale_factor: f32) {
        let mut verts: Vec<Vertex> = Vec::new();
        let mut vector_verts: Vec<Vertex> = Vec::new();
        self.text_requests.clear();
        let scroll_y = input.scroll.render_offset;
        let entities = world.entities.all_entities().iter().map(|e| e.id).collect::<Vec<u32>>();

        verts.extend(build_background_quad(win_w, win_h));

        let mut active_tab = "h".to_string();
        for eid in entities.iter() {
            if let Some(nav) = world.get_component::<UINavBar>(*eid) { active_tab = nav.active_tab.clone(); break; }
        }

        let mut section_divider_y = 10000.0;
        for eid in entities.iter() {
            if let Some(section) = world.get_component::<UISection>(*eid) {
                if let Some(pos) = world.get_component::<Position>(*eid) {
                    if section.title == "Recommended" { section_divider_y = pos.y; break; }
                }
            }
        }

        for eid in entities.iter() {
            if let Some(page) = world.get_component::<Page>(*eid) { if page.0 != active_tab { continue; } }
            if let Some(header) = world.get_component::<UIHeader>(*eid) {
                if let Some(pos) = world.get_component::<Position>(*eid) {
                    let sy = pos.y - scroll_y;
                    self.render_text_scaled(&header.title, 18.0 * scale_factor, sy + 40.0 * scale_factor, 29.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
                    if !header.greeting.is_empty() { self.render_text_scaled(&header.greeting, 18.0 * scale_factor, sy + 74.0 * scale_factor, 27.0, [1.0, 1.0, 1.0, 1.0], scale_factor); }
                }
            }
        }

        for eid in entities.iter() {
            if let Some(page) = world.get_component::<Page>(*eid) { if page.0 != active_tab { continue; } }
            if let Some(card) = world.get_component::<UICard>(*eid) {
                if let (Some(pos), Some(size)) = (world.get_component::<Position>(*eid), world.get_component::<Size>(*eid)) {
                    if pos.y < section_divider_y {
                        let sy = pos.y - scroll_y;
                        if sy > -size.height && sy < win_h {
                            let strength = if card.is_hovered { 1.1 } else { 0.88 };
                            self.render_horizontal_card(&mut verts, pos.x, sy, size.width, size.height, win_w, win_h, card.tint, strength, &card.title, card.subtitle.as_deref(), scale_factor);
                        }
                    }
                }
            }
        }

        for eid in entities.iter() {
            if let Some(page) = world.get_component::<Page>(*eid) { if page.0 != active_tab { continue; } }
            if let Some(section) = world.get_component::<UISection>(*eid) {
                if let Some(pos) = world.get_component::<Position>(*eid) {
                    let sy = pos.y - scroll_y;
                    match section.title.as_str() {
                        "Recommended" => self.render_text_scaled("Рекомендуемые", 18.0 * scale_factor, sy - 25.0 * scale_factor, 21.0, [1.0, 1.0, 1.0, 1.0], scale_factor),
                        "NewRelease" => self.render_text_scaled("Новый релиз исполнителя", 18.0 * scale_factor, sy - 25.0 * scale_factor, 11.0, [1.0, 1.0, 1.0, 0.45], scale_factor),
                        "ForYou" => self.render_text_scaled("Для вас", 18.0 * scale_factor, sy - 25.0 * scale_factor, 21.0, [1.0, 1.0, 1.0, 1.0], scale_factor),
                        "SearchHeader" => {
                            self.render_text_scaled("Поиск", 18.0 * scale_factor, sy + 40.0 * scale_factor, 28.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
                            verts.extend(build_glass_quad(12.0 * scale_factor, sy + 70.0 * scale_factor, win_w - 24.0 * scale_factor, 45.0 * scale_factor, win_w, win_h, [0.2, 0.2, 0.2], 1.0, [12.0 * scale_factor; 4]));
                            self.render_text_scaled("Трек, альбом, артист...", 18.0 * scale_factor, sy + 98.0 * scale_factor, 12.5, [1.0, 1.0, 1.0, 0.4], scale_factor);
                        }, _ => {}
                    }
                }
            }
        }

        for eid in entities.iter() {
            if let Some(page) = world.get_component::<Page>(*eid) { if page.0 != active_tab { continue; } }
            if let Some(card) = world.get_component::<UICard>(*eid) {
                if let (Some(pos), Some(size)) = (world.get_component::<Position>(*eid), world.get_component::<Size>(*eid)) {
                    if pos.y >= section_divider_y {
                        let sy = pos.y - scroll_y;
                        if sy > -size.height && sy < win_h {
                            let strength = if card.is_hovered { 1.1 } else { 0.88 };
                            self.render_vertical_card(&mut verts, pos.x, sy, size.width, size.height, win_w, win_h, card.tint, strength, &card.title, card.subtitle.as_deref(), scale_factor);
                        }
                    }
                }
            }
        }

        for eid in world.query_with_mut::<UINavBar>() {
            if let (Some(pos), Some(size)) = (world.get_component::<Position>(eid), world.get_component::<Size>(eid)) {
                let r = size.height * 0.5;
                verts.extend(build_glass_quad(pos.x, pos.y, size.width, size.height, win_w, win_h, [0.15, 0.15, 0.2], 1.2, [r; 4]));
            }
        }
        for eid in world.query_with_mut::<UINavButton>() {
            if let (Some(btn), Some(pos), Some(size)) = (world.get_component::<UINavButton>(eid), world.get_component::<Position>(eid), world.get_component::<Size>(eid)) {
                let color = if btn.is_active { [1.0, 1.0, 1.0, 1.0] } else { [1.0, 1.0, 1.0, 0.55] };
                self.render_icon(&btn.id, pos.x + (size.width - 22.0 * scale_factor) * 0.5, pos.y + 9.0 * scale_factor, 22.0 * scale_factor, color, &mut vector_verts);
                let text_w = self.measure_text(&btn.label) * 11.0 * scale_factor;
                self.render_text_scaled(&btn.label, pos.x + (size.width - text_w) * 0.5, pos.y + 42.0 * scale_factor, 11.0, color, scale_factor);
            }
        }

        self.vertex_count = (verts.len() as u32).min(10000);
        for v in verts.iter_mut().take(self.vertex_count as usize) {
            v.position[0] = (v.position[0] / win_w) * 2.0 - 1.0;
            v.position[1] = 1.0 - (v.position[1] / win_h) * 2.0;
        }
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts[..self.vertex_count as usize]));

        self.vector_vertex_count = (vector_verts.len() as u32).min(10000);
        for v in vector_verts.iter_mut().take(self.vector_vertex_count as usize) {
            v.position[0] = (v.position[0] / win_w) * 2.0 - 1.0;
            v.position[1] = 1.0 - (v.position[1] / win_h) * 2.0;
        }
        self.queue.write_buffer(&self.vector_vertex_buffer, 0, bytemuck::cast_slice(&vector_verts[..self.vector_vertex_count as usize]));
    }
}
