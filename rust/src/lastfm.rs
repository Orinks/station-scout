use crate::storage::AppState;
use crate::tracklog::TrackEntry;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs::{OpenOptions, create_dir_all};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const API_ROOT: &str = "https://ws.audioscrobbler.com/2.0/";
const RETRYABLE_ERROR_CODES: &[&str] = &["11", "16"];

#[derive(Clone, Debug)]
pub struct LastFmClient {
    api_key: String,
    api_secret: String,
    session_key: String,
    api_root: String,
}

#[derive(Clone, Debug)]
pub struct LastFmProxyClient {
    proxy_url: String,
    session_key: String,
}

#[derive(Clone, Debug)]
pub struct LastFmScrobbleCache {
    pub path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LastFmError {
    message: String,
}

impl LastFmProxyClient {
    pub fn new(proxy_url: &str, session_key: &str) -> Self {
        Self {
            proxy_url: proxy_url.trim_end_matches('/').to_string(),
            session_key: session_key.to_string(),
        }
    }

    pub fn request_token(&self) -> Result<(String, String), LastFmError> {
        let payload = self.post("token", &json!({}))?;
        let token = json_string(&payload, "token");
        let auth_url = json_string(&payload, "authUrl");
        if token.is_empty() || auth_url.is_empty() {
            return Err(LastFmError::new(
                "Last.fm proxy did not return an auth token",
            ));
        }
        Ok((token, auth_url))
    }

    pub fn create_session(&self, token: &str) -> Result<(String, String), LastFmError> {
        let payload = self.post("session", &json!({ "token": token }))?;
        let session_key = json_string(&payload, "sessionKey");
        let username = json_string(&payload, "username");
        if session_key.is_empty() {
            return Err(LastFmError::new(
                "Last.fm proxy did not return a session key",
            ));
        }
        Ok((session_key, username))
    }

    pub fn update_now_playing(&self, entry: &TrackEntry) -> Result<(), LastFmError> {
        if entry.uncertain || entry.artist.is_empty() || entry.title.is_empty() {
            return Ok(());
        }
        self.post(
            "now-playing",
            &json!({
                "sessionKey": self.session_key,
                "artist": entry.artist,
                "track": entry.title,
            }),
        )
        .map(|_| ())
    }

    pub fn scrobble(&self, entry: &TrackEntry) -> Result<(), LastFmError> {
        if entry.uncertain || entry.artist.is_empty() || entry.title.is_empty() {
            return Ok(());
        }
        self.post(
            "scrobble",
            &json!({
                "sessionKey": self.session_key,
                "artist": entry.artist,
                "track": entry.title,
                "timestamp": entry.timestamp.timestamp().to_string(),
            }),
        )
        .map(|_| ())
    }

    fn post(&self, action: &str, payload: &Value) -> Result<Value, LastFmError> {
        let response = reqwest::blocking::Client::new()
            .post(format!("{}/{}", self.proxy_url, action))
            .header("Content-Type", "application/json")
            .body(payload.to_string())
            .send()
            .map_err(LastFmError::from_display)?;
        let status = response.status();
        let text = response.text().map_err(LastFmError::from_display)?;
        let value = serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({}));
        if !status.is_success() {
            let message = json_string(&value, "error");
            return Err(LastFmError::new(if message.is_empty() {
                format!("Last.fm proxy request failed: {}", status.as_u16())
            } else {
                message
            }));
        }
        if !value.is_object() {
            return Err(LastFmError::new(
                "Last.fm proxy returned an invalid response",
            ));
        }
        Ok(value)
    }
}

impl LastFmScrobbleCache {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    pub fn append(&self, entry: &TrackEntry) -> io::Result<()> {
        if entry.uncertain || entry.artist.is_empty() || entry.title.is_empty() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let payload = json!({
            "artist": entry.artist,
            "title": entry.title,
            "raw": entry.raw,
            "timestamp": entry.timestamp.timestamp(),
        });
        writeln!(file, "{payload}")?;
        Ok(())
    }
}

impl LastFmClient {
    pub fn new(api_key: &str, api_secret: &str, session_key: &str) -> Self {
        Self::new_with_api_root(api_key, api_secret, session_key, API_ROOT)
    }

    pub fn new_with_api_root(
        api_key: &str,
        api_secret: &str,
        session_key: &str,
        api_root: &str,
    ) -> Self {
        Self {
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            session_key: session_key.to_string(),
            api_root: api_root.to_string(),
        }
    }

    pub fn request_token(&self) -> Result<String, LastFmError> {
        let mut payload = BTreeMap::from([
            ("method".to_string(), "auth.getToken".to_string()),
            ("api_key".to_string(), self.api_key.clone()),
        ]);
        let text = self.post_signed_payload(&mut payload)?;
        let document = roxmltree::Document::parse(&text).map_err(LastFmError::from_display)?;
        let token = child_text(document.root_element(), "token").unwrap_or_default();
        if token.is_empty() {
            return Err(LastFmError::new("Last.fm did not return an auth token"));
        }
        Ok(token)
    }

    pub fn auth_url(&self, token: &str) -> String {
        format!(
            "https://www.last.fm/api/auth/?api_key={}&token={}",
            self.api_key, token
        )
    }

