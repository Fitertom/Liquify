fn main() {
    let _extractor = ndk::media::media_extractor::MediaExtractor::new();
    let _codec = ndk::media::media_codec::MediaCodec::from_decoder_type("video/avc");
    let _reader = ndk::media::image_reader::ImageReader::new(1920, 1080, ndk::media::image_reader::ImageFormat::RGBA_8888, 3);
}
