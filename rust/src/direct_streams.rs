use crate::models::Station;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use url::Url;

const STREAMURL_BASE: &str = "https://streamurl.link";

pub fn streamurl_search_url(query: &str) -> String {
    format!(
        "{}/s/{}/",
        STREAMURL_BASE.trim_end_matches('/'),
        utf8_percent_encode(query.trim(), NON_ALPHANUMERIC)
    )
}

pub fn station_from_direct_url(name: &str, url: &str, homepage: &str) -> Result<Station, String> {
    let clean_url = url.trim();
    if !is_stream_url_candidate(clean_url) {
        return Err("Enter a direct stream URL beginning with http, https, or icy.".to_string());
    }
    Ok(Station::direct_stream(
        if name.trim().is_empty() {
            clean_url
        } else {
            name.trim()
        },
        clean_url,
        homepage.trim(),
        "Direct stream URL",
    ))
}

pub fn is_stream_url_candidate(value: &str) -> bool {
    let Ok(parsed) = Url::parse(value.trim()) else {
        return false;
    };
    matches!(parsed.scheme(), "http" | "https" | "icy") && parsed.host_str().is_some()
}
