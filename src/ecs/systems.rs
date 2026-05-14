use crate::ecs::world::World;
use crate::ecs::components::*;
use crate::input::InputState;
use crate::InputAction;
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

    // --- FULLSCREEN PLAYER (initially hidden) ---
    let player = world.create_entity();
    world.add_component(player, Player {
        title: "Недавно добавлено".to_string(),
        artist: "Выберите трек".to_string(),
        progress: 0.0,
        duration: 215.0,
        is_playing: false,
        is_liked: false,
    });
    world.add_component(player, Position { x: 0.0, y: 0.0 });
    world.add_component(player, Size { width: 0.0, height: 0.0 });
    world.add_component(player, Renderable { visible: false });
    world.add_component(player, Page("p".to_string()));

    // Player controls (Prev, Play, Next)
    let control_types = [("prev", ControlType::Prev), ("play", ControlType::Play), ("next", ControlType::Next)];
    for (id, ctrl_type) in control_types {
        let ctrl = world.create_entity();
        world.add_component(ctrl, PlayerControl { control_type: ctrl_type });
        world.add_component(ctrl, Position { x: 0.0, y: 0.0 });
        world.add_component(ctrl, Size { width: 0.0, height: 0.0 });
        world.add_component(ctrl, Renderable { visible: false });
        world.add_component(ctrl, Page("p".to_string()));
    }

    // Like button
    let like_btn = world.create_entity();
    world.add_component(like_btn, PlayerControl { control_type: ControlType::Like });
    world.add_component(like_btn, Position { x: 0.0, y: 0.0 });
    world.add_component(like_btn, Size { width: 0.0, height: 0.0 });
    world.add_component(like_btn, Renderable { visible: false });
    world.add_component(like_btn, Page("p".to_string()));

    // Shuffle and Repeat buttons
    let sr_types = [("shuffle", ControlType::Shuffle), ("repeat", ControlType::Repeat)];
    for (id, ctrl_type) in sr_types {
        let ctrl = world.create_entity();
        world.add_component(ctrl, PlayerControl { control_type: ctrl_type });
        world.add_component(ctrl, Position { x: 0.0, y: 0.0 });
        world.add_component(ctrl, Size { width: 0.0, height: 0.0 });
        world.add_component(ctrl, Renderable { visible: false });
        world.add_component(ctrl, Page("p".to_string()));
    }

    // Close button (for full player)
    let close_btn = world.create_entity();
    world.add_component(close_btn, PlayerControl { control_type: ControlType::Close });
    world.add_component(close_btn, Position { x: 0.0, y: 0.0 });
    world.add_component(close_btn, Size { width: 0.0, height: 0.0 });
    world.add_component(close_btn, Renderable { visible: false });
    world.add_component(close_btn, Page("p".to_string()));

    // Progress bar
    let progress = world.create_entity();
    world.add_component(progress, ProgressBar { value: 0.0, max: 1.0 });
    world.add_component(progress, Position { x: 0.0, y: 0.0 });
    world.add_component(progress, Size { width: 0.0, height: 0.0 });
    world.add_component(progress, Renderable { visible: false });
    world.add_component(progress, Page("p".to_string()));

    // --- MINI PLAYER (visible by default with sample track) ---
    let mini = world.create_entity();
    world.add_component(mini, MiniPlayer {
        title: "Недавно добавлено".to_string(),
        artist: "Выберите трек".to_string(),
        progress: 0.35,
        is_playing: true,
    });
    world.add_component(mini, Position { x: 0.0, y: 0.0 });
    world.add_component(mini, Size { width: 0.0, height: 0.0 });
    world.add_component(mini, Renderable { visible: true });
    world.add_component(mini, Page("min".to_string()));

    // Mini player album art entity
    let mini_art = world.create_entity();
    world.add_component(mini_art, UIImage { path: "assets/temp_icon.png".to_string() });
    world.add_component(mini_art, Position { x: 0.0, y: 0.0 });
    world.add_component(mini_art, Size { width: 0.0, height: 0.0 });
    world.add_component(mini_art, Renderable { visible: true });
    world.add_component(mini_art, Page("min".to_string()));

    // Mini player controls (play/pause, next)
    let mini_controls = [
        ("play", ControlType::Play),
        ("next", ControlType::Next),
    ];
    for (id, ctrl_type) in mini_controls {
        let ctrl = world.create_entity();
        world.add_component(ctrl, PlayerControl { control_type: ctrl_type });
        world.add_component(ctrl, Position { x: 0.0, y: 0.0 });
        world.add_component(ctrl, Size { width: 0.0, height: 0.0 });
        world.add_component(ctrl, Renderable { visible: true });
        world.add_component(ctrl, Page("min".to_string()));
    }

    // Mini player progress bar
    let mini_prog = world.create_entity();
    world.add_component(mini_prog, ProgressBar { value: 0.35, max: 1.0 });
    world.add_component(mini_prog, Position { x: 0.0, y: 0.0 });
    world.add_component(mini_prog, Size { width: 0.0, height: 0.0 });
    world.add_component(mini_prog, Renderable { visible: true });
    world.add_component(mini_prog, Page("min".to_string()));
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

    // --- FULLSCREEN PLAYER LAYOUT ---
    let player_eids: Vec<u32> = world.query_with_mut::<Player>();
    if !player_eids.is_empty() {
        let player_eid = player_eids[0];
        let player_visible = world.get_component::<Renderable>(player_eid)
            .map(|r| r.visible).unwrap_or(false);

        if player_visible {
            // Position player full-screen (covers entire window)
            if let Some(pos) = world.get_component_mut::<Position>(player_eid) {
                pos.x = 0.0;
                pos.y = 0.0;
            }
            if let Some(size) = world.get_component_mut::<Size>(player_eid) {
                size.width = win_w;
                size.height = win_h;
            }

            // Layout album art (large, centered top)
            let art_size = if win_w < win_h { win_w } else { win_h } * 0.45;
            let art_x = (win_w - art_size) / 2.0;
            let art_y = win_h * 0.18; // Start around 18% from top

            // Find album art entity (has no specific component but belongs to player page)
            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "p" && world.get_component::<UIImage>(eid).is_some() {
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = art_x;
                            pos.y = art_y;
                        }
                        if let Some(size) = world.get_component_mut::<Size>(eid) {
                            size.width = art_size;
                            size.height = art_size;
                        }
                        break;
                    }
                }
            }

            // Track info (title and artist)
            let text_y = art_y + art_size + 40.0 * s;
            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "p" && world.get_component::<UIHeader>(eid).is_some() {
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = 20.0 * s;
                            pos.y = text_y;
                        }
                        break;
                    }
                }
            }

            // Like button (top right)
            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "p" && world.get_component::<PlayerControl>(eid).map(|c| c.control_type == ControlType::Like).unwrap_or(false) {
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = win_w - 60.0 * s;
                            pos.y = 20.0 * s;
                        }
                        if let Some(size) = world.get_component_mut::<Size>(eid) {
                            size.width = 40.0 * s;
                            size.height = 40.0 * s;
                        }
                        break;
                    }
                }
            }

            // Close button (top left)
            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "p" && world.get_component::<PlayerControl>(eid).map(|c| c.control_type == ControlType::Close).unwrap_or(false) {
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = 20.0 * s;
                            pos.y = 20.0 * s;
                        }
                        if let Some(size) = world.get_component_mut::<Size>(eid) {
                            size.width = 40.0 * s;
                            size.height = 40.0 * s;
                        }
                        break;
                    }
                }
            }

            // Controls (Prev, Play, Next) - bottom area
            let controls_y = win_h - 200.0 * s;
            let btn_size = 72.0 * s;
            let play_size = 72.0 * s * 1.3; // Play button slightly larger
            let total_width = (btn_size * 2.0) + play_size + 40.0 * s;
            let start_x = (win_w - total_width) / 2.0;

            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "p" && world.get_component::<PlayerControl>(eid).is_some() {
                        let ctrl = world.get_component::<PlayerControl>(eid).unwrap();
                        let idx = match ctrl.control_type {
                            ControlType::Prev => 0,
                            ControlType::Play => 1,
                            ControlType::Next => 2,
                            _ => continue,
                        };
                        let x = start_x + idx as f32 * (btn_size + 20.0 * s);
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = x;
                            pos.y = controls_y;
                        }
                        if let Some(size) = world.get_component_mut::<Size>(eid) {
                            size.width = if idx == 1 { play_size } else { btn_size };
                            size.height = if idx == 1 { play_size } else { btn_size };
                        }
                    }
                }
            }

            // Shuffle and Repeat buttons
            let sr_y = controls_y + 100.0 * s;
            let sr_start_x = win_w * 0.35;
            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "p" && world.get_component::<PlayerControl>(eid).is_some() {
                        let ctrl = world.get_component::<PlayerControl>(eid).unwrap();
                        let idx = match ctrl.control_type {
                            ControlType::Shuffle => 0,
                            ControlType::Repeat => 1,
                            _ => continue,
                        };
                        let x = sr_start_x + idx as f32 * 80.0 * s;
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = x;
                            pos.y = sr_y;
                        }
                        if let Some(size) = world.get_component_mut::<Size>(eid) {
                            size.width = 40.0 * s;
                            size.height = 40.0 * s;
                        }
                    }
                }
            }

            // Progress bar (slider)
            let prog_y = controls_y - 60.0 * s;
            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "p" && world.get_component::<ProgressBar>(eid).is_some() {
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = 28.0 * s;
                            pos.y = prog_y;
                        }
                        if let Some(size) = world.get_component_mut::<Size>(eid) {
                            size.width = win_w - 56.0 * s;
                            size.height = 18.0 * s;
                        }
                        break;
                    }
                }
            }
        }
    }

    // --- MINI PLAYER LAYOUT ---
    let mini_eids: Vec<u32> = world.query_with_mut::<MiniPlayer>();
    if !mini_eids.is_empty() {
        let mini_eid = mini_eids[0];
        let mini_visible = world.get_component::<Renderable>(mini_eid)
            .map(|r| r.visible).unwrap_or(false);

        if mini_visible {
            // Mini player positioned above nav bar
            let nav_gap = 8.0 * s;
            let nav_outer_gap = 12.0 * s;
            let mini_h = 58.0 * s;
            let mini_w = (win_w - 20.0 * s).min(386.0 * s);
            let mini_x = (win_w - mini_w) / 2.0;
            let mini_y = win_h - mini_h - nav_h - nav_gap - nav_outer_gap;

            if let Some(pos) = world.get_component_mut::<Position>(mini_eid) {
                pos.x = mini_x;
                pos.y = mini_y;
            }
            if let Some(size) = world.get_component_mut::<Size>(mini_eid) {
                size.width = mini_w;
                size.height = mini_h;
            }

            // Mini player album art (34x34)
            let art_x = mini_x + 12.0 * s;
            let art_y = mini_y + (mini_h - 34.0 * s) / 2.0;
            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "min" && world.get_component::<UIImage>(eid).is_some() {
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = art_x;
                            pos.y = art_y;
                        }
                        if let Some(size) = world.get_component_mut::<Size>(eid) {
                            size.width = 34.0 * s;
                            size.height = 34.0 * s;
                        }
                        break;
                    }
                }
            }

            // Mini player text (title and artist)
            let text_x = art_x + 42.0 * s;
            let text_y = mini_y + (mini_h - 24.0 * s) / 2.0;
            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "min" && world.get_component::<UIHeader>(eid).is_some() {
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = text_x;
                            pos.y = text_y;
                        }
                        break;
                    }
                }
            }

            // Mini player controls (play/pause and next)
            let pb_x = mini_x + mini_w - 80.0 * s;
            let pb_y = mini_y + (mini_h - 26.0 * s) / 2.0;
            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "min" && world.get_component::<PlayerControl>(eid).is_some() {
                        let ctrl = world.get_component::<PlayerControl>(eid).unwrap();
                        let x = if ctrl.control_type == ControlType::Play {
                            pb_x
                        } else {
                            pb_x + 40.0 * s
                        };
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = x;
                            pos.y = pb_y;
                        }
                        if let Some(size) = world.get_component_mut::<Size>(eid) {
                            size.width = 26.0 * s;
                            size.height = 26.0 * s;
                        }
                    }
                }
            }

            // Mini player progress bar
            let prog_x = mini_x + 14.0 * s;
            let prog_y = mini_y + mini_h - 2.0 * s;
            for &eid in &entities {
                if let Some(Page(p)) = world.get_component::<Page>(eid) {
                    if p == "min" && world.get_component::<ProgressBar>(eid).is_some() {
                        if let Some(pos) = world.get_component_mut::<Position>(eid) {
                            pos.x = prog_x;
                            pos.y = prog_y;
                        }
                        if let Some(size) = world.get_component_mut::<Size>(eid) {
                            size.width = mini_w - 28.0 * s;
                            size.height = 2.0;
                        }
                        break;
                    }
                }
            }
        }
    }

    active_page_max_y + 100.0 * s
}