    pub fn create_session(&self, token: &str) -> Result<(String, String), LastFmError> {
        let mut payload = BTreeMap::from([
            ("method".to_string(), "auth.getSession".to_string()),
            ("api_key".to_string(), self.api_key.clone()),
            ("token".to_string(), token.to_string()),
        ]);
        let text = self.post_signed_payload(&mut payload)?;
        let document = roxmltree::Document::parse(&text).map_err(LastFmError::from_display)?;
        let session = document
            .descendants()
            .find(|node| node.has_tag_name("session"))
            .ok_or_else(|| LastFmError::new("Last.fm did not return a session"))?;
        let session_key = child_text(session, "key").unwrap_or_default();
        let username = child_text(session, "name").unwrap_or_default();
        if session_key.is_empty() {
            return Err(LastFmError::new("Last.fm did not return a session key"));
        }
        Ok((session_key, username))
    }

    pub fn update_now_playing(&self, entry: &TrackEntry) -> Result<(), LastFmError> {
        if entry.uncertain || entry.artist.is_empty() || entry.title.is_empty() {
            return Ok(());
        }
        self.post_signed(BTreeMap::from([
            ("method".to_string(), "track.updateNowPlaying".to_string()),
            ("artist".to_string(), entry.artist.clone()),
            ("track".to_string(), entry.title.clone()),
        ]))
    }

    pub fn scrobble(&self, entry: &TrackEntry) -> Result<(), LastFmError> {
        if entry.uncertain || entry.artist.is_empty() || entry.title.is_empty() {
            return Ok(());
        }
        self.post_signed(BTreeMap::from([
            ("method".to_string(), "track.scrobble".to_string()),
            ("artist".to_string(), entry.artist.clone()),
            ("track".to_string(), entry.title.clone()),
            (
                "timestamp".to_string(),
                entry.timestamp.timestamp().to_string(),
            ),
            ("chosenByUser".to_string(), "0".to_string()),
        ]))
    }

    fn post_signed(&self, mut payload: BTreeMap<String, String>) -> Result<(), LastFmError> {
        self.post_signed_payload(&mut payload).map(|_| ())
    }

    fn post_signed_payload(
        &self,
        payload: &mut BTreeMap<String, String>,
    ) -> Result<String, LastFmError> {
        payload.insert("api_key".to_string(), self.api_key.clone());
        if !self.session_key.is_empty() {
            payload.insert("sk".to_string(), self.session_key.clone());
        }
        payload.insert(
            "api_sig".to_string(),
            sign_lastfm_request(payload, &self.api_secret),
        );
        let text = reqwest::blocking::Client::new()
            .post(&self.api_root)
            .form(payload)
            .send()
            .map_err(LastFmError::from_display)?
            .error_for_status()
            .map_err(LastFmError::from_display)?
            .text()
            .map_err(LastFmError::from_display)?;
        raise_for_lfm_failure(&text)?;
        Ok(text)
    }
}

pub fn sign_lastfm_request(params: &BTreeMap<String, String>, api_secret: &str) -> String {
    let mut signature_base = String::new();
    for (key, value) in params {
        if key != "format" {
            signature_base.push_str(key);
            signature_base.push_str(value);
        }
    }
    signature_base.push_str(api_secret);
    format!("{:x}", md5::compute(signature_base.as_bytes()))
}

pub fn lastfm_track_key(entry: &TrackEntry) -> String {
    format!("{}\0{}", entry.artist, entry.title).to_lowercase()
}

pub fn should_scrobble_lastfm(
    state: &AppState,
    entry: &TrackEntry,
    sent_tracks: &HashSet<String>,
) -> bool {
    if !state.lastfm_enabled || !state.lastfm_scrobble_enabled {
        return false;
    }
    if entry.uncertain || entry.artist.is_empty() || entry.title.is_empty() {
        return false;
    }
    !sent_tracks.contains(&lastfm_track_key(entry))
}

pub fn should_show_stream_title(entry: &TrackEntry) -> bool {
    !entry.uncertain && !entry.artist.is_empty() && !entry.title.is_empty()
}

pub fn lastfm_settings_view_state(state: &AppState) -> (String, String, bool) {
    if state.lastfm_enabled {
        return (
            format!(
                "Connected as {}.",
                if state.lastfm_username.is_empty() {
                    "your account"
                } else {
                    &state.lastfm_username
                }
            ),
            "Reconnect Last.fm".to_string(),
            false,
        );
    }
    if !state.lastfm_pending_token.is_empty() {
        return (
            "Waiting for browser approval. Choose Finish Last.fm afterward.".to_string(),
            "Restart Last.fm".to_string(),
            true,
        );
    }
    (
        "Not connected.".to_string(),
        "Connect Last.fm".to_string(),
        false,
    )
}

fn raise_for_lfm_failure(xml_text: &str) -> Result<(), LastFmError> {
    let document = roxmltree::Document::parse(xml_text).map_err(LastFmError::from_display)?;
    let root = document.root_element();
    if root.attribute("status") == Some("ok") {
        return Ok(());
    }
    let error = root.descendants().find(|node| node.has_tag_name("error"));
    let code = error
        .and_then(|node| node.attribute("code"))
        .unwrap_or_default();
    let message = error
        .and_then(|node| node.text())
        .unwrap_or("Last.fm request failed");
    let retry = if RETRYABLE_ERROR_CODES.contains(&code) {
        "retryable"
    } else {
        "not retryable"
    };
    Err(LastFmError::new(format!(
        "{message} ({retry}, code {code})"
    )))
}

fn child_text(node: roxmltree::Node<'_, '_>, tag_name: &str) -> Option<String> {
    node.children()
        .find(|child| child.has_tag_name(tag_name))
        .and_then(|child| child.text())
        .map(str::to_string)
}

fn json_string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

impl LastFmError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn from_display(error: impl fmt::Display) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

impl fmt::Display for LastFmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LastFmError {}
