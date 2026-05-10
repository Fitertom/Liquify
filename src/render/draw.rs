use crate::ecs::components::*;
use crate::ecs::world::World;
use crate::render::geometry::{Vertex, build_glass_quad, build_shadow_quad, build_background_quad};
use crate::render::pipeline::RenderPipeline;
use crate::input::InputState;

impl RenderPipeline {
    pub fn render_icon(
        &self,
        icon_id: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [f32; 4],
        vertices: &mut Vec<Vertex>,
    ) {
        let index = match icon_id {
            "h" => 0,
            "s" => 1,
            "l" => 2,
            "sett" => 3,
            "heart" => 4,
            "play" => 5,
            "pause" => 6,
            "prev" => 7,
            "next" => 8,
            "music" => 9,
            "image" => 10,
            "plus" => 11,
            "check" => 12,
            "radio" => 13,
            "arrow-left" => 14,
            "more" => 15,
            _ => 0,
        };

        let col = index % 4;
        let row = index / 4;
        let u0 = col as f32 * 0.25;
        let u1 = (col + 1) as f32 * 0.25;
        let v0 = row as f32 * 0.25;
        let v1 = (row + 1) as f32 * 0.25;

        let icon_color = color;

        vertices.extend([
            Vertex { position: [x, y], tex_coord: [u0, v0], screen_uv: [-1.0, -1.0], color: icon_color, quad_size: [size, size, 0.0] },
            Vertex { position: [x + size, y], tex_coord: [u1, v0], screen_uv: [-1.0, -1.0], color: icon_color, quad_size: [size, size, 0.0] },
            Vertex { position: [x + size, y + size], tex_coord: [u1, v1], screen_uv: [-1.0, -1.0], color: icon_color, quad_size: [size, size, 0.0] },
            Vertex { position: [x, y], tex_coord: [u0, v0], screen_uv: [-1.0, -1.0], color: icon_color, quad_size: [size, size, 0.0] },
            Vertex { position: [x + size, y + size], tex_coord: [u1, v1], screen_uv: [-1.0, -1.0], color: icon_color, quad_size: [size, size, 0.0] },
            Vertex { position: [x, y + size], tex_coord: [u0, v1], screen_uv: [-1.0, -1.0], color: icon_color, quad_size: [size, size, 0.0] },
        ]);
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
            let atlas_w = self.font_atlas.width as f32;
            let atlas_h = self.font_atlas.height as f32;
            let u0 = glyph.rect[0] as f32 / atlas_w;
            let v0 = glyph.rect[1] as f32 / atlas_h;
            let u1 = (glyph.rect[0] + glyph.rect[2]) as f32 / atlas_w;
            let v1 = (glyph.rect[1] + glyph.rect[3]) as f32 / atlas_h;
            let px = x + glyph.offset[0] * actual_scale;
            let py = y + glyph.offset[1] * actual_scale;
            let gw_f = glyph.rect[2] as f32 * actual_scale;
            let gh_f = glyph.rect[3] as f32 * actual_scale;

            vertices.extend([
                Vertex { position: [px, py], tex_coord: [u0, v0], screen_uv: [0.0, 0.0], color, quad_size: [gw_f, gh_f, 0.0] },
                Vertex { position: [px + gw_f, py], tex_coord: [u1, v0], screen_uv: [0.0, 0.0], color, quad_size: [gw_f, gh_f, 0.0] },
                Vertex { position: [px + gw_f, py + gh_f], tex_coord: [u1, v1], screen_uv: [0.0, 0.0], color, quad_size: [gw_f, gh_f, 0.0] },
                Vertex { position: [px, py], tex_coord: [u0, v0], screen_uv: [0.0, 0.0], color, quad_size: [gw_f, gh_f, 0.0] },
                Vertex { position: [px + gw_f, py + gh_f], tex_coord: [u1, v1], screen_uv: [0.0, 0.0], color, quad_size: [gw_f, gh_f, 0.0] },
                Vertex { position: [px, py + gh_f], tex_coord: [u0, v1], screen_uv: [0.0, 0.0], color, quad_size: [gw_f, gh_f, 0.0] },
            ]);

            x += glyph.advance * actual_scale;
        }
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

    pub fn render_cover(
        &self,
        x: f32,
        y: f32,
        size: f32,
        color: [f32; 4],
        vertices: &mut Vec<Vertex>,
    ) {
        let u0 = 0.0;
        let u1 = 1.0;
        let v0 = 0.0;
        let v1 = 1.0;

        vertices.extend([
            Vertex { position: [x, y], tex_coord: [u0, v0], screen_uv: [-2.0, -1.0], color, quad_size: [size, size, 0.0] },
            Vertex { position: [x + size, y], tex_coord: [u1, v0], screen_uv: [-2.0, -1.0], color, quad_size: [size, size, 0.0] },
            Vertex { position: [x + size, y + size], tex_coord: [u1, v1], screen_uv: [-2.0, -1.0], color, quad_size: [size, size, 0.0] },
            Vertex { position: [x, y], tex_coord: [u0, v0], screen_uv: [-2.0, -1.0], color, quad_size: [size, size, 0.0] },
            Vertex { position: [x + size, y + size], tex_coord: [u1, v1], screen_uv: [-2.0, -1.0], color, quad_size: [size, size, 0.0] },
            Vertex { position: [x, y + size], tex_coord: [u0, v1], screen_uv: [-2.0, -1.0], color, quad_size: [size, size, 0.0] },
        ]);
    }

