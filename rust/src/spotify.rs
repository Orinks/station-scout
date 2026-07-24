use crate::tracklog::TrackEntry;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use url::form_urlencoded::Serializer;

pub const AUTH_URL: &str = "https://accounts.spotify.com/authorize";
pub const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
pub const API_URL: &str = "https://api.spotify.com/v1";
pub const DEFAULT_SCOPES: &[&str] = &[
    "playlist-modify-private",
    "playlist-modify-public",
    "user-read-private",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpotifyAuthRequest {
    pub url: String,
    pub state: String,
    pub code_verifier: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpotifyTokenSet {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub expires_in: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpotifyPlaylistResult {
    pub name: String,
    pub url: String,
    pub matched_tracks: usize,
    pub skipped_tracks: usize,
}

#[derive(Clone, Debug)]
pub struct SpotifyClient {
    client_id: String,
    redirect_uri: String,
    token_url: String,
    api_base_url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpotifyError {
    message: String,
}

impl SpotifyClient {
    pub fn new(client_id: &str, redirect_uri: &str) -> Self {
        Self::new_with_urls(client_id, redirect_uri, TOKEN_URL, API_URL)
    }

    pub fn new_with_token_url(client_id: &str, redirect_uri: &str, token_url: &str) -> Self {
        Self::new_with_urls(client_id, redirect_uri, token_url, API_URL)
    }

    pub fn new_with_urls(
        client_id: &str,
        redirect_uri: &str,
        token_url: &str,
        api_base_url: &str,
    ) -> Self {
        Self {
            client_id: client_id.to_string(),
            redirect_uri: redirect_uri.to_string(),
            token_url: token_url.to_string(),
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<SpotifyTokenSet, SpotifyError> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("client_id", self.client_id.as_str()),
            ("code_verifier", code_verifier),
        ];
        let payload = reqwest::blocking::Client::new()
            .post(&self.token_url)
            .form(&params)
            .send()
            .map_err(SpotifyError::from_display)?
            .error_for_status()
            .map_err(SpotifyError::from_display)?
            .text()
            .map_err(SpotifyError::from_display)?;
        let payload: SpotifyTokenPayload =
            serde_json::from_str(&payload).map_err(SpotifyError::from_display)?;
        Ok(payload.into_token_set(None, unix_seconds()))
    }

    pub fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<SpotifyTokenSet, SpotifyError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
        ];
        let payload = reqwest::blocking::Client::new()
            .post(&self.token_url)
            .form(&params)
            .send()
            .map_err(SpotifyError::from_display)?
            .error_for_status()
            .map_err(SpotifyError::from_display)?
            .text()
            .map_err(SpotifyError::from_display)?;
        let payload: SpotifyTokenPayload =
            serde_json::from_str(&payload).map_err(SpotifyError::from_display)?;
        Ok(payload.into_token_set(Some(refresh_token), unix_seconds()))
    }

    pub fn create_playlist_from_tracks(
        &self,
        access_token: &str,
        name: &str,
        tracks: &[(String, String)],
    ) -> Result<SpotifyPlaylistResult, SpotifyError> {
        let user_id = self.current_user_id(access_token)?;
        let playlist = self.create_playlist(access_token, &user_id, name)?;
        let playlist_id = json_string(&playlist, &["id"]).unwrap_or_default();
        let playlist_url =
            json_string(&playlist, &["external_urls", "spotify"]).unwrap_or_default();
        let mut uris = Vec::new();
        let mut skipped = 0;
        for (artist, title) in tracks {
            match self.search_track_uri(access_token, artist, title)? {
                Some(uri) if !uri.is_empty() => uris.push(uri),
                _ => skipped += 1,
            }
        }
        if !uris.is_empty() {
            self.add_items_to_playlist(access_token, &playlist_id, &uris)?;
        }
        Ok(SpotifyPlaylistResult {
            name: name.to_string(),
            url: playlist_url,
            matched_tracks: uris.len(),
            skipped_tracks: skipped,
        })
    }

    fn current_user_id(&self, access_token: &str) -> Result<String, SpotifyError> {
        let value = self.get_json(access_token, &format!("{}/me", self.api_base_url), &[])?;
        json_string(&value, &["id"]).ok_or_else(|| SpotifyError::new("missing Spotify user id"))
    }

    fn create_playlist(
        &self,
        access_token: &str,
        user_id: &str,
        name: &str,
    ) -> Result<Value, SpotifyError> {
        let user_id = encode_path_segment(user_id);
        self.post_json(
            access_token,
            &format!("{}/users/{user_id}/playlists", self.api_base_url),
            &json!({
                "name": name,
                "public": false,
                "description": "Created from a Station Scout tracking session."
            }),
        )
    }

    fn search_track_uri(
        &self,
        access_token: &str,
        artist: &str,
        title: &str,
    ) -> Result<Option<String>, SpotifyError> {
        let query = format!("track:\"{title}\" artist:\"{artist}\"");
        let value = self.get_json(
            access_token,
            &format!("{}/search", self.api_base_url),
            &[("q", query.as_str()), ("type", "track"), ("limit", "1")],
        )?;
        Ok(value
            .pointer("/tracks/items/0/uri")
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    fn add_items_to_playlist(
        &self,
        access_token: &str,
        playlist_id: &str,
        uris: &[String],
    ) -> Result<(), SpotifyError> {
        let playlist_id = encode_path_segment(playlist_id);
        for chunk in uris.chunks(100) {
            self.post_json(
                access_token,
                &format!("{}/playlists/{playlist_id}/tracks", self.api_base_url),
                &json!({ "uris": chunk }),
            )?;
        }
        Ok(())
    }

    fn get_json(
        &self,
        access_token: &str,
        url: &str,
        query: &[(&str, &str)],
    ) -> Result<Value, SpotifyError> {
        let payload = reqwest::blocking::Client::new()
            .get(url)
            .bearer_auth(access_token)
            .query(query)
            .send()
            .map_err(SpotifyError::from_display)?
            .error_for_status()
            .map_err(SpotifyError::from_display)?
            .text()
            .map_err(SpotifyError::from_display)?;
        serde_json::from_str(&payload).map_err(SpotifyError::from_display)
    }

    fn post_json(
        &self,
        access_token: &str,
        url: &str,
        body: &Value,
    ) -> Result<Value, SpotifyError> {
        let payload = reqwest::blocking::Client::new()
            .post(url)
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .map_err(SpotifyError::from_display)?
            .error_for_status()
            .map_err(SpotifyError::from_display)?
            .text()
            .map_err(SpotifyError::from_display)?;
        serde_json::from_str(&payload).map_err(SpotifyError::from_display)
    }
}

pub fn build_spotify_auth_request(
    client_id: &str,
    redirect_uri: &str,
    scopes: &[&str],
) -> SpotifyAuthRequest {
    // PKCE and the CSRF `state` only protect the flow if these values are
    // unpredictable, so draw them from the OS CSPRNG. Byte counts mirror the
    // reference Python app (secrets.token_urlsafe(24) / token_urlsafe(64)).
    let state = random_token(24);
    let code_verifier = random_token(64);
    let code_challenge = code_challenge(&code_verifier);
    let query = Serializer::new(String::new())
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", &scopes.join(" "))
        .append_pair("state", &state)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", &code_challenge)
        .finish();
    SpotifyAuthRequest {
        url: format!("{AUTH_URL}?{query}"),
        state,
        code_verifier,
    }
}

pub fn spotify_playlist_tracks(entries: &[TrackEntry]) -> Vec<(String, String)> {
    let mut tracks = Vec::new();
    let mut seen = HashSet::new();
    for entry in entries {
        if entry.uncertain || entry.artist.is_empty() || entry.title.is_empty() {
            continue;
        }
        let key = format!("{}\0{}", entry.artist, entry.title).to_lowercase();
        if seen.insert(key) {
            tracks.push((entry.artist.clone(), entry.title.clone()));
        }
    }
    tracks
}

pub fn code_challenge(code_verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()))
}

/// Generate a URL-safe random token from `num_bytes` of OS entropy. The
/// base64url-no-pad alphabet (`A-Za-z0-9-_`) is a subset of the PKCE
/// unreserved character set, so the result is safe to use directly as a
/// `code_verifier` or CSRF `state`.
fn random_token(num_bytes: usize) -> String {
    let mut bytes = vec![0_u8; num_bytes];
    getrandom::fill(&mut bytes).expect("operating system CSPRNG must be available");
    URL_SAFE_NO_PAD.encode(&bytes)
}

impl SpotifyError {
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

impl fmt::Display for SpotifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SpotifyError {}

#[derive(Debug, Deserialize)]
struct SpotifyTokenPayload {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: i64,
}

impl SpotifyTokenPayload {
    fn into_token_set(self, fallback_refresh_token: Option<&str>, now: i64) -> SpotifyTokenSet {
        let expires_in = if self.expires_in > 0 {
            self.expires_in
        } else {
            3600
        };
        SpotifyTokenSet {
            access_token: self.access_token,
            refresh_token: self
                .refresh_token
                .or_else(|| fallback_refresh_token.map(str::to_string))
                .unwrap_or_default(),
            expires_at: now + expires_in,
            expires_in,
        }
    }
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(str::to_string)
}

fn encode_path_segment(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}
