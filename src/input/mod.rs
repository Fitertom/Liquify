#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputAction {
    None,
    ButtonPress(usize),
    Key(char),
}

pub struct ScrollState {
    /// Current scroll offset in pixels (positive = scrolled down)
    pub offset: f32,
    /// Smoothed offset for rendering (interpolates between touch events)
    pub render_offset: f32,
    /// Current velocity in px/sec for inertia
    pub velocity: f32,
    /// Whether a touch drag is currently happening
    pub is_dragging: bool,
    /// Touch Y when drag started
    touch_start_y: f32,
    /// Scroll offset when drag started
    offset_at_start: f32,
    /// Last touch Y for velocity calculation
    last_touch_y: f32,
    /// Timestamp of last touch move (seconds)
    last_touch_time: f64,
    /// Velocity samples for smooth inertia
    velocity_samples: [f32; 5],
    velocity_sample_idx: usize,
    /// Max scroll extent
    pub content_height: f32,
}

impl ScrollState {
    pub fn set_content_height(&mut self, height: f32) {
        self.content_height = height;
    }
    pub fn new() -> Self {
        ScrollState {
            offset: 0.0,
            render_offset: 0.0,
            velocity: 0.0,
            is_dragging: false,
            touch_start_y: 0.0,
            offset_at_start: 0.0,
            last_touch_y: 0.0,
            last_touch_time: 0.0,
            velocity_samples: [0.0; 5],
            velocity_sample_idx: 0,
            content_height: 2000.0,
        }
    }

    pub fn on_touch_start(&mut self, y: f32, time: f64) {
        self.is_dragging = true;
        self.touch_start_y = y;
        self.offset_at_start = self.render_offset; // Use rendered position as base
        self.last_touch_y = y;
        self.last_touch_time = time;
        self.velocity = 0.0;
        self.velocity_samples = [0.0; 5];
        self.velocity_sample_idx = 0;
    }

    pub fn on_touch_move(&mut self, y: f32, time: f64, view_height: f32) {
        if !self.is_dragging {
            return;
        }
        let delta = self.last_touch_y - y; // positive delta = scroll down
        let dt = (time - self.last_touch_time) as f32;

        if dt > 0.001 {
            let instant_vel = delta / dt;
            self.velocity_samples[self.velocity_sample_idx % 5] = instant_vel;
            self.velocity_sample_idx += 1;
        }

        self.offset = self.offset_at_start + (self.touch_start_y - y);
        self.clamp_with_rubber_band(view_height);

        self.last_touch_y = y;
        self.last_touch_time = time;
    }

    pub fn on_touch_end(&mut self) {
        self.is_dragging = false;
        // Snap render_offset to final touch position
        self.offset = self.render_offset;
        // Average velocity from samples for smooth inertia
        let count = self.velocity_sample_idx.min(5);
        if count > 0 {
            let sum: f32 = self.velocity_samples[..count].iter().sum();
            self.velocity = sum / count as f32;
        } else {
            self.velocity = 0.0;
        }
        // Cap velocity
        self.velocity = self.velocity.clamp(-8000.0, 8000.0);
    }

    /// Called every frame with dt in seconds
    pub fn update(&mut self, dt: f32, view_height: f32) {
        if self.is_dragging {
            // Smooth interpolation: render_offset chases offset
            // Using exponential smoothing for butter-smooth scrolling
            // Factor ~120 means ~120Hz effective smoothing rate
            let t = 1.0 - (-dt * 120.0_f32).exp();
            self.render_offset += (self.offset - self.render_offset) * t;
            return;
        }

        let max_scroll = (self.content_height - view_height).max(0.0);

        // Rubber band bounce back
        if self.offset < 0.0 {
            let spring = -self.offset * 12.0;
            self.velocity += spring * dt;
            self.velocity *= 0.85_f32.powf(dt * 60.0);
        } else if self.offset > max_scroll {
            let overshoot = self.offset - max_scroll;
            let spring = -overshoot * 12.0;
            self.velocity += spring * dt;
            self.velocity *= 0.85_f32.powf(dt * 60.0);
        } else {
            // Normal deceleration (friction)
            self.velocity *= 0.97_f32.powf(dt * 60.0);
        }

        self.offset += self.velocity * dt;
        // During inertia, render_offset tracks offset tightly
        self.render_offset = self.offset;

        // Stop when velocity is negligible
        if self.velocity.abs() < 0.5 && self.offset >= 0.0 && self.offset <= max_scroll {
            self.velocity = 0.0;
        }
    }

    fn clamp_with_rubber_band(&mut self, view_height: f32) {
        let max_rubber = 60.0;
        let max_scroll = (self.content_height - view_height).max(0.0);
        
        if self.offset < -max_rubber {
            self.offset = -max_rubber;
        } else if self.offset > max_scroll + max_rubber {
            self.offset = max_scroll + max_rubber;
        }
    }
}

pub struct InputState {
    pub mouse_pos: (f32, f32),
    pub mouse_pressed: bool,
    pub last_action: InputAction,
    pub hovered_button: Option<usize>,
    pub scroll: ScrollState,
}

impl InputState {
    pub fn new() -> Self {
        InputState {
            mouse_pos: (0.0, 0.0),
            mouse_pressed: false,
            last_action: InputAction::None,
            hovered_button: None,
            scroll: ScrollState::new(),
        }
    }

    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.mouse_pos = (x, y);
    }

    pub fn on_mouse_press(&mut self) {
        self.mouse_pressed = true;
    }

    pub fn on_mouse_release(&mut self) {
        self.mouse_pressed = false;
    }

    pub fn reset_action(&mut self) {
        self.last_action = InputAction::None;
    }
}
