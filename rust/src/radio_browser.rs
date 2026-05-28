use crate::models::{BrowseFacet, BrowseFacetKind, Station};
use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

pub const DEFAULT_SERVERS: [&str; 2] = [
    "https://de1.api.radio-browser.info",
    "https://nl1.api.radio-browser.info",
];
pub const DEFAULT_USER_AGENT: &str = "StationScout/1.0 (+https://github.com/Orinks/station-scout)";
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 12;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiRequest {
    pub path: String,
    pub params: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RadioBrowserError {
    message: String,
}

impl RadioBrowserError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RadioBrowserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RadioBrowserError {}

#[derive(Clone, Debug)]
pub struct RadioBrowserClient {
    base_urls: Vec<String>,
    timeout: Duration,
    user_agent: String,
}

impl Default for RadioBrowserClient {
    fn default() -> Self {
        Self::new(DEFAULT_SERVERS)
    }
}

impl RadioBrowserClient {
    pub fn new(base_urls: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            base_urls: base_urls.into_iter().map(Into::into).collect(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECONDS),
            user_agent: DEFAULT_USER_AGENT.to_string(),
        }
    }

    pub fn search(
        &self,
        name: &str,
        country: &str,
        language: &str,
        tag: &str,
        limit: i32,
    ) -> Result<Vec<Station>, RadioBrowserError> {
        let request = search_request(name, country, language, tag, limit);
        let rows = self.get_json(&request)?;
        Ok(rows
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row.is_object().then(|| Station::from_api(row.clone())))
            .collect())
    }

    pub fn browse_facets(
        &self,
        kind: BrowseFacetKind,
        limit: i32,
    ) -> Result<Vec<BrowseFacet>, RadioBrowserError> {
        let endpoint = match kind {
            BrowseFacetKind::Genre => BrowseEndpoint::Genre,
            BrowseFacetKind::Location => BrowseEndpoint::Location,
            BrowseFacetKind::Language => BrowseEndpoint::Language,
        };
        let request = browse_facet_request(endpoint, limit);
        let rows = self.get_json(&request)?;
        let facets = rows
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| {
                row.is_object()
                    .then(|| BrowseFacet::from_api(kind.clone(), row.clone()))
            })
            .filter_map(clean_browse_facet)
            .take(limit.clamp(1, 500) as usize)
            .collect();
        Ok(facets)
    }

    pub fn browse_stations(
        &self,
        facet: &BrowseFacet,
        limit: i32,
    ) -> Result<Vec<Station>, RadioBrowserError> {
        let stations = match facet.kind {
            BrowseFacetKind::Genre => self.search("", "", "", facet.query_value(), limit)?,
            BrowseFacetKind::Location => self.search("", facet.query_value(), "", "", limit)?,
            BrowseFacetKind::Language => self.search("", "", facet.query_value(), "", limit)?,
        };
        Ok(discovery_sorted(stations))
    }

    pub fn click_url(&self, stationuuid: &str) -> Result<String, RadioBrowserError> {
        let request = ApiRequest {
            path: format!("/json/url/{stationuuid}"),
            params: BTreeMap::new(),
        };
        let payload = self.get_json(&request)?;
        let ok = payload
            .get("ok")
            .and_then(|value| {
                value
                    .as_bool()
                    .or_else(|| value.as_str().map(|text| text.eq_ignore_ascii_case("true")))
            })
            .unwrap_or(false);
        if !ok {
            return Err(RadioBrowserError::new(
                "Radio Browser did not return a playable URL.",
            ));
        }
        Ok(payload
            .get("url")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string())
    }

    fn get_json(&self, request: &ApiRequest) -> Result<serde_json::Value, RadioBrowserError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .user_agent(&self.user_agent)
            .build()
            .map_err(|error| RadioBrowserError::new(error.to_string()))?;
        let mut errors = Vec::new();
        for base_url in &self.base_urls {
            let url = format!("{}{}", base_url.trim_end_matches('/'), request.path);
            let result = client.get(&url).query(&request.params).send();
            match result {
                Ok(response) => match response.error_for_status() {
                    Ok(response) => match response.bytes() {
                        Ok(body) => match serde_json::from_slice::<serde_json::Value>(&body) {
                            Ok(payload) => return Ok(payload),
                            Err(error) => errors.push(format!("{url}: {error}")),
                        },
                        Err(error) => errors.push(format!("{url}: {error}")),
                    },
                    Err(error) => errors.push(format!("{url}: {error}")),
                },
                Err(error) => errors.push(format!("{url}: {error}")),
            }
        }
        Err(RadioBrowserError::new(if errors.is_empty() {
            "Radio Browser request failed.".to_string()
        } else {
            errors.join("; ")
        }))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowseEndpoint {
    Genre,
    Location,
    Language,
}

pub fn search_request(
    name: &str,
    country: &str,
    language: &str,
    tag: &str,
    limit: i32,
) -> ApiRequest {
    let mut params = BTreeMap::from([
        ("hidebroken".to_string(), "true".to_string()),
        ("limit".to_string(), limit.clamp(1, 100).to_string()),
        ("order".to_string(), "clicktrend".to_string()),
        ("reverse".to_string(), "true".to_string()),
    ]);
    if !name.is_empty() {
        params.insert("name".to_string(), name.to_string());
    }
    if country.len() == 2 && country.chars().all(|char| char.is_ascii_alphabetic()) {
        params.insert("countrycode".to_string(), country.to_uppercase());
    } else if !country.is_empty() {
        params.insert("country".to_string(), country.to_string());
    }
    if !language.is_empty() {
        params.insert("language".to_string(), language.to_string());
    }
    if !tag.is_empty() {
        params.insert("tag".to_string(), tag.to_string());
    }
    ApiRequest {
        path: "/json/stations/search".to_string(),
        params,
    }
}

pub fn browse_facet_request(kind: BrowseEndpoint, limit: i32) -> ApiRequest {
    let path = match kind {
        BrowseEndpoint::Genre => "/json/tags",
        BrowseEndpoint::Location => "/json/countries",
        BrowseEndpoint::Language => "/json/languages",
    };
    ApiRequest {
        path: path.to_string(),
        params: BTreeMap::from([
            ("hidebroken".to_string(), "true".to_string()),
            ("limit".to_string(), limit.clamp(1, 500).to_string()),
            ("order".to_string(), "stationcount".to_string()),
            ("reverse".to_string(), "true".to_string()),
        ]),
    }
}

pub fn clean_browse_facet(facet: BrowseFacet) -> Option<BrowseFacet> {
    let name = facet
        .name
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() || name.starts_with('#') || !name.chars().any(char::is_alphanumeric) {
        return None;
    }
    let code = if facet.kind == BrowseFacetKind::Location {
        facet.code.trim().to_uppercase()
    } else {
        facet.code.trim().to_string()
    };
    Some(BrowseFacet {
        kind: facet.kind,
        name,
        station_count: facet.station_count,
        code,
    })
}

pub fn discovery_sorted(mut stations: Vec<Station>) -> Vec<Station> {
    stations.sort_by(|left, right| {
        right
            .discovery_quality_score()
            .cmp(&left.discovery_quality_score())
            .then_with(|| right.bitrate.cmp(&left.bitrate))
            .then_with(|| right.clicktrend.cmp(&left.clicktrend))
            .then_with(|| right.clickcount.cmp(&left.clickcount))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    stations
}
