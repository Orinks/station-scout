use crate::models::{Station, TuneTimer};
use crate::notifications::QuietHours;
use keyring_core::{Entry, Error as KeyringBackendError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const SECRET_FIELDS: &[&str] = &[
    "lastfm_session_key",
    "lastfm_pending_token",
    "spotify_access_token",
    "spotify_refresh_token",
    "spotify_code_verifier",
    "podcast_index_key",
    "podcast_index_secret",
    "openrouter",
];
pub const KEYRING_SERVICE: &str = "Station Scout";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppState {
    #[serde(default)]
    pub favorites: Vec<Station>,
    #[serde(default)]
    pub recents: Vec<Station>,
    #[serde(default)]
    pub timers: Vec<TuneTimer>,
    #[serde(default = "default_volume")]
    pub volume: f64,
    #[serde(default)]
    pub track_log_folder: String,
    #[serde(default)]
    pub lastfm_enabled: bool,
    #[serde(default = "default_lastfm_scrobble")]
    pub lastfm_scrobble_enabled: bool,
    #[serde(default)]
    pub lastfm_username: String,
    #[serde(skip)]
    pub lastfm_session_key: String,
    #[serde(skip)]
    pub lastfm_pending_token: String,
    #[serde(default)]
    pub spotify_enabled: bool,
    #[serde(skip)]
    pub spotify_access_token: String,
    #[serde(skip)]
    pub spotify_refresh_token: String,
    #[serde(default)]
    pub spotify_token_expires_at: i64,
    #[serde(default)]
    pub spotify_auth_state: String,
    #[serde(skip)]
    pub spotify_code_verifier: String,
    #[serde(skip)]
    pub podcast_index_api_key: String,
    #[serde(skip)]
    pub podcast_index_api_secret: String,
    #[serde(skip)]
    pub openrouter_api_key: String,
    #[serde(default)]
    pub podcast: PodcastSettings,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PodcastSettings {
    #[serde(default = "default_podcast_polling_interval")]
    pub polling_interval_minutes: i32,
    #[serde(default)]
    pub quiet_hours: QuietHours,
    #[serde(default)]
    pub start_with_windows: bool,
    #[serde(default = "default_playback_speed")]
    pub playback_speed: f64,
    #[serde(default = "default_sleep_timer_minutes")]
    pub sleep_timer_default_minutes: i32,
    #[serde(default)]
    pub openrouter_model: String,
    #[serde(default)]
    pub first_run_completed: bool,
    #[serde(default = "default_schema_version")]
    pub schema_version: i32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            favorites: Vec::new(),
            recents: Vec::new(),
            timers: Vec::new(),
            volume: 1.0,
            track_log_folder: String::new(),
            lastfm_enabled: false,
            lastfm_scrobble_enabled: true,
            lastfm_username: String::new(),
            lastfm_session_key: String::new(),
            lastfm_pending_token: String::new(),
            spotify_enabled: false,
            spotify_access_token: String::new(),
            spotify_refresh_token: String::new(),
            spotify_token_expires_at: 0,
            spotify_auth_state: String::new(),
            spotify_code_verifier: String::new(),
            podcast_index_api_key: String::new(),
            podcast_index_api_secret: String::new(),
            openrouter_api_key: String::new(),
            podcast: PodcastSettings::default(),
        }
    }
}

impl Default for PodcastSettings {
    fn default() -> Self {
        Self {
            polling_interval_minutes: default_podcast_polling_interval(),
            quiet_hours: QuietHours::default(),
            start_with_windows: false,
            playback_speed: default_playback_speed(),
            sleep_timer_default_minutes: default_sleep_timer_minutes(),
            openrouter_model: String::new(),
            first_run_completed: false,
            schema_version: default_schema_version(),
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    credentials: Box<dyn CredentialStore>,
}

pub trait CredentialStore: Send + Sync {
    fn get(&self, name: &str) -> String;
    fn set(&self, name: &str, value: &str);
    fn delete(&self, name: &str);
}

#[derive(Clone, Default)]
pub struct MemoryCredentialStore {
    values: Arc<Mutex<HashMap<String, String>>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyringCredentialStore {
    service_name: String,
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            credentials: Box::new(NoopCredentialStore),
        }
    }

    pub fn default_path() -> PathBuf {
        let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from);
        station_scout_settings_path(appdata.as_deref(), home.as_deref())
    }

    pub fn default_track_log_folder(&self) -> PathBuf {
        self.path
            .parent()
            .map(|parent| parent.join("track sessions"))
            .unwrap_or_else(|| PathBuf::from("track sessions"))
    }

    pub fn track_log_folder(&self, state: &AppState) -> PathBuf {
        if state.track_log_folder.trim().is_empty() {
            return self.default_track_log_folder();
        }
        PathBuf::from(state.track_log_folder.trim())
    }

    pub fn with_credentials(mut self, credentials: Box<dyn CredentialStore>) -> Self {
        self.credentials = credentials;
        self
    }

    pub fn load(&self) -> Result<AppState, String> {
        if !self.path.exists() {
            return Ok(self.load_secrets(AppState::default()));
        }
        let text = fs::read_to_string(&self.path).map_err(|exc| exc.to_string())?;
        let payload = serde_json::from_str::<Value>(&text).ok();
        let Ok(mut state) = serde_json::from_str::<AppState>(&text) else {
            return Ok(self.load_secrets(AppState::default()));
        };
        state.volume = clamp_volume(state.volume);
        let migrated = payload
            .as_ref()
            .and_then(Value::as_object)
            .is_some_and(|object| self.migrate_plaintext_secrets(object));
        state = self.load_secrets(state);
        if migrated {
            self.save(&state)?;
        }
        Ok(state)
    }

