use crate::models::Station;
use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::HashSet;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const INVALID_TITLE_MARKERS: &[&str] = &[
    "advert",
    "commercial",
    "promo",
    "sweeper",
    "jingle",
    "station id",
    "unknown",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackEntry {
    pub artist: String,
    pub title: String,
    pub raw: String,
    pub timestamp: DateTime<Utc>,
    pub uncertain: bool,
}

impl TrackEntry {
    pub fn new(artist: &str, title: &str, raw: &str) -> Self {
        Self {
            artist: artist.to_string(),
            title: title.to_string(),
            raw: raw.to_string(),
            timestamp: Utc::now(),
            uncertain: false,
        }
    }

    pub fn display_line(&self) -> String {
        let line = if !self.artist.is_empty() && !self.title.is_empty() {
            format!("{} - {}", self.artist, self.title)
        } else {
            self.raw.clone()
        };
        if self.uncertain {
            format!("{line} ?")
        } else {
            line
        }
    }
}

pub fn parse_stream_title(raw_title: &str, now: Option<DateTime<Utc>>) -> Option<TrackEntry> {
    let raw = clean(raw_title);
    if raw.is_empty() || is_bare_station_identifier(&raw) {
        return None;
    }
    let lowered = raw.to_lowercase();
    let mut uncertain = INVALID_TITLE_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker));
    let song_spot = metadata_attribute(&raw, "song_spot");
    if matches!(
        song_spot.as_deref().map(str::to_uppercase).as_deref(),
        Some("T")
    ) {
        uncertain = true;
    }

    let mut artist = String::new();
    let mut title = String::new();
    if let (Some(metadata_artist), Some(metadata_title)) = (
        metadata_attribute(&raw, "artist"),
        metadata_attribute(&raw, "title"),
    ) {
        artist = clean(&metadata_artist);
        title = clean(&metadata_title);
    } else {
        for separator in [" - ", " – ", " — ", " / "] {
            if let Some((left, right)) = raw.split_once(separator) {
                artist = left.trim().to_string();
                title = metadata_text_value(right).unwrap_or_else(|| right.trim().to_string());
                break;
            }
        }
    }

    if artist.is_empty() || title.is_empty() {
        let by_re = Regex::new(r"(?i)^(.+?)\s+by\s+(.+)$").expect("valid regex");
        if let Some(captures) = by_re.captures(&raw) {
            title =
                metadata_text_value(&captures[1]).unwrap_or_else(|| captures[1].trim().to_string());
            artist = captures[2].trim().to_string();
        }
    }

    if artist.is_empty() || title.is_empty() {
        uncertain = true;
        artist.clear();
        title.clear();
    }

    Some(TrackEntry {
        artist,
        title,
        raw,
        timestamp: now.unwrap_or_else(Utc::now),
        uncertain,
    })
}

pub fn extract_stream_title(metadata: &str) -> String {
    let re = Regex::new(r"StreamTitle='(?P<title>.*?)';").expect("valid regex");
    re.captures(metadata)
        .and_then(|captures| captures.name("title").map(|m| m.as_str().to_string()))
        .unwrap_or_default()
}

pub struct TrackSessionLog {
    pub station: Station,
    pub show_name: String,
    pub started_at: DateTime<Utc>,
    pub path: PathBuf,
    pub entries: Vec<TrackEntry>,
    seen: HashSet<String>,
    current_stationuuid: String,
}

impl TrackSessionLog {
    pub fn new(root: &Path, station: Station, show_name: &str, started_at: DateTime<Utc>) -> Self {
        let clean_show_name = show_name.trim().to_string();
        let name_for_path = if clean_show_name.is_empty() {
            station.name.as_str()
        } else {
            clean_show_name.as_str()
        };
        let path = root.join(format!(
            "{}-{}.txt",
            slug(name_for_path),
            started_at.format("%Y-%m-%d-%H%M")
        ));
        let current_stationuuid = station.stationuuid.clone();
        Self {
            station,
            show_name: clean_show_name,
            started_at,
            path,
            entries: Vec::new(),
            seen: HashSet::new(),
            current_stationuuid,
        }
    }

    pub fn add_station(&mut self, station: &Station) -> std::io::Result<bool> {
        if station.stationuuid == self.current_stationuuid {
            return Ok(false);
        }
        self.current_stationuuid = station.stationuuid.clone();
        if let Some(parent) = self.path.parent() {
            create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;
        writeln!(file, "\n== Station: {} ==", station.name)?;
        Ok(true)
    }

    pub fn add(&mut self, entry: TrackEntry, station: Option<Station>) -> std::io::Result<bool> {
        if let Some(station) = station {
            self.add_station(&station)?;
        }
        let key = format!("{}\0{}", self.current_stationuuid, entry.display_line()).to_lowercase();
        if self.seen.contains(&key) {
            return Ok(false);
        }
        self.seen.insert(key);
        self.entries.push(entry.clone());
        if let Some(parent) = self.path.parent() {
            create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;
        writeln!(file, "{}", entry.display_line())?;
        Ok(true)
    }
}

fn clean(value: &str) -> String {
    static SPACE_RE: OnceLock<Regex> = OnceLock::new();
    static LEADING_TRACK_RE: OnceLock<Regex> = OnceLock::new();
    let value = SPACE_RE
        .get_or_init(|| Regex::new(r"\s+").expect("valid regex"))
        .replace_all(&value.replace('\0', " "), " ")
        .to_string();
    let value = LEADING_TRACK_RE
        .get_or_init(|| Regex::new(r"^\d+\.\s*").expect("valid regex"))
        .replace(&value, "")
        .to_string();
    value
        .trim()
        .trim_matches(|char| char == ' ' || char == '-')
        .to_string()
}

fn metadata_text_value(value: &str) -> Option<String> {
    metadata_attribute(value, "text").map(|text| clean(&text))
}

fn metadata_attribute(value: &str, name: &str) -> Option<String> {
    for (index, _) in value.match_indices(name) {
        let before = value[..index].chars().next_back();
        if before.is_some_and(|char| char.is_ascii_alphanumeric() || char == '_') {
            continue;
        }
        let after_name = &value[index + name.len()..];
        let Some(after_equals) = after_name.strip_prefix('=') else {
            continue;
        };
        let mut chars = after_equals.char_indices();
        let Some((_, quote @ ('"' | '\''))) = chars.next() else {
            continue;
        };
        let content_start = quote.len_utf8();
        for (offset, char) in after_equals[content_start..].char_indices() {
            if char == quote {
                return Some(after_equals[content_start..content_start + offset].to_string());
            }
        }
    }
    None
}

fn is_bare_station_identifier(value: &str) -> bool {
    let re = Regex::new(r"^[A-Z0-9]{1,6}$").expect("valid regex");
    re.is_match(value)
}

fn slug(value: &str) -> String {
    let re = Regex::new(r"[^a-zA-Z0-9]+").expect("valid regex");
    let slug = re.replace_all(value, "-").trim_matches('-').to_lowercase();
    if slug.is_empty() {
        "station-scout-session".to_string()
    } else {
        slug
    }
}
