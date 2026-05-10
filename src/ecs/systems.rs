use crate::ecs::world::World;
use crate::ecs::components::*;
use crate::input::{InputState};
use crate::render::RenderPipeline;

pub fn system_ui_init(world: &mut World) {
    // --- HOME PAGE ---
    let h_page = "h".to_string();

    // Header
    let header = world.create_entity();
    world.add_component(header, UIHeader { 
        title: "Liquify".to_string(), 
        greeting: "Добрый вечер".to_string() 
    });
    world.add_component(header, Position { x: 0.0, y: 0.0 });
    world.add_component(header, Renderable { visible: true });
    world.add_component(header, Page(h_page.clone()));

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
        world.add_component(card, Page(h_page.clone()));
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
    world.add_component(nr, Page(h_page.clone()));

    // Recommended
    let rec_titles = ["Микс дня", "Похожее", "Энергичное", "Спокойное"];
    for title in rec_titles {
        let card = world.create_entity();
        world.add_component(card, UICard {
            title: title.to_string(),
            subtitle: None,
            icon: "radio".to_string(),
            tint: [0.1, 0.1, 0.1],
            is_hovered: false,
        });
        world.add_component(card, UISection { title: "Recommended".to_string() });
        world.add_component(card, Position { x: 0.0, y: 0.0 });
        world.add_component(card, Size { width: 0.0, height: 0.0 });
        world.add_component(card, Renderable { visible: true });
        world.add_component(card, Page(h_page.clone()));
    }

    // For You
    let mix_titles = ["Микс дня 1", "Открытия недели", "Ежедневный микс"];
    for title in mix_titles {
        let card = world.create_entity();
        world.add_component(card, UICard {
            title: title.to_string(),
            subtitle: Some("Основано на вашем вкусе".to_string()),
            icon: "music".to_string(),
            tint: [0.2, 0.2, 0.2],
            is_hovered: false,
        });
        world.add_component(card, UISection { title: "ForYou".to_string() });
        world.add_component(card, Position { x: 0.0, y: 0.0 });
        world.add_component(card, Size { width: 0.0, height: 0.0 });
        world.add_component(card, Renderable { visible: true });
        world.add_component(card, Page(h_page.clone()));
    }

    // --- SEARCH PAGE ---
    let s_page = "s".to_string();
    let s_header = world.create_entity();
    world.add_component(s_header, UISection { title: "SearchHeader".to_string() });
    world.add_component(s_header, Position { x: 0.0, y: 0.0 });
    world.add_component(s_header, Renderable { visible: true });
    world.add_component(s_header, Page(s_page.clone()));

    // Search Categories
    let categories = [
        ("Поп", [0.93, 0.28, 0.6]),
        ("Хип-хоп", [0.96, 0.62, 0.04]),
        ("Рок", [0.94, 0.27, 0.27]),
        ("Танцевальная", [0.06, 0.73, 0.51]),
        ("Для отдыха", [0.23, 0.51, 0.96]),
        ("Инди", [0.65, 0.55, 0.98]),
        ("Джаз", [0.5, 0.5, 0.5]),
        ("Классика", [0.3, 0.6, 0.9]),
    ];

    for (title, tint) in categories {
        let card = world.create_entity();
        world.add_component(card, UICard {
            title: title.to_string(),
            subtitle: None,
            icon: "music".to_string(),
            tint,
            is_hovered: false,
        });
        world.add_component(card, UISection { title: "SearchCategory".to_string() });
        world.add_component(card, Position { x: 0.0, y: 0.0 });
        world.add_component(card, Size { width: 0.0, height: 0.0 });
        world.add_component(card, Renderable { visible: true });
        world.add_component(card, Page(s_page.clone()));
    }

    // --- LIBRARY PAGE ---
    let l_page = "l".to_string();
    let l_header = world.create_entity();
    world.add_component(l_header, UIHeader { title: "Медиатека".to_string(), greeting: "".to_string() });
    world.add_component(l_header, Position { x: 0.0, y: 0.0 });
    world.add_component(l_header, Renderable { visible: true });
    world.add_component(l_header, Page(l_page.clone()));

    let lib_items = [
        ("Любимые треки", "Плейлист · 128 треков", "heart", [0.6, 0.4, 1.0]),
        ("Ваш микс", "Плейлист · Liquify", "music", [0.2, 0.8, 0.6]),
        ("Radio Relax", "Станция", "radio", [1.0, 0.6, 0.2]),
        ("Imagine Dragons", "Исполнитель", "music", [0.5, 0.5, 0.5]),
        ("After Hours", "Альбом · The Weeknd", "image", [0.9, 0.1, 0.1]),
    ];

    for (title, sub, icon, tint) in lib_items {
        let card = world.create_entity();
        world.add_component(card, UICard {
            title: title.to_string(),
            subtitle: Some(sub.to_string()),
            icon: icon.to_string(),
            tint,
            is_hovered: false,
        });
        world.add_component(card, UISection { title: "LibraryList".to_string() });
        world.add_component(card, Position { x: 0.0, y: 0.0 });
        world.add_component(card, Size { width: 0.0, height: 0.0 });
        world.add_component(card, Renderable { visible: true });
        world.add_component(card, Page(l_page.clone()));
    }

    // --- SETTINGS PAGE ---
    let sett_page = "sett".to_string();
    let sett_header = world.create_entity();
    world.add_component(sett_header, UISection { title: "SettingsHeader".to_string() });
    world.add_component(sett_header, Position { x: 0.0, y: 0.0 });
    world.add_component(sett_header, Renderable { visible: true });
    world.add_component(sett_header, Page(sett_page.clone()));

    let settings_options = [
        ("Spotify", "Не подключено", "radio"),
        ("Обои", "Настройка фона", "image"),
        ("Размытие фона", "Настройка эффектов", "settings"),
        ("Анимация интерфейса", "Включено", "check"),
        ("Кэширование", "840 MB", "list"),
    ];

    for (title, sub, icon) in settings_options {
        let card = world.create_entity();
        world.add_component(card, UICard {
            title: title.to_string(),
            subtitle: Some(sub.to_string()),
            icon: icon.to_string(),
            tint: [0.15, 0.15, 0.15],
            is_hovered: false,
        });
        world.add_component(card, UISection { title: "SettingsList".to_string() });
        world.add_component(card, Position { x: 0.0, y: 0.0 });
        world.add_component(card, Size { width: 0.0, height: 0.0 });
        world.add_component(card, Renderable { visible: true });
        world.add_component(card, Page(sett_page.clone()));
    }

    // --- NAVIGATION BAR (GLOBAL) ---
    let navbar = world.create_entity();
    world.add_component(navbar, UINavBar { active_tab: "h".to_string() });
    world.add_component(navbar, Position { x: 0.0, y: 0.0 });
    world.add_component(navbar, Size { width: 0.0, height: 0.0 });
    world.add_component(navbar, Renderable { visible: true });

    let nav_buttons = [
        ("h", "Главная", "h"),
        ("s", "Поиск", "s"),
        ("l", "Медиатека", "l"),
        ("sett", "Настройки", "sett"),
    ];

    for (id, label, icon) in nav_buttons {
        let btn = world.create_entity();
        world.add_component(btn, UINavButton {
            id: id.to_string(),
            label: label.to_string(),
            icon: icon.to_string(),
            is_active: id == "h",
        });
        world.add_component(btn, Position { x: 0.0, y: 0.0 });
        world.add_component(btn, Size { width: 0.0, height: 0.0 });
        world.add_component(btn, Renderable { visible: true });
    }
}

