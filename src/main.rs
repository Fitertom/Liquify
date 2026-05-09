mod ecs;
mod calculator;
mod font;
mod render;
mod ui;
mod input;

use render::RenderPipeline;
use input::{InputState, InputAction};
use ecs::systems;

use std::io::{self, Write};
use std::time::Instant;

use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent, ElementState, MouseButton, TouchPhase},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder},
    window::Window,
};

#[cfg(target_os = "android")]
use winit::platform::android::{activity::AndroidApp, EventLoopBuilderExtAndroid};

#[cfg(target_os = "android")]
fn vibrate_android(app: &AndroidApp, ms: i64) {
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() { return; }
    
    let vm = unsafe { jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) };
    let vm = match vm {
        Ok(vm) => vm,
        Err(_) => return,
    };

    let mut env = match vm.attach_current_thread() {
        Ok(env) => env,
        Err(_) => return,
    };

    let activity_ptr = app.activity_as_ptr();
    if activity_ptr.is_null() { return; }
    let activity = unsafe { jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject) };

    if let Ok(vibrator_name) = env.new_string("vibrator") {
        if let Ok(vibrator) = env.call_method(
            &activity,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[jni::objects::JValue::Object(vibrator_name.as_ref())],
        ) {
            if let Ok(vibrator_obj) = vibrator.l() {
                let _ = env.call_method(
                    vibrator_obj,
                    "vibrate",
                    "(J)V",
                    &[jni::objects::JValue::Long(ms)],
                );
            }
        }
    }
}

struct AppState {
    renderer: RenderPipeline,
    world: ecs::World,
    input: InputState,
    window_size: (f32, f32),
    scale_factor: f32,
    fps_frames: u32,
    fps_start: Instant,
    fps_text: String,
    last_frame_time: Instant,
    #[cfg(target_os = "android")]
    android_app: AndroidApp,
}

impl AppState {
    async fn new(window: &Window, #[cfg(target_os = "android")] android_app: AndroidApp) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        let font_data = include_bytes!("../fonts/PlusJakartaSans-Bold.ttf");
        let renderer = RenderPipeline::new(window, font_data).await;

        let mut world = ecs::World::new();
        systems::system_ui_init(&mut world);
        systems::system_layout(&mut world, size.width as f32, size.height as f32, scale_factor);

        AppState {
            renderer,
            world,
            input: InputState::new(),
            window_size: (size.width as f32, size.height as f32),
            scale_factor,
            fps_frames: 0,
            fps_start: Instant::now(),
            fps_text: String::from("0 FPS"),
            last_frame_time: Instant::now(),
            #[cfg(target_os = "android")]
            android_app,
        }
    }

    fn on_resize(&mut self, width: u32, height: u32, scale_factor: f32) {
        self.window_size = (width as f32, height as f32);
        self.scale_factor = scale_factor;
        self.renderer.resize(width, height);
        systems::system_layout(&mut self.world, width as f32, height as f32, scale_factor);
    }

    fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.input.on_mouse_move(x, y);
        // systems::system_input_ui_hover(&mut self.world, &mut self.input);
    }

    fn on_mouse_press(&mut self) {
        self.input.on_mouse_press();
        if let Some(idx) = self.input.hovered_button {
            self.input.last_action = InputAction::ButtonPress(idx);
            #[cfg(target_os = "android")]
            vibrate_android(&self.android_app, 10);
        }
    }

    fn on_mouse_release(&mut self) {
        self.input.on_mouse_release();
    }

    fn on_key(&mut self, ch: char) {
        self.input.last_action = InputAction::Key(ch);
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        self.fps_frames += 1;
        let elapsed = self.fps_start.elapsed().as_secs_f32();
        if elapsed >= 1.0 {
            let fps = (self.fps_frames as f32 / elapsed) as u32;
            self.fps_text = format!("{fps} FPS");
            let _ = writeln!(io::stdout(), "{fps} FPS");
            self.fps_frames = 0;
            self.fps_start = Instant::now();
        }

        // Update scroll physics (inertia + rubber band)
        self.input.scroll.update(dt.min(0.05), self.window_size.1);

        systems::system_render(
            &mut self.world,
            &mut self.renderer,
            &self.input,
            &self.fps_text,
            self.window_size.0,
            self.window_size.1,
            self.scale_factor,
        );
        self.input.reset_action();
    }
}

