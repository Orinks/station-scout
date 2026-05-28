use crate::podcasts::feed_client::{
    DEFAULT_TIMEOUT_SECONDS, DEFAULT_USER_AGENT, validate_feed_url,
};
use crate::storage::AppState;
use reqwest::StatusCode;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

pub const PODCAST_INDEX_SEARCH_URL: &str = "https://api.podcastindex.org/api/1.0/search/byterm";
pub const ITUNES_SEARCH_URL: &str = "https://itunes.apple.com/search";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PodcastIndexCredentials {
    pub key: String,
    pub secret: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResult {
    pub feed_url: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchOutcome {
    pub results: Vec<SearchResult>,
    pub source: String,
    pub fallback_used: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryError {
    message: String,
    kind: DiscoveryErrorKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscoveryErrorKind {
    Request,
    Auth,
}

impl DiscoveryError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: DiscoveryErrorKind::Request,
        }
    }

    fn auth(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: DiscoveryErrorKind::Auth,
        }
    }

    pub fn kind(&self) -> DiscoveryErrorKind {
        self.kind
    }

    pub fn is_auth_error(&self) -> bool {
        self.kind == DiscoveryErrorKind::Auth
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DiscoveryError {}

#[derive(Clone, Debug)]
pub struct PodcastSearchClient {
    podcast_index_url: String,
    itunes_url: String,
}

pub fn podcast_index_credentials_from_state(state: &AppState) -> Option<PodcastIndexCredentials> {
    let key = state.podcast_index_api_key.trim();
    let secret = state.podcast_index_api_secret.trim();
    if key.is_empty() || secret.is_empty() {
        return None;
    }
    Some(PodcastIndexCredentials {
        key: key.to_string(),
        secret: secret.to_string(),
    })
}

impl Default for PodcastSearchClient {
    fn default() -> Self {
        Self::new(PODCAST_INDEX_SEARCH_URL, ITUNES_SEARCH_URL)
    }
}

impl PodcastSearchClient {
    pub fn new(podcast_index_url: impl Into<String>, itunes_url: impl Into<String>) -> Self {
        Self {
            podcast_index_url: podcast_index_url.into(),
            itunes_url: itunes_url.into(),
        }
    }

    pub fn search(
        &self,
        query: &str,
        credentials: Option<&PodcastIndexCredentials>,
        auth_date: &str,
        max_results: i32,
    ) -> Result<SearchOutcome, DiscoveryError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(SearchOutcome {
                results: Vec::new(),
                source: "podcast_index".to_string(),
                fallback_used: false,
            });
        }
        if let Some(credentials) = credentials.filter(|credential| {
            !credential.key.trim().is_empty() && !credential.secret.trim().is_empty()
        }) {
            let results = self.search_podcast_index(query, credentials, auth_date, max_results)?;
            Ok(SearchOutcome {
                results,
                source: "podcast_index".to_string(),
                fallback_used: false,
            })
        } else {
            let results = self.search_itunes(query, max_results)?;
            Ok(SearchOutcome {
                results,
                source: "itunes".to_string(),
                fallback_used: true,
            })
        }
    }

    fn search_podcast_index(
        &self,
        query: &str,
        credentials: &PodcastIndexCredentials,
        auth_date: &str,
        max_results: i32,
    ) -> Result<Vec<SearchResult>, DiscoveryError> {
        let headers = build_podcast_index_auth_headers(credentials, auth_date);
        let payload = self.get_json(
            &self.podcast_index_url,
            &[
                ("q", query.to_string()),
                ("max", max_results.clamp(1, 200).to_string()),
            ],
            &headers,
            Some("Podcast Index"),
        )?;
        Ok(payload
            .get("feeds")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(parse_podcast_index_result)
            .collect())
    }

    fn search_itunes(
        &self,
        query: &str,
        max_results: i32,
    ) -> Result<Vec<SearchResult>, DiscoveryError> {
        let payload = self.get_json(
            &self.itunes_url,
            &[
                ("term", query.to_string()),
                ("entity", "podcast".to_string()),
                ("limit", max_results.clamp(1, 200).to_string()),
            ],
            &BTreeMap::new(),
            None,
        )?;
        Ok(payload
            .get("results")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(parse_itunes_result)
            .collect())
    }

    fn get_json(
        &self,
        url: &str,
        query: &[(&str, String)],
        headers: &BTreeMap<String, String>,
        auth_failure_provider: Option<&str>,
    ) -> Result<serde_json::Value, DiscoveryError> {
        validate_feed_url(url).map_err(|error| DiscoveryError::new(error.to_string()))?;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
            .user_agent(DEFAULT_USER_AGENT)
            .build()
            .map_err(|error| DiscoveryError::new(error.to_string()))?;
        let mut request = client.get(url).query(query);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request
            .send()
            .map_err(|error| DiscoveryError::new(error.to_string()))?;
        if auth_failure_provider.is_some()
            && matches!(
                response.status(),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
            )
        {
            let provider = auth_failure_provider.unwrap_or("service");
            return Err(DiscoveryError::auth(format!(
                "{provider} rejected the credential (status={})",
                response.status().as_u16()
            )));
        }
        let response = response
            .error_for_status()
            .map_err(|error| DiscoveryError::new(error.to_string()))?;
        let body = response
            .bytes()
            .map_err(|error| DiscoveryError::new(error.to_string()))?;
        serde_json::from_slice(&body).map_err(|error| DiscoveryError::new(error.to_string()))
    }
}

pub fn build_podcast_index_auth_headers(
    credentials: &PodcastIndexCredentials,
    auth_date: &str,
) -> BTreeMap<String, String> {
    let mut hasher = Sha1::new();
    hasher.update(format!("{}{}{}", credentials.key, credentials.secret, auth_date).as_bytes());
    let authorization = to_lower_hex(&hasher.finalize());
    BTreeMap::from([
        ("Authorization".to_string(), authorization),
        ("User-Agent".to_string(), DEFAULT_USER_AGENT.to_string()),
        ("X-Auth-Date".to_string(), auth_date.to_string()),
        ("X-Auth-Key".to_string(), credentials.key.clone()),
    ])
}

fn parse_podcast_index_result(value: &serde_json::Value) -> Option<SearchResult> {
    let feed_url = text(value.get("url"))?;
    if validate_feed_url(&feed_url).is_err() {
        return None;
    }
    Some(SearchResult {
        title: text(value.get("title")).unwrap_or_else(|| feed_url.clone()),
        author: text(value.get("author")),
        description: text(value.get("description")),
        image_url: text(value.get("image")).or_else(|| text(value.get("artwork"))),
        feed_url,
        source: "podcast_index".to_string(),
    })
}

fn parse_itunes_result(value: &serde_json::Value) -> Option<SearchResult> {
    let feed_url = text(value.get("feedUrl"))?;
    if validate_feed_url(&feed_url).is_err() {
        return None;
    }
    Some(SearchResult {
        title: text(value.get("collectionName")).unwrap_or_else(|| feed_url.clone()),
        author: text(value.get("artistName")),
        description: text(value.get("collectionCensoredName")),
        image_url: text(value.get("artworkUrl600"))
            .or_else(|| text(value.get("artworkUrl100")))
            .or_else(|| text(value.get("artworkUrl60"))),
        feed_url,
        source: "itunes".to_string(),
    })
}

fn text(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(text) => Some(text.trim().to_string()),
        serde_json::Value::Null => None,
        other => Some(other.to_string().trim_matches('"').to_string()),
    }
    .filter(|value| !value.is_empty())
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
