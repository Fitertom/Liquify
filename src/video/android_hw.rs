use crate::video::{VideoBackend, VideoFrame};
use libc;
use ndk_sys::{
    AASSET_MODE_UNKNOWN, AAsset_close, AAsset_openFileDescriptor, AAssetManager_fromJava,
    AAssetManager_open, AImage_delete, AImage_getHardwareBuffer, AImageReader_acquireLatestImage,
    AImageReader_acquireNextImage, AImageReader_getWindow, AImageReader_new,
    AMEDIAFORMAT_KEY_DURATION, AMEDIAFORMAT_KEY_HEIGHT, AMEDIAFORMAT_KEY_MIME,
    AMEDIAFORMAT_KEY_WIDTH, AMediaCodec_configure, AMediaCodec_createDecoderByType,
    AMediaCodec_dequeueInputBuffer, AMediaCodec_dequeueOutputBuffer, AMediaCodec_flush,
    AMediaCodec_getInputBuffer,
    AMediaCodec_queueInputBuffer, AMediaCodec_releaseOutputBuffer, AMediaCodec_start,
    AMediaCodecBufferInfo, AMediaExtractor_advance, AMediaExtractor_getSampleTime,
    AMediaExtractor_getTrackCount, AMediaExtractor_getTrackFormat, AMediaExtractor_new,
    AMediaExtractor_readSampleData, AMediaExtractor_seekTo, AMediaExtractor_selectTrack,
    AMediaExtractor_setDataSourceFd, AMediaFormat_getInt32, AMediaFormat_getInt64,
    AMediaFormat_getString,
};
use std::ffi::CString;
use std::time::Duration;

pub struct AndroidHwBackend {
    extractor: *mut ndk_sys::AMediaExtractor,
    codec: *mut ndk_sys::AMediaCodec,
    reader: *mut ndk_sys::AImageReader,
    width: u32,
    height: u32,
    duration: Duration,

    playback_start: Option<std::time::Instant>,
    first_pts_us: Option<i64>,
    current_hb: Option<*mut ndk_sys::AHardwareBuffer>,
    
    pending_out_idx: Option<isize>,
    pending_pts_us: i64,
    frame_on_surface: bool,
}

