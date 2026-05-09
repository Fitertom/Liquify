pub struct CardLayout {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub title: String,
    pub subtitle: Option<String>,
    pub color: [f32; 3],
    pub icon_type: String, // "heart", "play", "music", etc.
}

pub struct SectionLayout {
    pub title: String,
    pub cards: Vec<CardLayout>,
}

pub struct UILayout {
    pub quick_grid: Vec<CardLayout>,
    pub new_release: Option<CardLayout>,
    pub recommended: SectionLayout,
    pub mixes: SectionLayout,
    pub win_w: f32,
    pub win_h: f32,
}

impl UILayout {
    pub fn new() -> Self {
        UILayout {
            quick_grid: Vec::new(),
            new_release: None,
            recommended: SectionLayout { title: "Рекомендуемые".to_string(), cards: Vec::new() },
            mixes: SectionLayout { title: "Для вас".to_string(), cards: Vec::new() },
            win_w: 0.0,
            win_h: 0.0,
        }
    }

    pub fn compute(&mut self, win_w: f32, win_h: f32) {
        self.win_w = win_w;
        self.win_h = win_h;

        let margin = 18.0;
        let mut y = 60.0; // Header height approx

        // Greeting
        y += 40.0; // Skip greeting for now, just layout

        // Quick Grid (2 columns)
        let grid_gap = 8.0;
        let card_w = (win_w - margin * 2.0 - grid_gap) / 2.0;
        let card_h = 56.0;

        self.quick_grid.clear();
        let titles = [
            ("Любимые треки", [0.65, 0.55, 0.98], "heart"),
            ("Недавно играло", [0.96, 0.62, 0.04], "play"),
            ("Все треки", [0.94, 0.27, 0.27], "music"),
            ("Моя медиатека", [0.06, 0.73, 0.51], "image"),
            ("Поиск треков", [0.23, 0.51, 0.96], "search"),
            ("Плейлисты", [0.93, 0.28, 0.6], "list"),
        ];

        for (i, (title, color, icon)) in titles.iter().enumerate() {
            let row = i / 2;
            let col = i % 2;
            self.quick_grid.push(CardLayout {
                x: margin + col as f32 * (card_w + grid_gap),
                y: y + row as f32 * (card_h + grid_gap),
                w: card_w,
                h: card_h,
                title: title.to_string(),
                subtitle: None,
                color: *color,
                icon_type: icon.to_string(),
            });
        }
        y += 3.0 * (card_h + grid_gap) + 20.0;

        // New Release
        self.new_release = Some(CardLayout {
            x: margin,
            y,
            w: win_w - margin * 2.0,
            h: 130.0,
            title: "Недавно добавлено".to_string(),
            subtitle: Some("Сингл · Выберите трек".to_string()),
            color: [0.65, 0.55, 0.98],
            icon_type: "music".to_string(),
        });
        y += 130.0 + 30.0;

        // Recommended (Horizontal scroll simulation)
        self.recommended.cards.clear();
        let rec_titles = ["Микс дня", "Похожее", "Энергичное"];
        for (i, title) in rec_titles.iter().enumerate() {
            self.recommended.cards.push(CardLayout {
                x: margin + i as f32 * 162.0,
                y: y + 30.0, // After title
                w: 150.0,
                h: 200.0,
                title: title.to_string(),
                subtitle: None,
                color: [0.4, 0.4, 0.4],
                icon_type: "radio".to_string(),
            });
        }
        y += 240.0;

        // Mixes
        self.mixes.cards.clear();
        let mix_titles = ["Микс дня 1", "Открытия недели", "Ежедневный микс"];
        for (i, title) in mix_titles.iter().enumerate() {
            self.mixes.cards.push(CardLayout {
                x: margin + i as f32 * 167.0,
                y: y + 30.0,
                w: 155.0,
                h: 220.0,
                title: title.to_string(),
                subtitle: Some("Основано на вашем вкусе".to_string()),
                color: [0.5, 0.5, 0.5],
                icon_type: "headset".to_string(),
            });
        }
    }

    pub fn hit_test(&self, _mx: f32, _my: f32) -> Option<usize> {
        None // Placeholder for now
    }
}