    pub fn render_card(
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
        verts.extend(build_glass_quad(x, y, w, h, win_w, win_h, tint, strength, w.min(h) * 0.2));

        // Render title
        let text_color = [1.0, 1.0, 1.0, 1.0];
        let mut text_y = y + (h - 24.0 * scale_factor) * 0.5; // Default center
        let text_x_offset = 65.0 * scale_factor;

        if let Some(sub) = subtitle {
            text_y = y + 22.0 * scale_factor;
            self.render_text_scaled(
                title,
                x + text_x_offset,
                text_y + 15.0 * scale_factor, // Baseline for 15px title
                0.47,
                text_color,
                verts,
                scale_factor,
            );
            self.render_text_scaled(
                sub,
                x + text_x_offset,
                text_y + 35.0 * scale_factor, // Baseline for 12px sub
                0.375,
                [1.0, 1.0, 1.0, 0.5],
                verts,
                scale_factor,
            );
        } else {
            // For small cards (quick grid)
            let tx = x + text_x_offset; 
            self.render_text_scaled(
                title,
                tx,
                y + 34.0 * scale_factor, // Centered baseline for 13px font in 56px card
                0.41,
                text_color,
                verts,
                scale_factor,
            );
        }

        // Render icon placeholder / Cover
        let icon_box_size = h;
        
        // If it's a content card, show the photo
        let is_content = _icon == "image" || _icon == "music" || _icon == "radio" || _icon == "heart";
        
        if is_content {
            self.render_cover(x, y, icon_box_size, [1.0, 1.0, 1.0, 1.0], verts);
        } else {
            verts.extend(build_glass_quad(
                x,
                y,
                icon_box_size,
                icon_box_size,
                win_w,
                win_h,
                tint,
                strength * 1.5,
                icon_box_size * 0.2,
            ));
        }

        // Render the actual icon overlay
        let icon_draw_size = icon_box_size * 0.5;
        let icon_offset = (icon_box_size - icon_draw_size) / 2.0;
        self.render_icon(
            _icon,
            x + icon_offset,
            y + icon_offset,
            icon_draw_size,
            if is_content { [1.0, 1.0, 1.0, 0.5] } else { [1.0, 1.0, 1.0, 0.8] },
            verts,
        );
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

        // 1. Determine active tab and handle interaction
        let mut active_tab = "h".to_string();
        let mut tab_changed = None;

        for eid in world.query_with_mut::<UINavBar>() {
            if let Some(nav) = world.get_component::<UINavBar>(eid) {
                active_tab = nav.active_tab.clone();
            }
        }

        if input.mouse_pressed {
            for eid in world.query_with_mut::<UINavButton>() {
                if let (Some(btn), Some(pos), Some(size)) = (
                    world.get_component::<UINavButton>(eid),
                    world.get_component::<Position>(eid),
                    world.get_component::<Size>(eid),
                ) {
                    if input.mouse_pos.0 >= pos.x && input.mouse_pos.0 <= pos.x + size.width &&
                       input.mouse_pos.1 >= pos.y && input.mouse_pos.1 <= pos.y + size.height {
                        tab_changed = Some(btn.id.clone());
                        break;
                    }
                }
            }
        }

        if let Some(new_tab) = tab_changed {
            active_tab = new_tab.clone();
            for eid in world.query_with_mut::<UINavBar>() {
                if let Some(nav) = world.get_component_mut::<UINavBar>(eid) {
                    nav.active_tab = new_tab.clone();
                }
            }
            for eid in world.query_with_mut::<UINavButton>() {
                if let Some(btn) = world.get_component_mut::<UINavButton>(eid) {
                    btn.is_active = btn.id == new_tab;
                }
            }
        }

        let mut rendered_sections = std::collections::HashSet::new();
        let entities: Vec<u32> = world.entities.all_entities().iter().map(|e| e.id).collect();
        for eid in entities {
            // Filter by page
            if let Some(page) = world.get_component::<Page>(eid) {
                if page.0 != active_tab {
                    continue;
                }
            }

            // Render Header
            if let Some(header) = world.get_component::<UIHeader>(eid) {
                if let Some(pos) = world.get_component::<Position>(eid) {
                    let sy = pos.y - scroll_y;
                    self.render_text_scaled(
                        &header.title,
                        18.0 * scale_factor,
                        sy + 40.0 * scale_factor,
                        0.8,
                        [1.0, 1.0, 1.0, 1.0],
                        &mut verts,
                        scale_factor,
                    );
                    if !header.greeting.is_empty() {
                        self.render_text_scaled(
                            &header.greeting,
                            18.0 * scale_factor,
                            sy + 60.0 * scale_factor,
                            0.87,
                            [1.0, 1.0, 1.0, 1.0],
                            &mut verts,
                            scale_factor,
                        );
                    }
                }
            }

            // Render Sections (Titles)
            if let Some(section) = world.get_component::<UISection>(eid) {
                if section.title == "Recommended" && !rendered_sections.contains(&section.title) {
                    if let Some(pos) = world.get_component::<Position>(eid) {
                        let sy = pos.y - scroll_y - 25.0 * scale_factor;
                        if sy > -50.0 && sy < win_h + 50.0 {
                            self.render_text_scaled(
                                "Рекомендуемые",
                                18.0 * scale_factor,
                                sy,
                                0.68,
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
                                18.0 * scale_factor,
                                sy,
                                0.375,
                                [1.0, 1.0, 1.0, 0.45],
                                &mut verts,
                                scale_factor,
                            );
                        }
                        rendered_sections.insert(section.title.clone());
                    }
                }
                if section.title == "ForYou" && !rendered_sections.contains(&section.title) {
                    if let Some(pos) = world.get_component::<Position>(eid) {
                        let sy = pos.y - scroll_y - 25.0 * scale_factor;
                        if sy > -50.0 && sy < win_h + 50.0 {
                            self.render_text_scaled(
                                "Для вас",
                                18.0 * scale_factor,
                                sy,
                                0.68,
                                [1.0, 1.0, 1.0, 1.0],
                                &mut verts,
                                scale_factor,
                            );
                        }
                        rendered_sections.insert(section.title.clone());
                    }
                }

                // Page Headers
                if section.title == "SearchHeader" {
                    if let Some(pos) = world.get_component::<Position>(eid) {
                        self.render_text_scaled(
                            "Поиск",
                            18.0 * scale_factor,
                            pos.y + 40.0 * scale_factor,
                            0.87,
                            [1.0, 1.0, 1.0, 1.0],
                            &mut verts,
                            scale_factor,
                        );
                        // Search box placeholder
                        verts.extend(build_glass_quad(
                            18.0 * scale_factor,
                            pos.y + 60.0 * scale_factor,
                            win_w - 36.0 * scale_factor,
                            45.0 * scale_factor,
                            win_w,
                            win_h,
                            [0.2, 0.2, 0.2],
                            1.0,
                            12.0 * scale_factor,
                        ));
                        self.render_text_scaled(
                            "Трек, альбом, артист...",
                            30.0 * scale_factor,
                            pos.y + 90.0 * scale_factor,
                            0.4,
                            [1.0, 1.0, 1.0, 0.4],
                            &mut verts,
                            scale_factor,
                        );
                    }
                }
                if section.title == "SettingsHeader" {
                    if let Some(pos) = world.get_component::<Position>(eid) {
                        self.render_text_scaled(
                            "Настройки",
                            18.0 * scale_factor,
                            pos.y + 40.0 * scale_factor,
                            0.87,
                            [1.0, 1.0, 1.0, 1.0],
                            &mut verts,
                            scale_factor,
                        );
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

        // Bottom Nav Bar (Global)
        for eid in world.query_with_mut::<UINavBar>() {
            if let (Some(pos), Some(size)) = (
                world.get_component::<Position>(eid),
                world.get_component::<Size>(eid),
            ) {
                verts.extend(build_glass_quad(
                    pos.x,
                    pos.y,
                    size.width,
                    size.height,
                    win_w,
                    win_h,
                    [0.15, 0.15, 0.2],
                    1.2,
                    size.height * 0.5,
                ));
            }
        }

        // Nav Buttons
        for eid in world.query_with_mut::<UINavButton>() {
            if let (Some(btn), Some(pos), Some(size)) = (
                world.get_component::<UINavButton>(eid),
                world.get_component::<Position>(eid),
                world.get_component::<Size>(eid),
            ) {
                let color = if btn.is_active { [1.0, 1.0, 1.0, 1.0] } else { [1.0, 1.0, 1.0, 0.55] };
                
                self.render_icon(
                    &btn.id,
                    pos.x + (size.width - 22.0 * scale_factor) * 0.5,
                    pos.y + 9.0 * scale_factor,
                    22.0 * scale_factor,
                    color,
                    &mut verts,
                );

                let text_w = self.measure_text(&btn.label) * 0.25 * scale_factor;
                self.render_text_scaled(
                    &btn.label,
                    pos.x + (size.width - text_w) * 0.5,
                    pos.y + 42.0 * scale_factor,
                    0.25,
                    color,
                    &mut verts,
                    scale_factor,
                );
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

        if self.font_atlas.dirty {
            self.font_atlas.upload_texture(&self.font_texture, &self.device, &self.queue);
            self.font_atlas.clear_dirty();
        }
    }
}