pub fn system_input_ui_hover(world: &mut World, input: &mut InputState) {
    let (mx, my) = input.mouse_pos;
    input.hovered_button = None;

    let in_bounds = |mx: f32, my: f32, pos: &Position, size: &Size| {
        mx >= pos.x && mx <= pos.x + size.width && my >= pos.y && my <= pos.y + size.height
    };

    // 1) Nav buttons (top priority)
    for eid in world.query_with_mut::<UINavButton>() {
        if let (Some(pos), Some(size)) = (world.get_component::<Position>(eid), world.get_component::<Size>(eid)) {
            if in_bounds(mx, my, pos, size) {
                input.hovered_button = Some(eid as usize);
                return;
            }
        }
    }

    // 2) Player controls (close, like, shuffle, repeat, prev, play, next)
    for eid in world.query_with_mut::<PlayerControl>() {
        if let Some(renderable) = world.get_component::<Renderable>(eid) {
            if !renderable.visible { continue; }
        }
        if let (Some(pos), Some(size)) = (world.get_component::<Position>(eid), world.get_component::<Size>(eid)) {
            if in_bounds(mx, my, pos, size) {
                input.hovered_button = Some(eid as usize);
                return;
            }
        }
    }

    // 3) Mini-player (click anywhere to open full player)
    for eid in world.query_with_mut::<MiniPlayer>() {
        if let Some(renderable) = world.get_component::<Renderable>(eid) {
            if !renderable.visible { continue; }
        }
        if let (Some(pos), Some(size)) = (world.get_component::<Position>(eid), world.get_component::<Size>(eid)) {
            if in_bounds(mx, my, pos, size) {
                input.hovered_button = Some(eid as usize);
                return;
            }
        }
    }
}

