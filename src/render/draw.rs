use crate::ecs::components::*;
use crate::ecs::world::World;
use crate::input::InputState;
use crate::render::geometry::{Vertex, build_background_quad, build_glass_quad, build_shadow_quad};
use crate::render::pipeline::RenderPipeline;

impl RenderPipeline {
    // Helper to render a rounded rectangle (glass panel)
    fn render_glass_panel(&self, verts: &mut Vec<Vertex>, x: f32, y: f32, w: f32, h: f32, win_w: f32, win_h: f32, tint: [f32; 3], strength: f32, radii: [f32; 4]) {
        verts.extend(build_glass_quad(x, y, w, h, win_w, win_h, tint, strength, radii));
    }

    // Helper to render progress bar (filled horizontal bar)
    fn render_progress_bar(verts: &mut Vec<Vertex>, x: f32, y: f32, w: f32, h: f32, win_w: f32, win_h: f32, progress: f32) {
        let bg_radii = [h * 0.5; 4];
        verts.extend(build_glass_quad(x, y, w, h, win_w, win_h, [0.2, 0.2, 0.2], 0.3, bg_radii));
        if progress > 0.0 {
            let fill_w = w * progress;
            verts.extend(build_glass_quad(x, y, fill_w, h, win_w, win_h, [0.65, 0.55, 0.98], 0.6, bg_radii));
        }
    }

    // Helper to create a rounded cover quad
    fn build_rounded_cover_quad(&self, x: f32, y: f32, size: f32, radii: [f32; 4]) -> [Vertex; 6] {
        let (xmin, ymin) = (x, y);
        let (xmax, ymax) = (x + size, y + size);
        let c = [1.0, 1.0, 1.0, 1.0];
        let s = [size, size];
        let tex_coords = [
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ];
        [
            Vertex { position: [xmin, ymin], tex_coord: tex_coords[0], screen_uv: [-2.0, -1.0], color: c, size: s, radii },
            Vertex { position: [xmax, ymin], tex_coord: tex_coords[1], screen_uv: [-2.0, -1.0], color: c, size: s, radii },
            Vertex { position: [xmax, ymax], tex_coord: tex_coords[2], screen_uv: [-2.0, -1.0], color: c, size: s, radii },
            Vertex { position: [xmin, ymin], tex_coord: tex_coords[3], screen_uv: [-2.0, -1.0], color: c, size: s, radii },
            Vertex { position: [xmax, ymax], tex_coord: tex_coords[4], screen_uv: [-2.0, -1.0], color: c, size: s, radii },
            Vertex { position: [xmin, ymax], tex_coord: tex_coords[5], screen_uv: [-2.0, -1.0], color: c, size: s, radii },
        ]
    }