#[cfg(not(target_os = "android"))]
fn main() {
    #[cfg(target_os = "android")]
    run_app(EventLoop::<()>::with_user_event(), None);
    #[cfg(not(target_os = "android"))]
    run_app(EventLoop::<()>::with_user_event());
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: AndroidApp) {
    let mut event_loop_builder = EventLoop::<()>::with_user_event();
    let app_clone = app.clone();
    event_loop_builder.with_android_app(app);
    run_app(event_loop_builder, Some(app_clone));
}

fn run_app(mut event_loop_builder: EventLoopBuilder<()>, #[cfg(target_os = "android")] android_app: Option<AndroidApp>) {
    let event_loop = event_loop_builder.build().unwrap();

    let mut window: Option<Window> = None;
    let mut state: Option<AppState> = None;

    #[allow(deprecated)]
    event_loop
        .run(move |event, elwt: &ActiveEventLoop| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::Resumed => {
                    if window.is_none() {
                        let attrs = Window::default_attributes()
                            .with_title("Calculator")
                            .with_inner_size(PhysicalSize::new(380, 520));
                        let created_window = elwt.create_window(attrs).unwrap();
                        #[cfg(target_os = "android")]
                        let s = pollster::block_on(AppState::new(&created_window, android_app.clone().unwrap()));
                        #[cfg(not(target_os = "android"))]
                        let s = pollster::block_on(AppState::new(&created_window));
                        state = Some(s);
                        created_window.request_redraw();
                        window = Some(created_window);
                    } else if let Some(window) = window.as_ref() {
                        window.request_redraw();
                    }
                }
                Event::Suspended => {
                    state = None;
                    window = None;
                }
                Event::WindowEvent { event: window_event, .. } => match window_event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::Resized(size) => {
                        if let (Some(ref mut s), Some(w)) = (state.as_mut(), window.as_ref()) {
                            s.on_resize(size.width, size.height, w.scale_factor() as f32);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        if let Some(ref mut s) = state {
                            s.frame();
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(ref mut s) = state {
                            s.on_mouse_move(position.x as f32, position.y as f32);
                            if let Some(window) = window.as_ref() {
                                window.request_redraw();
                            }
                        }
                    }
                    WindowEvent::MouseInput { button, state: elem_state, .. } => {
                        if elem_state == ElementState::Pressed
                            && button == MouseButton::Left
                        {
                            if let Some(ref mut s) = state {
                                s.on_mouse_press();
                                if let Some(window) = window.as_ref() {
                                    window.request_redraw();
                                }
                            }
                        }
                    }
                    WindowEvent::KeyboardInput { event: kb_event, .. } => {
                        if let Some(ref mut s) = state {
                            if kb_event.state == ElementState::Pressed {
                                if let Some(ref text) = kb_event.text {
                                    for ch in text.as_str().chars() {
                                        s.on_key(ch);
                                    }
                                }
                            }
                        }
                    }
                    WindowEvent::Touch(touch) => {
                        if let Some(ref mut s) = state {
                            let (tx, ty) = (touch.location.x as f32, touch.location.y as f32);
                            let time = s.fps_start.elapsed().as_secs_f64() + s.fps_frames as f64 * 0.001;
                            let time_abs = std::time::UNIX_EPOCH.elapsed().unwrap_or_default().as_secs_f64();
                            match touch.phase {
                                TouchPhase::Started => {
                                    s.on_mouse_move(tx, ty);
                                    s.input.scroll.on_touch_start(ty, time_abs);
                                    if let Some(window) = window.as_ref() {
                                        window.request_redraw();
                                    }
                                }
                                TouchPhase::Moved => {
                                    s.on_mouse_move(tx, ty);
                                    s.input.scroll.on_touch_move(ty, time_abs);
                                    if let Some(window) = window.as_ref() {
                                        window.request_redraw();
                                    }
                                }
                                TouchPhase::Ended | TouchPhase::Cancelled => {
                                    s.input.scroll.on_touch_end();
                                    s.on_mouse_release();
                                    s.on_mouse_move(-1000.0, -1000.0);
                                    if let Some(window) = window.as_ref() {
                                        window.request_redraw();
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                },
                Event::AboutToWait => {
                    if let Some(window) = window.as_ref() {
                        window.request_redraw();
                    }
                }
                _ => {}
            }
        })
        .unwrap();
}
