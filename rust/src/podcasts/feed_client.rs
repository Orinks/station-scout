use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;
use url::Url;

pub const DEFAULT_USER_AGENT: &str = "StationScout/1.0";
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 20;
pub const MAX_REDIRECTS: usize = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedUrlSchemeError {
    url: String,
}

impl fmt::Display for UnsupportedUrlSchemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsupported URL scheme: {:?}", self.url)
    }
}

impl std::error::Error for UnsupportedUrlSchemeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedFetchResponse {
    pub url: String,
    pub status: u16,
    pub body: Vec<u8>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub not_modified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedFetchError {
    pub url: String,
    pub status: Option<u16>,
    pub message: String,
}

impl FeedFetchError {
    fn new(url: impl Into<String>, status: Option<u16>, message: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            status,
            message: message.into(),
        }
    }
}

impl fmt::Display for FeedFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Fetch failed for {:?} (status={:?}): {}",
            self.url, self.status, self.message
        )
    }
}

impl std::error::Error for FeedFetchError {}

pub fn validate_feed_url(url: &str) -> Result<(), UnsupportedUrlSchemeError> {
    let parsed = Url::parse(url.trim()).map_err(|_| UnsupportedUrlSchemeError {
        url: url.to_string(),
    })?;
    if matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some() {
        Ok(())
    } else {
        Err(UnsupportedUrlSchemeError {
            url: url.to_string(),
        })
    }
}

pub fn feed_fetch_headers(
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::from([("User-Agent".to_string(), DEFAULT_USER_AGENT.to_string())]);
    if let Some(etag) = etag.filter(|value| !value.trim().is_empty()) {
        headers.insert("If-None-Match".to_string(), etag.to_string());
    }
    if let Some(last_modified) = last_modified.filter(|value| !value.trim().is_empty()) {
        headers.insert("If-Modified-Since".to_string(), last_modified.to_string());
    }
    headers
}

pub fn fetch_feed_sync(
    url: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<FeedFetchResponse, FeedFetchError> {
    validate_feed_url(url).map_err(|error| FeedFetchError::new(url, None, error.to_string()))?;
    let client = reqwest::blocking::Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .build()
        .map_err(|error| FeedFetchError::new(url, None, error.to_string()))?;

    let mut request = client.get(url);
    if let Some(etag) = etag.filter(|value| !value.trim().is_empty()) {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    if let Some(last_modified) = last_modified.filter(|value| !value.trim().is_empty()) {
        request = request.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
    }

    let response = request.send().map_err(|error| {
        FeedFetchError::new(
            url,
            error.status().map(|status| status.as_u16()),
            error.to_string(),
        )
    })?;
    let final_url = response.url().as_str().to_string();
    validate_feed_url(&final_url)
        .map_err(|error| FeedFetchError::new(final_url.clone(), None, error.to_string()))?;
    let status = response.status();
    let etag = header_value(&response, reqwest::header::ETAG);
    let last_modified = header_value(&response, reqwest::header::LAST_MODIFIED);

    if status == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(FeedFetchResponse {
            url: final_url,
            status: status.as_u16(),
            body: Vec::new(),
            etag,
            last_modified,
            not_modified: true,
        });
    }

    if status.is_client_error() || status.is_server_error() {
        return Err(FeedFetchError::new(
            final_url,
            Some(status.as_u16()),
            status.canonical_reason().unwrap_or("HTTP error"),
        ));
    }

    let body = response
        .bytes()
        .map_err(|error| {
            FeedFetchError::new(
                url,
                error.status().map(|status| status.as_u16()),
                error.to_string(),
            )
        })?
        .to_vec();
    Ok(FeedFetchResponse {
        url: final_url,
        status: status.as_u16(),
        body,
        etag,
        last_modified,
        not_modified: false,
    })
}

fn header_value(
    response: &reqwest::blocking::Response,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}
