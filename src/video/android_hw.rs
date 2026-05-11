use libc;
use std::time::Duration;
use crate::video::{VideoBackend, VideoFrame};
use ndk_sys::{
    AAssetManager_fromJava, AAssetManager_open, AAsset_openFileDescriptor,
    AAsset_close, AASSET_MODE_UNKNOWN,
    AMediaExtractor_new, AMediaExtractor_setDataSourceFd,
    AMediaExtractor_getTrackCount, AMediaExtractor_getTrackFormat,
    AMediaExtractor_selectTrack, AMediaExtractor_readSampleData,
    AMediaExtractor_advance, AMediaExtractor_getSampleTime,
    AMediaExtractor_seekTo,
    AMediaFormat_getString, AMediaFormat_getInt32, AMediaFormat_getInt64,
    AMEDIAFORMAT_KEY_MIME, AMEDIAFORMAT_KEY_WIDTH, AMEDIAFORMAT_KEY_HEIGHT, AMEDIAFORMAT_KEY_DURATION,
    AMediaCodec_createDecoderByType, AMediaCodec_configure, AMediaCodec_start,
    AMediaCodec_dequeueInputBuffer, AMediaCodec_getInputBuffer, AMediaCodec_queueInputBuffer,
    AMediaCodec_dequeueOutputBuffer, AMediaCodec_releaseOutputBuffer,
    AImageReader_new, AImageReader_getWindow, AImageReader_acquireNextImage, AImageReader_acquireLatestImage,
    AImage_getHardwareBuffer, AImage_delete,
    AMediaCodecBufferInfo
};
use std::ffi::CString;

pub struct AndroidHwBackend {
    extractor: *mut ndk_sys::AMediaExtractor,
    codec: *mut ndk_sys::AMediaCodec,
    reader: *mut ndk_sys::AImageReader,
    width: u32,
    height: u32,
    duration: Duration,
}