impl AndroidHwBackend {
    pub fn new(path: &str) -> Option<Self> {
        log::error!(
            "Rust: [DEBUG_V4] Creating NDK MediaCodec for path: {}",
            path
        );
        let ctx = ndk_context::android_context();
        let vm_ptr = ctx.vm();
        let context_ptr = ctx.context();

        unsafe {
            let jvm = jni::JavaVM::from_raw(vm_ptr as *mut _).ok()?;
            let mut env = jvm.attach_current_thread().ok()?;
            let context_obj = jni::objects::JObject::from_raw(context_ptr as *mut _);

            // Получаем AssetManager
            let assets_obj = match env.call_method(
                &context_obj,
                "getAssets",
                "()Landroid/content/res/AssetManager;",
                &[],
            ) {
                Ok(obj) => obj.l().unwrap(),
                Err(e) => {
                    log::error!("Rust: [CRITICAL_ERROR] getAssets failed: {:?}", e);
                    return None;
                }
            };

            let am_ptr = AAssetManager_fromJava(
                env.get_native_interface() as *mut _,
                assets_obj.as_raw() as *mut _,
            );
            if am_ptr.is_null() {
                log::error!("Rust: [CRITICAL_ERROR] AAssetManager_fromJava returned null");
                return None;
            }

            let c_path = CString::new(path).unwrap();
            let asset = AAssetManager_open(am_ptr, c_path.as_ptr(), AASSET_MODE_UNKNOWN as i32);
            if asset.is_null() {
                log::error!(
                    "Rust: [CRITICAL_ERROR] AAssetManager_open failed for path: {}",
                    path
                );
                return None;
            }

            let mut start: i64 = 0;
            let mut length: i64 = 0;
            let fd = AAsset_openFileDescriptor(asset, &mut start, &mut length);
            AAsset_close(asset);
            if fd < 0 {
                log::error!("Rust: [CRITICAL_ERROR] AAsset_openFileDescriptor failed (fd < 0)");
                return None;
            }

            let extractor = AMediaExtractor_new();
            if AMediaExtractor_setDataSourceFd(extractor, fd, start, length)
                != ndk_sys::media_status_t(0)
            {
                log::error!("Rust: [CRITICAL_ERROR] AMediaExtractor_setDataSourceFd failed");
                return None;
            }

            let num_tracks = AMediaExtractor_getTrackCount(extractor);
            let mut video_track_idx = -1;
            let mut format = std::ptr::null_mut();
            let mut mime_ptr: *const std::os::raw::c_char = std::ptr::null();

            for i in 0..num_tracks {
                format = AMediaExtractor_getTrackFormat(extractor, i);
                if !format.is_null() {
                    AMediaFormat_getString(format, AMEDIAFORMAT_KEY_MIME, &mut mime_ptr);
                    if !mime_ptr.is_null() {
                        let mime_str = std::ffi::CStr::from_ptr(mime_ptr).to_str().unwrap_or("");
                        if mime_str.starts_with("video/") {
                            video_track_idx = i as isize;
                            break;
                        }
                    }
                }
            }

            if video_track_idx < 0 {
                log::error!("Rust: [CRITICAL_ERROR] No video track found");
                return None;
            }
            AMediaExtractor_selectTrack(extractor, video_track_idx as usize);

            let mut width: i32 = 0;
            let mut height: i32 = 0;
            let mut duration_us: i64 = 0;
            AMediaFormat_getInt32(format, AMEDIAFORMAT_KEY_WIDTH, &mut width);
            AMediaFormat_getInt32(format, AMEDIAFORMAT_KEY_HEIGHT, &mut height);
            AMediaFormat_getInt64(format, AMEDIAFORMAT_KEY_DURATION, &mut duration_us);

            log::error!(
                "Rust: [DEBUG_V4] Video mime: {:?}, {}x{}",
                mime_ptr,
                width,
                height
            );

            let codec = AMediaCodec_createDecoderByType(mime_ptr);
            if codec.is_null() {
                log::error!("Rust: [CRITICAL_ERROR] AMediaCodec_createDecoderByType returned null");
                return None;
            }

            // Создаем AImageReader. Format 34 = AIMAGE_FORMAT_PRIVATE
            let mut reader: *mut ndk_sys::AImageReader = std::ptr::null_mut();
            if AImageReader_new(width, height, 34, 4, &mut reader) != ndk_sys::media_status_t(0)
                || reader.is_null()
            {
                log::error!("Rust: [CRITICAL_ERROR] AImageReader_new failed");
                return None;
            }

            let mut window: *mut ndk_sys::ANativeWindow = std::ptr::null_mut();
            if AImageReader_getWindow(reader, &mut window) != ndk_sys::media_status_t(0)
                || window.is_null()
            {
                log::error!("Rust: [CRITICAL_ERROR] AImageReader_getWindow failed");
                return None;
            }

            // Настраиваем кодек С Surface
            if AMediaCodec_configure(codec, format, window, std::ptr::null_mut(), 0)
                != ndk_sys::media_status_t(0)
            {
                log::error!("Rust: [CRITICAL_ERROR] AMediaCodec_configure failed");
                return None;
            }

            if AMediaCodec_start(codec) != ndk_sys::media_status_t(0) {
                log::error!("Rust: [CRITICAL_ERROR] AMediaCodec_start failed");
                return None;
            }

            log::error!("Rust: NDK Decoder init success: {}x{}", width, height);

            Some(Self {
                extractor,
                codec,
                reader,
                width: width as u32,
                height: height as u32,
                duration: Duration::from_micros(duration_us as u64),
                playback_start: None,
                first_pts_us: None,
                current_hb: None,
                pending_out_idx: None,
                pending_pts_us: 0,
                frame_on_surface: false,
            })
        }
    }

    unsafe fn try_acquire_from_reader(&mut self) -> Option<VideoFrame> {
        let mut image_ptr: *mut ndk_sys::AImage = std::ptr::null_mut();
        let res = unsafe { ndk_sys::AImageReader_acquireNextImage(self.reader, &mut image_ptr) };
        
        if res == ndk_sys::media_status_t(0) && !image_ptr.is_null() {
            let mut hb_ptr: *mut ndk_sys::AHardwareBuffer = std::ptr::null_mut();
            if unsafe { ndk_sys::AImage_getHardwareBuffer(image_ptr, &mut hb_ptr) } == ndk_sys::media_status_t(0) && !hb_ptr.is_null() {
                
                if let Some(old_hb) = self.current_hb.take() {
                    unsafe { ndk_sys::AHardwareBuffer_release(old_hb) };
                }

                unsafe { ndk_sys::AHardwareBuffer_acquire(hb_ptr) };
                self.current_hb = Some(hb_ptr);
                
                let frame = VideoFrame::HardwareBuffer(hb_ptr as *mut std::ffi::c_void);
                unsafe { ndk_sys::AImage_delete(image_ptr) };
                return Some(frame);
            }
            unsafe { ndk_sys::AImage_delete(image_ptr) };
        }
        None
    }
}

