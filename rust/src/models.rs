use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrowseFacetKind {
    Genre,
    Location,
    Language,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowseFacet {
    pub kind: BrowseFacetKind,
    pub name: String,
    pub station_count: i32,
    pub code: String,
}

impl BrowseFacet {
    pub fn from_api(kind: BrowseFacetKind, payload: Value) -> Self {
        let code_key = if kind == BrowseFacetKind::Location {
            "iso_3166_1"
        } else {
            "iso_639"
        };
        Self {
            kind,
            name: value_string(payload.get("name")).unwrap_or_default(),
            station_count: value_i32(payload.get("stationcount")),
            code: value_string(payload.get(code_key)).unwrap_or_default(),
        }
    }

    pub fn query_value(&self) -> &str {
        if self.kind == BrowseFacetKind::Location && !self.code.is_empty() {
            &self.code
        } else {
            &self.name
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Station {
    pub stationuuid: String,
    pub name: String,
    pub url_resolved: String,
    pub homepage: String,
    pub favicon: String,
    pub tags: String,
    pub country: String,
    pub countrycode: String,
    pub state: String,
    pub language: String,
    pub codec: String,
    pub bitrate: i32,
    pub hls: i32,
    pub lastcheckok: i32,
    pub votes: i32,
    pub clickcount: i32,
    pub clicktrend: i32,
    pub source: String,
}

impl Station {
    pub fn new(stationuuid: &str, name: &str, url_resolved: &str) -> Self {
        Self {
            stationuuid: stationuuid.to_string(),
            name: name.to_string(),
            url_resolved: url_resolved.to_string(),
            homepage: String::new(),
            favicon: String::new(),
            tags: String::new(),
            country: String::new(),
            countrycode: String::new(),
            state: String::new(),
            language: String::new(),
            codec: String::new(),
            bitrate: 0,
            hls: 0,
            lastcheckok: 0,
            votes: 0,
            clickcount: 0,
            clicktrend: 0,
            source: "Radio Browser".to_string(),
        }
    }

    pub fn from_api(payload: Value) -> Self {
        Self {
            stationuuid: value_string(payload.get("stationuuid")).unwrap_or_default(),
            name: value_string(payload.get("name"))
                .unwrap_or_else(|| "Unknown station".to_string()),
            url_resolved: value_string(payload.get("url_resolved"))
                .or_else(|| value_string(payload.get("url")))
                .unwrap_or_default(),
            homepage: value_string(payload.get("homepage")).unwrap_or_default(),
            favicon: value_string(payload.get("favicon")).unwrap_or_default(),
            tags: value_string(payload.get("tags")).unwrap_or_default(),
            country: value_string(payload.get("country")).unwrap_or_default(),
            countrycode: value_string(payload.get("countrycode")).unwrap_or_default(),
            state: value_string(payload.get("state")).unwrap_or_default(),
            language: value_string(payload.get("language")).unwrap_or_default(),
            codec: value_string(payload.get("codec")).unwrap_or_default(),
            bitrate: value_i32(payload.get("bitrate")),
            hls: value_i32(payload.get("hls")),
            lastcheckok: value_i32(payload.get("lastcheckok")),
            votes: value_i32(payload.get("votes")),
            clickcount: value_i32(payload.get("clickcount")),
            clicktrend: value_i32(payload.get("clicktrend")),
            source: value_string(payload.get("source"))
                .unwrap_or_else(|| "Radio Browser".to_string()),
        }
    }

    pub fn direct_stream(name: &str, url: &str, homepage: &str, source: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(format!("{source}\0{name}\0{url}").as_bytes());
        let digest = to_lower_hex(&hasher.finalize())[..24].to_string();
        Self {
            stationuuid: format!("direct-{digest}"),
            name: name.to_string(),
            url_resolved: url.to_string(),
            homepage: homepage.to_string(),
            source: source.to_string(),
            lastcheckok: 1,
            ..Self::new("", "", "")
        }
    }

    pub fn subtitle(&self) -> String {
        [&self.country, &self.language, &self.tags]
            .into_iter()
            .filter(|part| !part.is_empty())
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ")
    }

    pub fn quality_label(&self) -> String {
        let mut bits = Vec::new();
        if self.lastcheckok != 0 {
            bits.push("working".to_string());
        }
        if !self.codec.is_empty() {
            bits.push(self.codec.clone());
        }
        if self.bitrate != 0 {
            bits.push(format!("{} kbps", self.bitrate));
        }
        if self.hls != 0 {
            bits.push("HLS".to_string());
        }
        if self.source != "Radio Browser" {
            bits.push(self.source.clone());
        }
        if bits.is_empty() {
            "stream details unavailable".to_string()
        } else {
            bits.join(", ")
        }
    }

    pub fn discovery_quality_score(&self) -> i32 {
        let mut score = 0;
        if self.lastcheckok != 0 {
            score += 30;
        }
        score += match self.bitrate {
            320.. => 30,
            192.. => 25,
            128.. => 20,
            64.. => 10,
            1.. => 5,
            _ => 0,
        };

        match self.codec.to_lowercase().replace(' ', "").as_str() {
            "flac" | "alac" => score += 20,
            "opus" | "aac" | "aac+" | "heaac" | "vorbis" | "ogg" => score += 15,
            "mp3" => score += 10,
            "" => {}
            _ => score += 5,
        }

        if self.hls != 0 {
            score += 5;
        }
        if self.clicktrend > 0 {
            score += 15.min(self.clicktrend * 3);
        } else if self.clickcount > 0 {
            score += 10.min(self.clickcount / 100);
        }
        if self.votes > 0 {
            score += 5.min(self.votes / 100);
        }
        score.min(100)
    }

    pub fn discovery_quality_label(&self) -> String {
        let score = self.discovery_quality_score();
        let tier = if score >= 80 {
            "excellent"
        } else if score >= 60 {
            "good"
        } else if score >= 40 {
            "fair"
        } else {
            "limited"
        };
        format!(
            "{tier} discovery match, {score}/100, {}",
            self.quality_label()
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TuneTimer {
    pub stationuuid: String,
    pub station_name: String,
    pub time: String,
    pub enabled: bool,
    pub auto_play: bool,
    pub end_time: String,
    pub show_name: String,
    pub track_playlist: bool,
}

impl TuneTimer {
    pub fn new(stationuuid: &str, station_name: &str, time: &str) -> Self {
        Self {
            stationuuid: stationuuid.to_string(),
            station_name: station_name.to_string(),
            time: time.to_string(),
            enabled: true,
            auto_play: true,
            end_time: String::new(),
            show_name: String::new(),
            track_playlist: false,
        }
    }

    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).expect("TuneTimer serializes")
    }

    pub fn from_json(payload: Value) -> Self {
        let object = payload.as_object().cloned().unwrap_or_default();
        Self {
            stationuuid: map_string(&object, "stationuuid").unwrap_or_default(),
            station_name: map_string(&object, "station_name")
                .unwrap_or_else(|| "Unknown station".to_string()),
            time: map_string(&object, "time").unwrap_or_else(|| "09:00".to_string()),
            enabled: object
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            auto_play: object
                .get("auto_play")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            end_time: map_string(&object, "end_time").unwrap_or_default(),
            show_name: map_string(&object, "show_name").unwrap_or_default(),
            track_playlist: object
                .get("track_playlist")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }
    }
}

fn map_string(object: &Map<String, Value>, key: &str) -> Option<String> {
    value_string(object.get(key))
}

fn value_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Null => None,
        other => Some(other.to_string().trim_matches('"').to_string()),
    }
    .filter(|text| !text.is_empty())
}

fn value_i32(value: Option<&Value>) -> i32 {
    match value {
        Some(Value::Number(number)) => number.as_i64().unwrap_or(0) as i32,
        Some(Value::String(text)) => text.parse::<i32>().unwrap_or(0),
        Some(Value::Bool(flag)) => i32::from(*flag),
        _ => 0,
    }
}

fn to_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