impl AndroidHwBackend {
    pub fn new(path: &str) -> Option<Self> {
        log::error!("Rust: [DEBUG_V4] Creating NDK MediaCodec for path: {}", path);
        let ctx = ndk_context::android_context();
        let vm_ptr = ctx.vm();
        let context_ptr = ctx.context();
        
        unsafe {
            let jvm = jni::JavaVM::from_raw(vm_ptr as *mut _).ok()?;
            let mut env = jvm.attach_current_thread().ok()?;
            let context_obj = jni::objects::JObject::from_raw(context_ptr as *mut _);
            
            // Получаем AssetManager
            let assets_obj = match env.call_method(&context_obj, "getAssets", "()Landroid/content/res/AssetManager;", &[]) {
                Ok(obj) => obj.l().unwrap(),
                Err(e) => { log::error!("Rust: [CRITICAL_ERROR] getAssets failed: {:?}", e); return None; }
            };
            
            let am_ptr = AAssetManager_fromJava(env.get_native_interface() as *mut _, assets_obj.as_raw() as *mut _);
            if am_ptr.is_null() { 
                log::error!("Rust: [CRITICAL_ERROR] AAssetManager_fromJava returned null");
                return None; 
            }
            
            let c_path = CString::new(path).unwrap();
            let asset = AAssetManager_open(am_ptr, c_path.as_ptr(), AASSET_MODE_UNKNOWN as i32);
            if asset.is_null() { 
                log::error!("Rust: [CRITICAL_ERROR] AAssetManager_open failed for path: {}", path);
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
            if AMediaExtractor_setDataSourceFd(extractor, fd, start, length) != ndk_sys::media_status_t(0) {
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
            
            log::error!("Rust: [DEBUG_V4] Video mime: {:?}, {}x{}", mime_ptr, width, height);

            let codec = AMediaCodec_createDecoderByType(mime_ptr);
            if codec.is_null() { 
                log::error!("Rust: [CRITICAL_ERROR] AMediaCodec_createDecoderByType returned null");
                return None; 
            }

            // Создаем AImageReader. Format 34 = AIMAGE_FORMAT_PRIVATE (или 35 = YUV_420_888).
            let mut reader: *mut ndk_sys::AImageReader = std::ptr::null_mut();
            if AImageReader_new(width, height, 34, 4, &mut reader) != ndk_sys::media_status_t(0) || reader.is_null() {
                log::error!("Rust: [CRITICAL_ERROR] AImageReader_new failed");
                return None;
            }

            let mut window: *mut ndk_sys::ANativeWindow = std::ptr::null_mut();
            if AImageReader_getWindow(reader, &mut window) != ndk_sys::media_status_t(0) || window.is_null() {
                log::error!("Rust: [CRITICAL_ERROR] AImageReader_getWindow failed");
                return None;
            }
            
            // Настраиваем кодек С Surface
            if AMediaCodec_configure(codec, format, window, std::ptr::null_mut(), 0) != ndk_sys::media_status_t(0) {
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
            })
        }
    }
}

impl VideoBackend for AndroidHwBackend {
    fn next_frame(&mut self) -> Option<VideoFrame> {
        unsafe {
            // Читаем в кодек
            let in_idx = AMediaCodec_dequeueInputBuffer(self.codec, 1000);
            if in_idx >= 0 {
                let mut buf_size: usize = 0;
                let buf = AMediaCodec_getInputBuffer(self.codec, in_idx as usize, &mut buf_size);
                if !buf.is_null() {
                    let sample_size = AMediaExtractor_readSampleData(self.extractor, buf, buf_size);
                    if sample_size < 0 {
                        AMediaExtractor_seekTo(self.extractor, 0, ndk_sys::SeekMode(0));
                    } else {
                        let pts = AMediaExtractor_getSampleTime(self.extractor);
                        AMediaCodec_queueInputBuffer(self.codec, in_idx as usize, 0, sample_size as usize, pts as u64, 0);
                        AMediaExtractor_advance(self.extractor);
                    }
                }
            }

            // Достаем готовый кадр
            let mut info: AMediaCodecBufferInfo = std::mem::zeroed();
            log::error!("Rust: [DEBUG] step: dequeue output");
            let out_idx = AMediaCodec_dequeueOutputBuffer(self.codec, &mut info, 1000);
            
            if out_idx >= 0 {
                log::error!("Rust: [DEBUG] step: release to surface");
                // Рендерим кадр на поверхность AImageReader
                AMediaCodec_releaseOutputBuffer(self.codec, out_idx as usize, true);
                
                // Даем немножко времени AImageReader на то, чтобы буфер дошел по цепочке
                // В идеале нужен AImageReader_ImageListener, но для poll mode достаточно небольшого ожидания
                std::thread::sleep(Duration::from_millis(1));
                
                log::error!("Rust: [DEBUG] step: acquire image");
                let mut image_ptr: *mut ndk_sys::AImage = std::ptr::null_mut();
                
                // Пытаемся забрать кадр
                let res = AImageReader_acquireNextImage(self.reader, &mut image_ptr);
                if res == ndk_sys::media_status_t(0) && !image_ptr.is_null() {
                    log::error!("Rust: [DEBUG] step: get hw buffer");
                    let mut hb_ptr: *mut ndk_sys::AHardwareBuffer = std::ptr::null_mut();
                    if AImage_getHardwareBuffer(image_ptr, &mut hb_ptr) == ndk_sys::media_status_t(0) && !hb_ptr.is_null() {
                        let mut desc: ndk_sys::AHardwareBuffer_Desc = std::mem::zeroed();
                        ndk_sys::AHardwareBuffer_describe(hb_ptr, &mut desc);
                        log::error!("Rust: [DEBUG] AHB format: {}", desc.format);
                        
                        log::error!("Rust: [DEBUG] step: vulkan import");
                        // Acquire дополнительную ссылку чтобы буфер остался жив после AImage_delete
                        ndk_sys::AHardwareBuffer_acquire(hb_ptr);
                        let frame = VideoFrame::HardwareBuffer(hb_ptr as *mut std::ffi::c_void);
                        
                        AImage_delete(image_ptr);
                        
                        return Some(frame);
                    }
                    AImage_delete(image_ptr);
                } else {
                    log::error!("Rust: [DEBUG] AImageReader_acquireNextImage failed: result={:?}", res);
                }
            }
        }
        None
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
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


