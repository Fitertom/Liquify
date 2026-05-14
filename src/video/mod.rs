use std::time::Duration;

#[cfg(not(target_os = "android"))]
pub mod desktop;
#[cfg(target_os = "android")]
pub mod android_hw;
#[cfg(target_os = "android")]
pub mod vulkan_import;
#[cfg(target_os = "android")]
pub mod raw_vulkan_ycbcr;
#[cfg(target_os = "android")]
pub mod ycbcr_pipeline;

pub enum VideoFrame {
    Rgba(Vec<u8>, u32, u32),
    #[cfg(target_os = "android")]
    HardwareBuffer(*mut std::ffi::c_void), // AHardwareBuffer*
}

pub trait VideoBackend {
    fn next_frame(&mut self) -> Option<VideoFrame>;
    fn duration(&self) -> Duration;
    fn dimensions(&self) -> (u32, u32);
}

pub struct VideoPlayer {
    backend: Box<dyn VideoBackend>,
}

impl VideoPlayer {
    pub fn new(path: &str) -> Option<Self> {
        #[cfg(not(target_os = "android"))]
        {
            desktop::DesktopBackend::new(path).map(|b| Self { backend: Box::new(b) })
        }
        #[cfg(target_os = "android")]
        {
            android_hw::AndroidHwBackend::new(path).map(|b| Self { backend: Box::new(b) })
        }
    }

    pub fn next_frame(&mut self) -> Option<VideoFrame> {
        self.backend.next_frame()
    }

    pub fn duration(&self) -> Duration {
        self.backend.duration()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.backend.dimensions()
    }
}
