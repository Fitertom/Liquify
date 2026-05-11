use std::fs::File;
use std::io::BufReader;
use mp4::Mp4Reader;
use openh264::decoder::Decoder;
use openh264::formats::YUVSource;
use super::{VideoBackend, VideoFrame};
use std::time::Duration;

pub struct DesktopBackend {
    decoder: Decoder,
    mp4_reader: Mp4Reader<BufReader<File>>,
    video_track_id: u32,
    current_sample: u32,
    sample_count: u32,
    width: u32,
    height: u32,
    frame_duration: Duration,
}

impl DesktopBackend {
    pub fn new(path: &str) -> Option<Self> {
        let f = File::open(path).ok()?;
        let size = f.metadata().ok()?.len();
        let reader = BufReader::new(f);
        let mp4 = Mp4Reader::read_header(reader, size).ok()?;
        
        let mut video_track_id = 0;
        let mut track_info = None;
        for track in mp4.tracks().values() {
            if let Ok(track_type) = track.track_type() {
                if track_type == mp4::TrackType::Video {
                    video_track_id = track.track_id();
                    track_info = Some((track.sample_count(), track.width() as u32, track.height() as u32, track.timescale(), track.duration()));
                    break;
                }
            }
        }
        
        let (sample_count, width, height, _timescale, _duration_raw) = track_info?;
        let frame_duration = Duration::from_secs_f64(1.0 / 30.0);

        Some(Self {
            decoder: Decoder::new().ok()?,
            mp4_reader: mp4,
            video_track_id,
            current_sample: 1,
            sample_count,
            width,
            height,
            frame_duration,
        })
    }
}

impl VideoBackend for DesktopBackend {
    fn next_frame(&mut self) -> Option<VideoFrame> {
        for _ in 0..10 {
            if self.current_sample > self.sample_count {
                self.current_sample = 1;
            }

            let sample = self.mp4_reader.read_sample(self.video_track_id, self.current_sample).ok()??;
            self.current_sample += 1;

            let mut annex_b = Vec::with_capacity(sample.bytes.len() + 32);
            let mut pos = 0;
            while pos + 4 <= sample.bytes.len() {
                let len = u32::from_be_bytes([
                    sample.bytes[pos],
                    sample.bytes[pos + 1],
                    sample.bytes[pos + 2],
                    sample.bytes[pos + 3],
                ]) as usize;
                pos += 4;
                if pos + len > sample.bytes.len() { break; }
                annex_b.extend_from_slice(&[0, 0, 0, 1]);
                annex_b.extend_from_slice(&sample.bytes[pos..pos + len]);
                pos += len;
            }

            let input = if annex_b.is_empty() { &sample.bytes[..] } else { &annex_b[..] };

            if let Ok(Some(frame)) = self.decoder.decode(input) {
                let (w_u, h_u) = frame.dimensions();
                let (w, h) = (w_u as u32, h_u as u32);
                let mut rgba = vec![0u8; (w * h * 4) as usize];
                let mut rgb = vec![0u8; (w * h * 3) as usize];
                frame.write_rgb8(&mut rgb);
                for i in 0..(w * h) as usize {
                    rgba[i * 4] = rgb[i * 3];
                    rgba[i * 4 + 1] = rgb[i * 3 + 1];
                    rgba[i * 4 + 2] = rgb[i * 3 + 2];
                    rgba[i * 4 + 3] = 255;
                }
                return Some(VideoFrame::Rgba(rgba, w, h));
            }
        }
        None
    }

    fn duration(&self) -> Duration {
        Duration::from_secs(10) // Fallback or implement properly
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
