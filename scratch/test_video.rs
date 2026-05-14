use openh264::decoder::Decoder;
use mp4::Mp4Reader;
use std::io::BufReader;
use std::fs::File;

fn main() {
    let f = File::open("background.mp4");
    if f.is_err() {
        println!("background.mp4 not found");
        return;
    }
    let f = f.unwrap();
    let reader = BufReader::new(f);
    let mp4 = Mp4Reader::read_header(reader, 0).expect("Failed to read MP4 header");
    
    let mut video_track_id = 0;
    let mut found = false;
    for track in mp4.tracks().values() {
        if track.track_type().unwrap() == mp4::TrackType::Video {
            video_track_id = track.track_id();
            found = true;
            break;
        }
    }
    
    if !found {
        println!("No video track found");
        return;
    }

    let mut decoder = Decoder::new().expect("Failed to create decoder");
    println!("MP4 and Decoder initialized successfully for track {}", video_track_id);
}