impl Drop for AndroidHwBackend {
    fn drop(&mut self) {
        unsafe {
            if let Some(hb) = self.current_hb.take() {
                ndk_sys::AHardwareBuffer_release(hb);
            }
            if !self.codec.is_null() {
                ndk_sys::AMediaCodec_stop(self.codec);
                ndk_sys::AMediaCodec_delete(self.codec);
            }
            if !self.extractor.is_null() {
                ndk_sys::AMediaExtractor_delete(self.extractor);
            }
            if !self.reader.is_null() {
                ndk_sys::AImageReader_delete(self.reader);
            }
        }
    }
}

impl VideoBackend for AndroidHwBackend {
    fn next_frame(&mut self) -> Option<VideoFrame> {
        unsafe {
            // 1. Проверяем поверхность
            if self.frame_on_surface {
                if let Some(frame) = self.try_acquire_from_reader() {
                    self.frame_on_surface = false;
                    return Some(frame);
                }
                return None;
            }

            // 2. Подаем данные
            let in_idx = AMediaCodec_dequeueInputBuffer(self.codec, 0);
            if in_idx >= 0 {
                let mut buf_size: usize = 0;
                let buf = AMediaCodec_getInputBuffer(self.codec, in_idx as usize, &mut buf_size);
                if !buf.is_null() {
                    let sample_size = AMediaExtractor_readSampleData(self.extractor, buf, buf_size);
                    if sample_size < 0 {
                        AMediaExtractor_seekTo(self.extractor, 0, ndk_sys::SeekMode(0));
                        if let Some(idx) = self.pending_out_idx.take() {
                            AMediaCodec_releaseOutputBuffer(self.codec, idx as usize, false);
                        }
                        AMediaCodec_flush(self.codec);
                        self.playback_start = None;
                        self.first_pts_us = None;
                        self.frame_on_surface = false;
                        log::error!("Liquify: Video looped and flushed.");
                        return None;
                    } else {
                        let pts = AMediaExtractor_getSampleTime(self.extractor);
                        AMediaCodec_queueInputBuffer(self.codec, in_idx as usize, 0, sample_size as usize, pts as u64, 0);
                        AMediaExtractor_advance(self.extractor);
                    }
                }
            }

            // 3. Получаем кадр
            let mut out_idx = self.pending_out_idx.take().unwrap_or(-1);
            let mut pts_us = self.pending_pts_us;

            if out_idx < 0 {
                let mut info: AMediaCodecBufferInfo = std::mem::zeroed();
                out_idx = AMediaCodec_dequeueOutputBuffer(self.codec, &mut info, 0);
                if out_idx >= 0 {
                    pts_us = info.presentationTimeUs;
                }
            }

            if out_idx >= 0 {
                if self.first_pts_us.is_none() {
                    self.playback_start = Some(std::time::Instant::now());
                    self.first_pts_us = Some(pts_us);
                }

                let elapsed = self.playback_start.unwrap().elapsed();
                let video_elapsed = Duration::from_micros((pts_us - self.first_pts_us.unwrap()).max(0) as u64);

                if video_elapsed > elapsed {
                    self.pending_out_idx = Some(out_idx);
                    self.pending_pts_us = pts_us;
                    return None;
                }

                AMediaCodec_releaseOutputBuffer(self.codec, out_idx as usize, true);
                self.frame_on_surface = true;

                if let Some(frame) = self.try_acquire_from_reader() {
                    self.frame_on_surface = false;
                    return Some(frame);
                }
            }
        }
        None
    }

    fn duration(&self) -> Duration { self.duration }
    fn dimensions(&self) -> (u32, u32) { (self.width, self.height) }
}

pub unsafe fn import_android_buffer(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    ptr: *mut std::ffi::c_void,
    width: u32,
    height: u32,
) -> Option<wgpu::Texture> {
    unsafe { super::vulkan_import::import_android_buffer(device, _queue, ptr, width, height) }
}