    // Helper: build icon vertices inline
    fn build_icon_verts(&self, icon_id: &str, x: f32, y: f32, size: f32, color: [f32; 4]) -> Vec<Vertex> {
        let id_f = match icon_id {
            "h" => 0.0, "s" => 1.0, "l" => 2.0, "sett" => 3.0, "heart" => 4.0,
            "play" => 5.0, "pause" => 6.0, "prev" => 7.0, "next" => 8.0,
            "music" => 9.0, "image" => 10.0, "plus" => 11.0, "check" => 12.0,
            "radio" => 13.0, "arrow-left" => 14.0, "more" => 15.0,
            "shuffle" => 16.0, "repeat" => 17.0, _ => 0.0,
        };
        let s = [size, size];
        let r = [0.0; 4];
        vec![
            Vertex { position: [x, y], tex_coord: [0.0, 0.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
            Vertex { position: [x + size, y], tex_coord: [1.0, 0.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
            Vertex { position: [x + size, y + size], tex_coord: [1.0, 1.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
            Vertex { position: [x, y], tex_coord: [0.0, 0.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
            Vertex { position: [x + size, y + size], tex_coord: [1.0, 1.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
            Vertex { position: [x, y + size], tex_coord: [0.0, 1.0], screen_uv: [id_f, 0.0], color, size: s, radii: r },
        ]
    }

    pub fn queue_content_text(&mut self, text: &str, x: f32, y: f32, scale: f32, color: [f32; 4], _scale_factor: f32) {
        self.content_text_requests.push((text.to_string(), x, y, scale, color));
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

    pub fn render_cover(x: f32, y: f32, size: f32, color: [f32; 4], radii: [f32; 4], vertices: &mut Vec<Vertex>) {
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

    pub fn render_horizontal_card(&mut self, x: f32, y: f32, w: f32, h: f32, win_w: f32, win_h: f32, tint: [f32; 3], strength: f32, title: &str, subtitle: Option<&str>, scale_factor: f32) {
        let r = 12.0 * scale_factor;
        self.content_vertices.extend(build_shadow_quad(x, y, w, h, 6.0 * scale_factor, 0.15));
        self.content_vertices.extend(build_glass_quad(x, y, w, h, win_w, win_h, tint, strength, [r; 4]));
        Self::render_cover(x, y, h, [1.0, 1.0, 1.0, 1.0], [r, 0.0, r, 0.0], &mut self.content_vertices);

        let text_x = x + h + 12.0 * scale_factor;
        if let Some(sub) = subtitle {
            let block_h = 32.0 * scale_factor;
            let ty = y + (h - block_h) * 0.5;
            self.queue_content_text(title, text_x, ty, 13.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
            self.queue_content_text(sub, text_x, ty + 18.0 * scale_factor, 10.0, [1.0, 1.0, 1.0, 0.5], scale_factor);
        } else {
            self.queue_content_text(title, text_x, y + (h - 13.0 * scale_factor) * 0.5, 13.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
        }
    }

    pub fn render_vertical_card(&mut self, x: f32, y: f32, w: f32, h: f32, win_w: f32, win_h: f32, tint: [f32; 3], strength: f32, title: &str, subtitle: Option<&str>, scale_factor: f32) {
        let r = 12.0 * scale_factor;
        self.content_vertices.extend(build_shadow_quad(x, y, w, h, 8.0 * scale_factor, 0.2));
        self.content_vertices.extend(build_glass_quad(x, y, w, h, win_w, win_h, tint, strength, [r; 4]));
        Self::render_cover(x, y, w, [1.0, 1.0, 1.0, 1.0], [r, r, 0.0, 0.0], &mut self.content_vertices);

        let text_y_base = y + w + 12.0 * scale_factor;
        self.queue_content_text(title, x + 12.0 * scale_factor, text_y_base, 14.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
        if let Some(sub) = subtitle {
            self.queue_content_text(sub, x + 12.0 * scale_factor, text_y_base + 20.0 * scale_factor, 11.0, [1.0, 1.0, 1.0, 0.5], scale_factor);
        }
    }

    pub fn build_frame_ecs(&mut self, world: &mut World, input: &InputState, fps_text: &str, win_w: f32, win_h: f32, scale_factor: f32) {
        // Clear both text queues and vertex buffers
        self.content_text_requests.clear();
        self.text_requests.clear();
        self.gradient_text_requests.clear();
        self.content_vertices.clear();
        self.ui_vertices.clear();
        self.scene_blur_ui_vertices.clear();
        self.content_vector_vertices.clear();
        self.ui_vector_vertices.clear();

        let scroll_y = input.scroll.render_offset;
        let entities = world.entities.all_entities().iter().map(|e| e.id).collect::<Vec<u32>>();
        let s = scale_factor;

        // Content layer: background
        self.content_vertices.extend(build_background_quad(win_w, win_h));

        // Determine active tab
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

        // === CONTENT LAYER ===
        // Headers
        for eid in entities.iter() {
            if let Some(page_comp) = world.get_component::<Page>(*eid) {
                if page_comp.0 != active_tab { continue; }
                if let Some(header) = world.get_component::<UIHeader>(*eid) {
                    if let Some(pos) = world.get_component::<Position>(*eid) {
                        let sy = pos.y - scroll_y;
                        let page_tag = page_comp.0.as_str();
                        if page_tag == "h" {
                            let title = &header.title;
                            let scale = 29.0;
                            let text_width = self.measure_text(title) * scale * scale_factor;
                            let x = (win_w - text_width) / 2.0;
                            self.queue_content_text(title, x, sy + 40.0 * scale_factor, scale, [1.0, 1.0, 1.0, 1.0], scale_factor);
                            if !header.greeting.is_empty() {
                                self.queue_content_text(&header.greeting, 18.0 * scale_factor, sy + 74.0 * scale_factor, 27.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
                            }
                        } else {
                            self.queue_content_text(&header.title, 18.0 * scale_factor, sy + 40.0 * scale_factor, 29.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
                            if !header.greeting.is_empty() {
                                self.queue_content_text(&header.greeting, 18.0 * scale_factor, sy + 74.0 * scale_factor, 27.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
                            }
                        }
                    }
                }
            }
        }

        // Horizontal cards (top section)
        for eid in entities.iter() {
            if let Some(page) = world.get_component::<Page>(*eid) { if page.0 != active_tab { continue; } }
            if let Some(card) = world.get_component::<UICard>(*eid) {
                if let (Some(pos), Some(size)) = (world.get_component::<Position>(*eid), world.get_component::<Size>(*eid)) {
                    if pos.y < section_divider_y {
                        let sy = pos.y - scroll_y;
                        if sy > -size.height && sy < win_h {
                            let strength = if card.is_hovered { 1.1 } else { 0.88 };
                            self.render_horizontal_card(pos.x, sy, size.width, size.height, win_w, win_h, card.tint, strength, &card.title, card.subtitle.as_deref(), scale_factor);
                        }
                    }
                }
            }
        }

        // Section labels
        for eid in entities.iter() {
            if let Some(page) = world.get_component::<Page>(*eid) { if page.0 != active_tab { continue; } }
            if let Some(section) = world.get_component::<UISection>(*eid) {
                if let Some(pos) = world.get_component::<Position>(*eid) {
                    let sy = pos.y - scroll_y;
                    match section.title.as_str() {
                        "Recommended" => self.queue_content_text("Рекомендуемые", 18.0 * scale_factor, sy - 25.0 * scale_factor, 21.0, [1.0, 1.0, 1.0, 1.0], scale_factor),
                        "NewRelease" => self.queue_content_text("Новый релиз исполнителя", 18.0 * scale_factor, sy - 25.0 * scale_factor, 11.0, [1.0, 1.0, 1.0, 0.45], scale_factor),
                        "ForYou" => self.queue_content_text("Для вас", 18.0 * scale_factor, sy - 25.0 * scale_factor, 21.0, [1.0, 1.0, 1.0, 1.0], scale_factor),
                        "SearchHeader" => {
                            self.queue_content_text("Поиск", 18.0 * scale_factor, sy + 40.0 * scale_factor, 28.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
                            self.content_vertices.extend(build_glass_quad(12.0 * scale_factor, sy + 70.0 * scale_factor, win_w - 24.0 * scale_factor, 45.0 * scale_factor, win_w, win_h, [0.2, 0.2, 0.2], 1.0, [12.0 * scale_factor; 4]));
                            self.queue_content_text("Трек, альбом, артист...", 18.0 * scale_factor, sy + 98.0 * scale_factor, 12.5, [1.0, 1.0, 1.0, 0.4], scale_factor);
                        }, _ => {}
                    }
                }
            }
        }

        // Vertical cards (bottom section)
        for eid in entities.iter() {
            if let Some(page) = world.get_component::<Page>(*eid) { if page.0 != active_tab { continue; } }
            if let Some(card) = world.get_component::<UICard>(*eid) {
                if let (Some(pos), Some(size)) = (world.get_component::<Position>(*eid), world.get_component::<Size>(*eid)) {
                    if pos.y >= section_divider_y {
                        let sy = pos.y - scroll_y;
                        if sy > -size.height && sy < win_h {
                            let strength = if card.is_hovered { 1.1 } else { 0.88 };
                            self.render_vertical_card(pos.x, sy, size.width, size.height, win_w, win_h, card.tint, strength, &card.title, card.subtitle.as_deref(), scale_factor);
                        }
                    }
                }
            }
        }

        // === UI LAYER: Navbar ===
        for eid in world.query_with_mut::<UINavBar>() {
            if let (Some(pos), Some(size)) = (world.get_component::<Position>(eid), world.get_component::<Size>(eid)) {
                let r = size.height * 0.5;
                self.scene_blur_ui_vertices.extend(build_glass_quad(pos.x, pos.y, size.width, size.height, win_w, win_h, [0.15, 0.15, 0.2], 1.2, [r; 4]));
            }
        }
        for eid in world.query_with_mut::<UINavButton>() {
            if let (Some(btn), Some(pos), Some(size)) = (world.get_component::<UINavButton>(eid), world.get_component::<Position>(eid), world.get_component::<Size>(eid)) {
                let color = if btn.is_active { [1.0, 1.0, 1.0, 1.0] } else { [1.0, 1.0, 1.0, 0.55] };
                self.ui_vector_vertices.extend(self.build_icon_verts(&btn.id, pos.x + (size.width - 22.0 * scale_factor) * 0.5, pos.y + 9.0 * scale_factor, 22.0 * scale_factor, color));
                let text_w = self.measure_text(&btn.label) * 11.0 * scale_factor;
                self.render_text_scaled(&btn.label, pos.x + (size.width - text_w) * 0.5, pos.y + 42.0 * scale_factor, 11.0, color, scale_factor);
            }
        }

        // === UI LAYER: Fullscreen Player ===
        for eid in world.query_with_mut::<Player>() {
            if let Some(renderable) = world.get_component::<Renderable>(eid) {
                if !renderable.visible { continue; }
            }
            let (pos_x, pos_y, size_w, size_h, player_title, player_artist, player_is_playing, player_is_liked, player_progress) = {
                if let (Some(pos), Some(size), Some(player)) = (
                    world.get_component::<Position>(eid),
                    world.get_component::<Size>(eid),
                    world.get_component::<Player>(eid)
                ) {
                    (pos.x, pos.y, size.width, size.height, player.title.clone(), player.artist.clone(), player.is_playing, player.is_liked, player.progress)
                } else { continue; }
            };
            let sy = pos_y - scroll_y;
            let bg_radii = [28.0 * scale_factor; 4];
            self.ui_vertices.extend(build_glass_quad(pos_x, pos_y, size_w, size_h, win_w, win_h, [0.08, 0.08, 0.1], 0.7, bg_radii));

            let art_size = f32::min(size_w, size_h) * 0.45;
            let art_x = pos_x + (size_w - art_size) * 0.5;
            let art_y = sy + size_h * 0.18;
            for &art_eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(art_eid) {
                    if p == "p" && world.get_component::<UIImage>(art_eid).is_some() {
                        if let Some(art_pos) = world.get_component_mut::<Position>(art_eid) {
                            art_pos.x = art_x;
                            art_pos.y = art_y;
                        }
                        if let Some(art_size_comp) = world.get_component_mut::<Size>(art_eid) {
                            art_size_comp.width = art_size;
                            art_size_comp.height = art_size;
                        }
                        let art_radii = [12.0 * scale_factor; 4];
                        self.ui_vertices.extend(self.build_rounded_cover_quad(art_x, art_y, art_size, art_radii));
                        break;
                    }
                }
            }

            let text_y = art_y + art_size + 40.0 * s;
            self.render_text_scaled(&player_title, pos_x + 20.0 * s, text_y, 28.0, [1.0, 1.0, 1.0, 1.0], scale_factor);
            self.render_text_scaled(&player_artist, pos_x + 20.0 * s, text_y + 40.0 * s, 16.0, [1.0, 1.0, 1.0, 0.6], scale_factor);

            for &like_eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(like_eid) {
                    if p == "p" && world.get_component::<PlayerControl>(like_eid).map(|c| c.control_type == ControlType::Like).unwrap_or(false) {
                        if let Some(like_pos) = world.get_component_mut::<Position>(like_eid) {
                            like_pos.x = pos_x + size_w - 60.0 * s;
                            like_pos.y = pos_y + 20.0 * s;
                        }
                        if let Some(like_size) = world.get_component_mut::<Size>(like_eid) {
                            like_size.width = 40.0 * s;
                            like_size.height = 40.0 * s;
                        }
                        let btn_color = if player_is_liked { [0.65, 0.55, 0.98, 1.0] } else { [1.0, 1.0, 1.0, 0.7] };
                        let like_radii = [20.0 * scale_factor; 4];
                        self.ui_vertices.extend(build_glass_quad(pos_x + size_w - 60.0 * s, pos_y + 20.0 * s, 40.0 * s, 40.0 * s, win_w, win_h, [0.15, 0.15, 0.15], 0.8, like_radii));
                        self.ui_vector_vertices.extend(self.build_icon_verts("heart", pos_x + size_w - 54.0 * s, pos_y + 26.0 * s, 20.0 * s, btn_color));
                        break;
                    }
                }
            }
            for &close_eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(close_eid) {
                    if p == "p" && world.get_component::<PlayerControl>(close_eid).map(|c| c.control_type == ControlType::Close).unwrap_or(false) {
                        if let Some(close_pos) = world.get_component_mut::<Position>(close_eid) {
                            close_pos.x = pos_x + 20.0 * s;
                            close_pos.y = pos_y + 20.0 * s;
                        }
                        if let Some(close_size) = world.get_component_mut::<Size>(close_eid) {
                            close_size.width = 40.0 * s;
                            close_size.height = 40.0 * s;
                        }
                        let close_radii = [20.0 * scale_factor; 4];
                        self.ui_vertices.extend(build_glass_quad(pos_x + 20.0 * s, pos_y + 20.0 * s, 40.0 * s, 40.0 * s, win_w, win_h, [0.15, 0.15, 0.15], 0.8, close_radii));
                        self.render_text_scaled("✕", pos_x + 28.0 * s, pos_y + 26.0 * s, 20.0 * s, [1.0, 1.0, 1.0, 0.9], scale_factor);
                        break;
                    }
                }
            }

            let controls_y = pos_y + size_h - 200.0 * s;
            let btn_sz = 72.0 * s;
            let play_sz = 72.0 * s * 1.3;
            let total_ctrl_w = btn_sz * 2.0 + play_sz + 40.0 * s;
            let ctrl_start_x = pos_x + (size_w - total_ctrl_w) / 2.0;

            let mut control_order = vec![ControlType::Prev, ControlType::Play, ControlType::Next];
            for (idx, ctrl_type) in control_order.iter().enumerate() {
                for &ctrl_eid in &entities {
                    if let Some(Page(p)) = world.get_component::<Page>(ctrl_eid) {
                        if p == "p" && world.get_component::<PlayerControl>(ctrl_eid).map(|c| c.control_type == *ctrl_type).unwrap_or(false) {
                            if let Some(ctrl_pos) = world.get_component_mut::<Position>(ctrl_eid) {
                                let x_offset = match idx { 0 => 0.0, 1 => btn_sz + 20.0 * s, 2 => btn_sz * 2.0 + 40.0 * s, _ => 0.0 };
                                ctrl_pos.x = ctrl_start_x + x_offset;
                                ctrl_pos.y = controls_y;
                            }
                            if let Some(ctrl_size) = world.get_component_mut::<Size>(ctrl_eid) {
                                ctrl_size.width = if *ctrl_type == ControlType::Play { play_sz } else { btn_sz };
                                ctrl_size.height = if *ctrl_type == ControlType::Play { play_sz } else { btn_sz };
                            }
                            let size = if *ctrl_type == ControlType::Play { play_sz } else { btn_sz };
                            let btn_radii = [size * 0.5; 4];
                            self.ui_vertices.extend(build_glass_quad(
                                ctrl_start_x + match idx { 0 => 0.0, 1 => btn_sz + 20.0*s, 2 => btn_sz*2.0 + 40.0*s, _ => 0.0 },
                                controls_y,
                                size, size,
                                win_w, win_h,
                                [0.12, 0.12, 0.12],
                                0.6,
                                btn_radii
                            ));
                            let icon_name = match ctrl_type {
                                ControlType::Prev => "prev",
                                ControlType::Play => if player_is_playing { "pause" } else { "play" },
                                ControlType::Next => "next",
                                _ => "",
                            };
                            let icon_size = size * 0.5;
                            let icon_x = ctrl_start_x + match idx {
                                0 => (btn_sz - icon_size) / 2.0,
                                1 => (play_sz - icon_size) / 2.0,
                                2 => (btn_sz - icon_size) / 2.0,
                                _ => 0.0,
                            } + match idx {
                                0 => 0.0,
                                1 => btn_sz + 20.0*s,
                                2 => btn_sz*2.0 + 40.0*s,
                                _ => 0.0,
                            };
                            let icon_y = controls_y + (size - icon_size) / 2.0;
                            self.ui_vector_vertices.extend(self.build_icon_verts(icon_name, icon_x, icon_y, icon_size, [1.0, 1.0, 1.0, 0.95]));
                        }
                    }
                }
            }

            let sr_y = controls_y + 100.0 * s;
            let sr_x_start = pos_x + size_w * 0.35;
            for &sr_eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(sr_eid) {
                    if p == "p" && world.get_component::<PlayerControl>(sr_eid).is_some() {
                        let control_type = world.get_component::<PlayerControl>(sr_eid).unwrap().control_type;
                        let sr_idx = match control_type { ControlType::Shuffle => 0, ControlType::Repeat => 1, _ => continue, };
                        let btn_x = sr_x_start + sr_idx as f32 * 80.0 * s;
                        if let Some(sr_pos) = world.get_component_mut::<Position>(sr_eid) {
                            sr_pos.x = btn_x;
                            sr_pos.y = sr_y;
                        }
                        if let Some(sr_size) = world.get_component_mut::<Size>(sr_eid) {
                            sr_size.width = 40.0 * s;
                            sr_size.height = 40.0 * s;
                        }
                        let btn_radii = [20.0 * scale_factor; 4];
                        self.ui_vertices.extend(build_glass_quad(btn_x, sr_y, 40.0 * s, 40.0 * s, win_w, win_h, [0.12, 0.12, 0.12], 0.6, btn_radii));
                        let icon_name = match control_type { ControlType::Shuffle => "shuffle", ControlType::Repeat => "repeat", _ => "", };
                        self.ui_vector_vertices.extend(self.build_icon_verts(icon_name, btn_x + 12.0 * s, sr_y + 10.0 * s, 20.0 * s, [1.0, 1.0, 1.0, 0.7]));
                    }
                }
            }

            let prog_y = pos_y + size_h - 160.0 * s;
            for &prog_eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(prog_eid) {
                    if p == "p" && world.get_component::<ProgressBar>(prog_eid).is_some() {
                        if let Some(prog_pos) = world.get_component_mut::<Position>(prog_eid) {
                            prog_pos.x = pos_x + 28.0 * s;
                            prog_pos.y = prog_y;
                        }
                        if let Some(prog_size) = world.get_component_mut::<Size>(prog_eid) {
                            prog_size.width = size_w - 56.0 * s;
                            prog_size.height = 18.0 * s;
                        }
                        let progress = player_progress;
                        Self::render_progress_bar(&mut self.ui_vertices, pos_x + 28.0 * s, prog_y, size_w - 56.0 * s, 18.0 * s, win_w, win_h, progress);
                    }
                }
            }
        }

        // === UI LAYER: MiniPlayer ===
        for eid in world.query_with_mut::<MiniPlayer>() {
            if let Some(renderable) = world.get_component::<Renderable>(eid) {
                if !renderable.visible { continue; }
            }
            let (pos_x, pos_y, size_w, size_h, mini_title, mini_artist, mini_is_playing, mini_progress) = {
                if let (Some(pos), Some(size), Some(mini)) = (
                    world.get_component::<Position>(eid),
                    world.get_component::<Size>(eid),
                    world.get_component::<MiniPlayer>(eid)
                ) {
                    (pos.x, pos.y, size.width, size.height, mini.title.clone(), mini.artist.clone(), mini.is_playing, mini.progress)
                } else { continue; }
            };
            let bg_radii = [16.0 * scale_factor; 4];
            self.scene_blur_ui_vertices.extend(build_glass_quad(pos_x, pos_y, size_w, size_h, win_w, win_h, [0.12, 0.12, 0.14], 0.5, bg_radii));

            let art_size = 34.0 * s;
            let art_x = pos_x + 12.0 * s;
            let art_y = pos_y + (size_h - art_size) * 0.5;
            for &art_eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(art_eid) {
                    if p == "min" && world.get_component::<UIImage>(art_eid).is_some() {
                        if let Some(art_pos) = world.get_component_mut::<Position>(art_eid) {
                            art_pos.x = art_x;
                            art_pos.y = art_y;
                        }
                        if let Some(art_size_comp) = world.get_component_mut::<Size>(art_eid) {
                            art_size_comp.width = art_size;
                            art_size_comp.height = art_size;
                        }
                        let art_radii = [8.0 * scale_factor; 4];
                        self.ui_vertices.extend(self.build_rounded_cover_quad(art_x, art_y, art_size, art_radii));
                        break;
                    }
                }
            }

            let text_x = pos_x + 54.0 * s;
            let text_y = pos_y + (size_h - 20.0 * s) * 0.5;
            self.render_text_scaled(&mini_title, text_x, text_y, 12.0, [1.0, 1.0, 1.0, 0.9], scale_factor);
            self.render_text_scaled(&mini_artist, text_x, text_y + 16.0 * s, 10.0, [1.0, 1.0, 1.0, 0.5], scale_factor);

            let pb_x = pos_x + size_w - 80.0 * s;
            let pb_y = pos_y + (size_h - 26.0 * s) * 0.5;
            for &pb_eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(pb_eid) {
                    if p == "min" && world.get_component::<PlayerControl>(pb_eid).is_some() {
                        let control_type = world.get_component::<PlayerControl>(pb_eid).unwrap().control_type;
                        let x = if control_type == ControlType::Play { pb_x } else { pb_x + 40.0 * s };
                        if let Some(pb_pos) = world.get_component_mut::<Position>(pb_eid) {
                            pb_pos.x = x;
                            pb_pos.y = pb_y;
                        }
                        if let Some(pb_size) = world.get_component_mut::<Size>(pb_eid) {
                            pb_size.width = 26.0 * s;
                            pb_size.height = 26.0 * s;
                        }
                        let icon_name = if control_type == ControlType::Play { if mini_is_playing { "pause" } else { "play" } } else { "next" };
                        self.ui_vector_vertices.extend(self.build_icon_verts(icon_name, x + 2.0*s, pb_y + 2.0*s, 22.0 * s, [1.0, 1.0, 1.0, 0.85]));
                    }
                }
            }

            let prog_x = pos_x + 14.0 * s;
            let prog_y = pos_y + size_h - 2.0 * s;
            for &prog_eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(prog_eid) {
                    if p == "min" && world.get_component::<ProgressBar>(prog_eid).is_some() {
                        if let Some(prog_pos) = world.get_component_mut::<Position>(prog_eid) {
                            prog_pos.x = prog_x;
                            prog_pos.y = prog_y;
                        }
                        if let Some(prog_size) = world.get_component_mut::<Size>(prog_eid) {
                            prog_size.width = size_w - 28.0 * s;
                            prog_size.height = 2.0;
                        }
                        let p = mini_progress;
                        self.ui_vertices.extend(build_glass_quad(prog_x, prog_y, size_w - 28.0 * s, 2.0, win_w, win_h, [0.2, 0.2, 0.2], 0.3, [1.0; 4]));
                        if p > 0.0 {
                            let fill_w = (size_w - 28.0 * s) * p;
                            self.ui_vertices.push(Vertex { position: [prog_x, prog_y], tex_coord: [0.0, 0.0], screen_uv: [0.0; 2], color: [0.65, 0.55, 0.98, 0.85], size: [fill_w, 2.0], radii: [0.0; 4] });
                        }
                        break;
                    }
                }
            }
        }

        for vertices in [
            &mut self.content_vertices,
            &mut self.ui_vertices,
            &mut self.scene_blur_ui_vertices,
            &mut self.content_vector_vertices,
            &mut self.ui_vector_vertices,
        ] {
            for v in vertices.iter_mut().take(10_000) {
                v.position[0] = (v.position[0] / win_w) * 2.0 - 1.0;
                v.position[1] = 1.0 - (v.position[1] / win_h) * 2.0;
            }
        }

        // Finalize vertex counts. GPU buffers are sized for 10k vertices per layer.
        self.content_vertex_count = (self.content_vertices.len() as u32).min(10_000);
        self.ui_vertex_count = (self.ui_vertices.len() as u32).min(10_000);
        self.scene_blur_ui_vertex_count = (self.scene_blur_ui_vertices.len() as u32).min(10_000);
        self.content_vector_vertex_count = (self.content_vector_vertices.len() as u32).min(10_000);
        self.ui_vector_vertex_count = (self.ui_vector_vertices.len() as u32).min(10_000);
    }
}
