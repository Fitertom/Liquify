use crate::ecs::world::World;
use crate::ecs::components::*;
use crate::input::{InputState};
use crate::render::RenderPipeline;

pub fn system_ui_init(world: &mut World) {
    // Create Header
    let header = world.create_entity();
    world.add_component(header, UIHeader { 
        title: "Liquify".to_string(), 
        greeting: "Добрый вечер".to_string() 
    });
    world.add_component(header, Position { x: 0.0, y: 0.0 });
    world.add_component(header, Renderable { visible: true });

    // Quick Grid Cards
    let titles = [
        ("Любимые треки", [0.65, 0.55, 0.98], "heart"),
        ("Недавно играло", [0.96, 0.62, 0.04], "play"),
        ("Все треки", [0.94, 0.27, 0.27], "music"),
        ("Моя медиатека", [0.06, 0.73, 0.51], "image"),
        ("Поиск треков", [0.23, 0.51, 0.96], "search"),
        ("Плейлисты", [0.93, 0.28, 0.6], "list"),
    ];

    for (title, tint, icon) in titles {
        let card = world.create_entity();
        world.add_component(card, UICard {
            title: title.to_string(),
            subtitle: None,
            icon: icon.to_string(),
            tint,
            is_hovered: false,
        });
        world.add_component(card, UISection { title: "QuickGrid".to_string() });
        world.add_component(card, Position { x: 0.0, y: 0.0 });
        world.add_component(card, Size { width: 0.0, height: 0.0 });
        world.add_component(card, Renderable { visible: true });
    }

    // New Release
    let nr = world.create_entity();
    world.add_component(nr, UICard {
        title: "Недавно добавлено".to_string(),
        subtitle: Some("Сингл · Выберите трек".to_string()),
        icon: "music".to_string(),
        tint: [0.0, 0.0, 0.0],
        is_hovered: false,
    });
    world.add_component(nr, UISection { title: "NewRelease".to_string() });
    world.add_component(nr, Position { x: 0.0, y: 0.0 });
    world.add_component(nr, Size { width: 0.0, height: 0.0 });
    world.add_component(nr, Renderable { visible: true });

    // Recommended
    let rec_titles = ["Микс дня", "Похожее", "Энергичное"];
    for title in rec_titles {
        let card = world.create_entity();
        world.add_component(card, UICard {
            title: title.to_string(),
            subtitle: None,
            icon: "radio".to_string(),
            tint: [0.3, 0.3, 0.3],
            is_hovered: false,
        });
        world.add_component(card, UISection { title: "Recommended".to_string() });
        world.add_component(card, Position { x: 0.0, y: 0.0 });
        world.add_component(card, Size { width: 0.0, height: 0.0 });
        world.add_component(card, Renderable { visible: true });
    }
}

pub fn system_layout(world: &mut World, win_w: f32, win_h: f32, scale_factor: f32) {
    let s = scale_factor;
    let margin = 18.0 * s;
    let mut current_y = 45.0 * s;

    // Header
    let header_ids = world.query_with_mut::<UIHeader>();
    for eid in header_ids {
        if let Some(pos) = world.get_component_mut::<Position>(eid) {
            pos.x = margin;
            pos.y = current_y;
        }
    }
    current_y += 85.0 * s; // Space for header and greeting

    // Quick Grid (2 columns)
    let grid_gap = 10.0 * s;
    let card_w = (win_w - margin * 2.0 - grid_gap) / 2.0;
    let card_h = 56.0 * s;
    
    let mut qg_idx = 0;
    let qg_ids = world.query_with_mut::<UISection>();
    
    for eid in qg_ids {
        let is_qg = world.get_component::<UISection>(eid).map(|s| s.title == "QuickGrid").unwrap_or(false);
        if is_qg {
            let row = qg_idx / 2;
            let col = qg_idx % 2;
            if let Some(pos) = world.get_component_mut::<Position>(eid) {
                pos.x = margin + col as f32 * (card_w + grid_gap);
                pos.y = current_y + row as f32 * (card_h + grid_gap);
            }
            if let Some(size) = world.get_component_mut::<Size>(eid) {
                size.width = card_w;
                size.height = card_h;
            }
            qg_idx += 1;
        }
    }
    current_y += 3.0 * (card_h + grid_gap) + 20.0 * s;

    // New Release
    let nr_ids = world.query_with_mut::<UISection>();
    for eid in nr_ids {
        let is_nr = world.get_component::<UISection>(eid).map(|s| s.title == "NewRelease").unwrap_or(false);
        if is_nr {
            if let Some(pos) = world.get_component_mut::<Position>(eid) {
                pos.x = margin;
                pos.y = current_y + 30.0 * s; // Space for section title
            }
            if let Some(size) = world.get_component_mut::<Size>(eid) {
                size.width = win_w - margin * 2.0;
                size.height = 140.0 * s;
            }
            current_y += 170.0 * s + 40.0 * s;
        }
    }

    // Recommended
    let mut rec_idx = 0;
    let rec_ids = world.query_with_mut::<UISection>();
    for eid in rec_ids {
        let is_rec = world.get_component::<UISection>(eid).map(|s| s.title == "Recommended").unwrap_or(false);
        if is_rec {
            if let Some(pos) = world.get_component_mut::<Position>(eid) {
                pos.x = margin + rec_idx as f32 * (165.0 * s);
                pos.y = current_y + 35.0 * s;
            }
            if let Some(size) = world.get_component_mut::<Size>(eid) {
                size.width = 150.0 * s;
                size.height = 200.0 * s;
            }
            rec_idx += 1;
        }
    }
}

pub fn system_render(
    world: &mut World,
    renderer: &mut RenderPipeline,
    input: &InputState,
    fps_text: &str,
    win_w: f32,
    win_h: f32,
    scale_factor: f32,
) {
    renderer.build_frame_ecs(world, input, fps_text, win_w, win_h, scale_factor);
    renderer.draw();
}
