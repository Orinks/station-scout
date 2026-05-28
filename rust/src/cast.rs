use crate::models::Station;
use url::Url;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CastDevice {
    pub name: String,
    pub uuid: String,
}

pub fn station_content_type(station: &Station, url: &str) -> &'static str {
    let codec = station.codec.trim().to_lowercase();
    let path = Url::parse(url)
        .ok()
        .map(|parsed| parsed.path().to_lowercase())
        .unwrap_or_else(|| url.to_lowercase());
    if station.hls != 0 || path.ends_with(".m3u8") || path.ends_with(".m3u") {
        return "application/x-mpegURL";
    }
    if matches!(codec.as_str(), "mp3" | "mpeg") || path.ends_with(".mp3") {
        return "audio/mpeg";
    }
    if matches!(codec.as_str(), "aac" | "aac+" | "he-aac")
        || path.ends_with(".aac")
        || path.ends_with(".m4a")
    {
        return "audio/aac";
    }
    if codec == "ogg" || path.ends_with(".ogg") {
        return "audio/ogg";
    }
    if codec == "opus" || path.ends_with(".opus") {
        return "audio/opus";
    }
    if codec == "flac" || path.ends_with(".flac") {
        return "audio/flac";
    }
    "audio/mpeg"
}