    pub fn save(&self, state: &AppState) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|exc| exc.to_string())?;
        }
        self.save_secrets(state);
        let mut persisted = state.clone();
        persisted.volume = clamp_volume(persisted.volume);
        let text = serde_json::to_string_pretty(&persisted).map_err(|exc| exc.to_string())?;
        fs::write(&self.path, text).map_err(|exc| exc.to_string())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn load_secrets(&self, mut state: AppState) -> AppState {
        state.lastfm_session_key = self.credentials.get("lastfm_session_key");
        state.lastfm_pending_token = self.credentials.get("lastfm_pending_token");
        state.spotify_access_token = self.credentials.get("spotify_access_token");
        state.spotify_refresh_token = self.credentials.get("spotify_refresh_token");
        state.spotify_code_verifier = self.credentials.get("spotify_code_verifier");
        state.podcast_index_api_key = self.credentials.get("podcast_index_key");
        state.podcast_index_api_secret = self.credentials.get("podcast_index_secret");
        state.openrouter_api_key = self.credentials.get("openrouter");
        state
    }

    fn save_secrets(&self, state: &AppState) {
        self.credentials
            .set("lastfm_session_key", &state.lastfm_session_key);
        self.credentials
            .set("lastfm_pending_token", &state.lastfm_pending_token);
        self.credentials
            .set("spotify_access_token", &state.spotify_access_token);
        self.credentials
            .set("spotify_refresh_token", &state.spotify_refresh_token);
        self.credentials
            .set("spotify_code_verifier", &state.spotify_code_verifier);
        self.credentials
            .set("podcast_index_key", &state.podcast_index_api_key);
        self.credentials
            .set("podcast_index_secret", &state.podcast_index_api_secret);
        self.credentials
            .set("openrouter", &state.openrouter_api_key);
    }

    fn migrate_plaintext_secrets(&self, payload: &serde_json::Map<String, Value>) -> bool {
        let mut migrated = false;
        for field in SECRET_FIELDS {
            let value = payload
                .get(*field)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if !value.is_empty() && self.credentials.get(field).is_empty() {
                self.credentials.set(field, &value);
                migrated = true;
            }
        }
        migrated
    }
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self::new(Self::default_path())
    }
}

impl Default for KeyringCredentialStore {
    fn default() -> Self {
        Self::new(KEYRING_SERVICE)
    }
}

impl KeyringCredentialStore {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    fn entry(&self, name: &str) -> keyring_core::Result<Entry> {
        Entry::new(&self.service_name, name)
    }
}

impl CredentialStore for KeyringCredentialStore {
    fn get(&self, name: &str) -> String {
        match self.entry(name).and_then(|entry| entry.get_password()) {
            Ok(value) => value,
            Err(_) => String::new(),
        }
    }

    fn set(&self, name: &str, value: &str) {
        if value.is_empty() {
            self.delete(name);
            return;
        }
        if let Ok(entry) = self.entry(name) {
            let _ = entry.set_password(value);
        }
    }

    fn delete(&self, name: &str) {
        if let Ok(entry) = self.entry(name) {
            match entry.delete_credential() {
                Ok(()) | Err(KeyringBackendError::NoEntry) => {}
                Err(_) => {}
            }
        }
    }
}

pub fn system_credential_store() -> Box<dyn CredentialStore> {
    let _ = keyring::use_native_store(true);
    Box::new(KeyringCredentialStore::default())
}

pub fn station_scout_settings_path(appdata: Option<&Path>, home: Option<&Path>) -> PathBuf {
    let root = appdata
        .map(Path::to_path_buf)
        .or_else(|| home.map(|home| home.join("AppData").join("Roaming")))
        .unwrap_or_else(|| PathBuf::from("."));
    root.join("Station Scout").join("station_scout.json")
}

impl CredentialStore for MemoryCredentialStore {
    fn get(&self, name: &str) -> String {
        self.values
            .lock()
            .expect("credential store lock should not be poisoned")
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    fn set(&self, name: &str, value: &str) {
        if value.is_empty() {
            self.delete(name);
            return;
        }
        self.values
            .lock()
            .expect("credential store lock should not be poisoned")
            .insert(name.to_string(), value.to_string());
    }

    fn delete(&self, name: &str) {
        self.values
            .lock()
            .expect("credential store lock should not be poisoned")
            .remove(name);
    }
}

struct NoopCredentialStore;

impl CredentialStore for NoopCredentialStore {
    fn get(&self, _name: &str) -> String {
        String::new()
    }

    fn set(&self, _name: &str, _value: &str) {}

    fn delete(&self, _name: &str) {}
}

pub fn add_unique_station(
    stations: Vec<Station>,
    station: Station,
    limit: Option<usize>,
) -> Vec<Station> {
    let mut updated: Vec<Station> = stations
        .into_iter()
        .filter(|item| item.stationuuid != station.stationuuid)
        .collect();
    updated.insert(0, station);
    if let Some(limit) = limit {
        updated.truncate(limit);
    }
    updated
}

fn clamp_volume(value: f64) -> f64 {
    if !value.is_finite() {
        return 1.0;
    }
    value.clamp(0.0, 1.0)
}

fn default_volume() -> f64 {
    1.0
}

fn default_lastfm_scrobble() -> bool {
    true
}

fn default_podcast_polling_interval() -> i32 {
    30
}

fn default_playback_speed() -> f64 {
    1.0
}

fn default_sleep_timer_minutes() -> i32 {
    15
}

fn default_schema_version() -> i32 {
    1
}