pub fn system_layout(world: &mut World, win_w: f32, win_h: f32, scale_factor: f32) -> f32 {
    let s = scale_factor;
    let margin = 18.0 * s;
    
    // 1. Determine active tab
    let mut active_tab = "h".to_string();
    for eid in world.query_with_mut::<UINavBar>() {
        if let Some(nav) = world.get_component::<UINavBar>(eid) {
            active_tab = nav.active_tab.clone();
        }
    }

    let mut active_page_max_y = 0.0;

    // 2. Layout all entities based on their page
    // We'll iterate through all entities and layout them if they belong to a page
    let entities: Vec<u32> = world.entities.all_entities().iter().map(|e| e.id).collect();
    
    // --- HOME PAGE LAYOUT ---
    let mut h_y = 45.0 * s;
    if active_tab == "h" {
        // Header
        for &eid in &entities {
            if let (Some(Page(p)), Some(_)) = (world.get_component::<Page>(eid), world.get_component::<UIHeader>(eid)) {
                if p == "h" {
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin;
                        pos.y = h_y;
                    }
                }
            }
        }
        h_y += 85.0 * s;

        // Quick Grid
        let grid_gap = 10.0 * s;
        let card_w = (win_w - margin * 2.0 - grid_gap) / 2.0;
        let card_h = 56.0 * s;
        let mut qg_idx = 0;
        for &eid in &entities {
            if let (Some(Page(p)), Some(sec)) = (world.get_component::<Page>(eid), world.get_component::<UISection>(eid)) {
                if p == "h" && sec.title == "QuickGrid" {
                    let row = qg_idx / 2;
                    let col = qg_idx % 2;
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin + col as f32 * (card_w + grid_gap);
                        pos.y = h_y + row as f32 * (card_h + grid_gap);
                    }
                    if let Some(size) = world.get_component_mut::<Size>(eid) {
                        size.width = card_w;
                        size.height = card_h;
                    }
                    qg_idx += 1;
                }
            }
        }
        h_y += 3.0 * (card_h + grid_gap) + 20.0 * s;

        // New Release
        for &eid in &entities {
            if let (Some(Page(p)), Some(sec)) = (world.get_component::<Page>(eid), world.get_component::<UISection>(eid)) {
                if p == "h" && sec.title == "NewRelease" {
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin;
                        pos.y = h_y + 30.0 * s;
                    }
                    if let Some(size) = world.get_component_mut::<Size>(eid) {
                        size.width = win_w - margin * 2.0;
                        size.height = 140.0 * s;
                    }
                    h_y += 170.0 * s + 40.0 * s;
                }
            }
        }

        // Recommended
        let mut rec_idx = 0;
        for &eid in &entities {
            if let (Some(Page(p)), Some(sec)) = (world.get_component::<Page>(eid), world.get_component::<UISection>(eid)) {
                if p == "h" && sec.title == "Recommended" {
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin + rec_idx as f32 * (165.0 * s);
                        pos.y = h_y + 35.0 * s;
                    }
                    if let Some(size) = world.get_component_mut::<Size>(eid) {
                        size.width = 150.0 * s;
                        size.height = 200.0 * s;
                    }
                    rec_idx += 1;
                }
            }
        }
        h_y += 240.0 * s;

        // For You
        let mut fy_idx = 0;
        for &eid in &entities {
            if let (Some(Page(p)), Some(sec)) = (world.get_component::<Page>(eid), world.get_component::<UISection>(eid)) {
                if p == "h" && sec.title == "ForYou" {
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin + fy_idx as f32 * (170.0 * s);
                        pos.y = h_y + 35.0 * s;
                    }
                    if let Some(size) = world.get_component_mut::<Size>(eid) {
                        size.width = 155.0 * s;
                        size.height = 210.0 * s;
                    }
                    fy_idx += 1;
                }
            }
        }
        h_y += 250.0 * s;
        active_page_max_y = h_y;
    }

    // --- SEARCH PAGE LAYOUT ---
    let mut s_y = 45.0 * s;
    if active_tab == "s" {
        for &eid in &entities {
            if let (Some(Page(p)), Some(sec)) = (world.get_component::<Page>(eid), world.get_component::<UISection>(eid)) {
                if p == "s" && sec.title == "SearchHeader" {
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin;
                        pos.y = s_y;
                    }
                }
            }
        }
        s_y += 150.0 * s;

        let grid_gap = 12.0 * s;
        let card_w = (win_w - margin * 2.0 - grid_gap) / 2.0;
        let card_h = 100.0 * s;
        let mut cat_idx = 0;
        for &eid in &entities {
            if let (Some(Page(p)), Some(sec)) = (world.get_component::<Page>(eid), world.get_component::<UISection>(eid)) {
                if p == "s" && sec.title == "SearchCategory" {
                    let row = cat_idx / 2;
                    let col = cat_idx % 2;
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin + col as f32 * (card_w + grid_gap);
                        pos.y = s_y + row as f32 * (card_h + grid_gap);
                    }
                    if let Some(size) = world.get_component_mut::<Size>(eid) {
                        size.width = card_w;
                        size.height = card_h;
                    }
                    cat_idx += 1;
                }
            }
        }
        s_y += ((cat_idx + 1) / 2) as f32 * (card_h + grid_gap);
        active_page_max_y = s_y;
    }

    // --- LIBRARY PAGE LAYOUT ---
    let mut l_y = 45.0 * s;
    if active_tab == "l" {
        for &eid in &entities {
            if let (Some(Page(p)), Some(_)) = (world.get_component::<Page>(eid), world.get_component::<UIHeader>(eid)) {
                if p == "l" {
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin;
                        pos.y = l_y;
                    }
                }
            }
        }
        l_y += 100.0 * s;

        let item_h = 70.0 * s;
        let item_gap = 1.0 * s;
        for &eid in &entities {
            if let (Some(Page(p)), Some(sec)) = (world.get_component::<Page>(eid), world.get_component::<UISection>(eid)) {
                if p == "l" && sec.title == "LibraryList" {
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin;
                        pos.y = l_y;
                    }
                    if let Some(size) = world.get_component_mut::<Size>(eid) {
                        size.width = win_w - margin * 2.0;
                        size.height = item_h;
                    }
                    l_y += item_h + item_gap;
                }
            }
        }
        active_page_max_y = l_y;
    }

    // --- SETTINGS PAGE LAYOUT ---
    let mut sett_y = 45.0 * s;
    if active_tab == "sett" {
        for &eid in &entities {
            if let (Some(Page(p)), Some(sec)) = (world.get_component::<Page>(eid), world.get_component::<UISection>(eid)) {
                if p == "sett" && sec.title == "SettingsHeader" {
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin;
                        pos.y = sett_y;
                    }
                }
            }
        }
        sett_y += 100.0 * s;

        let item_h = 65.0 * s;
        let item_gap = 10.0 * s;
        for &eid in &entities {
            if let (Some(Page(p)), Some(sec)) = (world.get_component::<Page>(eid), world.get_component::<UISection>(eid)) {
                if p == "sett" && sec.title == "SettingsList" {
                    if let Some(pos) = world.get_component_mut::<Position>(eid) {
                        pos.x = margin;
                        pos.y = sett_y;
                    }
                    if let Some(size) = world.get_component_mut::<Size>(eid) {
                        size.width = win_w - margin * 2.0;
                        size.height = item_h;
                    }
                    sett_y += item_h + item_gap;
                }
            }
        }
        active_page_max_y = sett_y;
    }

    // --- GLOBAL: NAVIGATION BAR ---
    let nav_h = 65.0 * s;
    let nav_y = win_h - nav_h - 10.0 * s;
    let nav_w = (win_w - 20.0 * s).min(400.0 * s);
    let nav_x = (win_w - nav_w) / 2.0;

    for eid in world.query_with_mut::<UINavBar>() {
        if let Some(pos) = world.get_component_mut::<Position>(eid) {
            pos.x = nav_x;
            pos.y = nav_y;
        }
        if let Some(size) = world.get_component_mut::<Size>(eid) {
            size.width = nav_w;
            size.height = nav_h;
        }
    }

    let btn_w = nav_w / 4.0;
    let nav_eids: Vec<u32> = world.query_with_mut::<UINavButton>();
    for eid in nav_eids {
        let btn_idx = match world.get_component::<UINavButton>(eid).map(|b| b.id.as_str()) {
            Some("h") => 0,
            Some("s") => 1,
            Some("l") => 2,
            Some("sett") => 3,
            _ => 0,
        };
        if let Some(pos) = world.get_component_mut::<Position>(eid) {
            pos.x = nav_x + btn_idx as f32 * btn_w;
            pos.y = nav_y;
        }
        if let Some(size) = world.get_component_mut::<Size>(eid) {
            size.width = btn_w;
            size.height = nav_h;
        }
    }

    active_page_max_y + 100.0 * s
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
