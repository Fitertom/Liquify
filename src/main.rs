#[cfg(not(target_os = "android"))]
use winit::event_loop::EventLoop;
#[cfg(not(target_os = "android"))]
use liquify::run_app;

#[cfg(not(target_os = "android"))]
fn main() {
    run_app(EventLoop::<()>::with_user_event());
}

#[cfg(target_os = "android")]
fn main() {
    // Android uses android_main from lib.rs
}
