use std::any::TypeId;

#[derive(Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[derive(Clone)]
pub struct UICard {
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: String,
    pub tint: [f32; 3],
    pub is_hovered: bool,
}

#[derive(Clone)]
pub struct UIHeader {
    pub title: String,
    pub greeting: String,
}

#[derive(Clone)]
pub struct UISection {
    pub title: String,
}

#[derive(Clone, Copy)]
pub struct Renderable {
    pub visible: bool,
}

#[derive(Clone, PartialEq)]
pub struct Page(pub String);

#[derive(Clone)]
pub struct UINavBar {
    pub active_tab: String,
}

#[derive(Clone)]
pub struct UINavButton {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct UIImage {
    pub path: String,
}

// Player Components
#[derive(Clone)]
pub struct Player {
    pub title: String,
    pub artist: String,
    pub progress: f32, // 0.0 to 1.0
    pub duration: f32, // in seconds
    pub is_playing: bool,
    pub is_liked: bool,
}

#[derive(Clone)]
pub struct MiniPlayer {
    pub title: String,
    pub artist: String,
    pub progress: f32,
    pub is_playing: bool,
}

#[derive(Clone)]
pub struct PlayerControl {
    pub control_type: ControlType,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ControlType {
    Prev,
    Play,
    Next,
    Shuffle,
    Repeat,
    Like,
    Close,
}

#[derive(Clone)]
pub struct ProgressBar {
    pub value: f32,
    pub max: f32,
}

pub fn type_id_of<T: 'static>() -> TypeId {
    TypeId::of::<T>()
}