pub fn system_navigation(world: &mut World, input: &InputState) {
    match input.last_action {
        InputAction::ButtonPress(eid) => {
            let eid = eid as u32;

            // Navigation bar buttons
            if let Some(btn) = world.get_component::<UINavButton>(eid) {
                let tab = btn.id.clone();
                // Update active tab
                for nav_eid in world.query_with_mut::<UINavBar>() {
                    if let Some(nav) = world.get_component_mut::<UINavBar>(nav_eid) {
                        nav.active_tab = tab.clone();
                    }
                }
                for btn_eid in world.query_with_mut::<UINavButton>() {
                    if let Some(b) = world.get_component_mut::<UINavButton>(btn_eid) {
                        b.is_active = b.id == tab;
                    }
                }
                // Tab switch: close full player if open, ensure mini-player visible
                let all_ids: Vec<u32> = world.entities.all_entities().iter().map(|e| e.id).collect();
                for entity_id in all_ids {
                    let page_tag = match world.get_component::<Page>(entity_id) {
                        Some(Page(p)) => p.clone(),
                        _ => continue,
                    };
                    if let Some(mut r) = world.get_component_mut::<Renderable>(entity_id) {
                        if page_tag == "p" {
                            r.visible = false;
                        } else if page_tag == "min" {
                            r.visible = true;
                        }
                    }
                }
            }
            // Mini-player click: open full player, hide mini
            else if world.get_component::<MiniPlayer>(eid).is_some() {
                let all_ids: Vec<u32> = world.entities.all_entities().iter().map(|e| e.id).collect();
                for entity_id in all_ids {
                    let page_tag = match world.get_component::<Page>(entity_id) {
                        Some(Page(p)) => p.clone(),
                        _ => continue,
                    };
                    if let Some(mut r) = world.get_component_mut::<Renderable>(entity_id) {
                        if page_tag == "p" {
                            r.visible = true;
                        } else if page_tag == "min" {
                            r.visible = false;
                        }
                    }
                }
            }
            // Player control buttons
            else if let Some(ctrl) = world.get_component::<PlayerControl>(eid) {
                match ctrl.control_type {
                    ControlType::Close => {
                        // Close full player, show mini
                        let all_ids: Vec<u32> = world.entities.all_entities().iter().map(|e| e.id).collect();
                        for entity_id in all_ids {
                            let page_tag = match world.get_component::<Page>(entity_id) {
                                Some(Page(p)) => p.clone(),
                                _ => continue,
                            };
                            if let Some(mut r) = world.get_component_mut::<Renderable>(entity_id) {
                                if page_tag == "p" {
                                    r.visible = false;
                                } else if page_tag == "min" {
                                    r.visible = true;
                                }
                            }
                        }
                    }
                    ControlType::Play => {
                        for player_eid in world.query_with_mut::<Player>() {
                            if let Some(mut p) = world.get_component_mut::<Player>(player_eid) {
                                p.is_playing = !p.is_playing;
                            }
                        }
                        for mini_eid in world.query_with_mut::<MiniPlayer>() {
                            if let Some(mut m) = world.get_component_mut::<MiniPlayer>(mini_eid) {
                                m.is_playing = !m.is_playing;
                            }
                        }
                    }
                    ControlType::Like => {
                        for player_eid in world.query_with_mut::<Player>() {
                            if let Some(mut p) = world.get_component_mut::<Player>(player_eid) {
                                p.is_liked = !p.is_liked;
                            }
                        }
                    }
                    ControlType::Prev => {
                        for player_eid in world.query_with_mut::<Player>() {
                            if let Some(mut p) = world.get_component_mut::<Player>(player_eid) {
                                p.progress = 0.0;
                            }
                        }
                    }
                    ControlType::Next => {
                        for player_eid in world.query_with_mut::<Player>() {
                            if let Some(mut p) = world.get_component_mut::<Player>(player_eid) {
                                p.progress = 0.0;
                            }
                        }
                    }
                    ControlType::Shuffle | ControlType::Repeat => {
                        // No behavior yet
                    }
                }
            }
        }
        InputAction::Key(ch) => {
            if ch == '\x1b' {
                let all_ids: Vec<u32> = world.entities.all_entities().iter().map(|e| e.id).collect();
                for entity_id in all_ids {
                    let page_tag = match world.get_component::<Page>(entity_id) {
                        Some(Page(p)) => p.clone(),
                        _ => continue,
                    };
                    if let Some(mut r) = world.get_component_mut::<Renderable>(entity_id) {
                        if page_tag == "p" {
                            r.visible = false;
                        } else if page_tag == "min" {
                            r.visible = true;
                        }
                    }
                }
            }
        }
        _ => {}
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
