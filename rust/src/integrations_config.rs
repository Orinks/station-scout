use serde_json::Value;
use std::path::Path;
use url::Url;

pub const DEFAULT_LASTFM_PROXY_URL: &str =
    "https://station-scout-lastfm-proxy.vercel.app/api/lastfm";
pub const DEFAULT_SPOTIFY_CONFIG_URL: &str =
    "https://station-scout-lastfm-proxy.vercel.app/api/spotify/config";
pub const DEFAULT_SPOTIFY_REDIRECT_URI: &str = "http://127.0.0.1/spotify/callback";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LastFmAppConfig {
    pub api_key: String,
    pub api_secret: String,
    pub proxy_url: String,
}

impl LastFmAppConfig {
    pub fn proxy(proxy_url: &str) -> Self {
        Self {
            api_key: String::new(),
            api_secret: String::new(),
            proxy_url: proxy_url.trim().trim_end_matches('/').to_string(),
        }
    }

    pub fn direct(api_key: &str, api_secret: &str) -> Self {
        Self {
            api_key: api_key.trim().to_string(),
            api_secret: api_secret.trim().to_string(),
            proxy_url: String::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpotifyAppConfig {
    pub client_id: String,
    pub redirect_uri: String,
}

pub fn lastfm_app_config() -> Option<LastFmAppConfig> {
    lastfm_app_config_from_values(
        std::env::var("STATION_SCOUT_LASTFM_PROXY_URL")
            .ok()
            .as_deref(),
        &std::env::var("STATION_SCOUT_LASTFM_API_KEY").unwrap_or_default(),
        &std::env::var("STATION_SCOUT_LASTFM_API_SECRET").unwrap_or_default(),
    )
}

pub fn lastfm_app_config_from_values(
    proxy_url: Option<&str>,
    api_key: &str,
    api_secret: &str,
) -> Option<LastFmAppConfig> {
    let proxy_url = proxy_url.unwrap_or(DEFAULT_LASTFM_PROXY_URL);
    let proxy_url = proxy_url.trim().trim_end_matches('/');
    if !proxy_url.is_empty() {
        return Some(LastFmAppConfig::proxy(proxy_url));
    }
    let api_key = api_key.trim();
    let api_secret = api_secret.trim();
    if api_key.is_empty() || api_secret.is_empty() {
        return None;
    }
    Some(LastFmAppConfig::direct(api_key, api_secret))
}

pub fn spotify_app_config() -> Option<SpotifyAppConfig> {
    let home_env = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|home| Path::new(&home).join(".env.local"));
    let local_client_id = home_env
        .as_deref()
        .map(|path| {
            let value = local_env_value(path, "STATION_SCOUT_SPOTIFY_CLIENT_ID");
            if value.is_empty() {
                local_env_value(path, "Spotify client ID")
            } else {
                value
            }
        })
        .unwrap_or_default();
    let env_client_id = std::env::var("STATION_SCOUT_SPOTIFY_CLIENT_ID").unwrap_or_default();
    let client_id = if env_client_id.trim().is_empty() {
        local_client_id.as_str()
    } else {
        env_client_id.as_str()
    };
    if let Some(config) = spotify_app_config_from_values(
        client_id,
        std::env::var("STATION_SCOUT_SPOTIFY_REDIRECT_URI")
            .ok()
            .as_deref(),
    ) {
        return Some(config);
    }
    let config_url = std::env::var("STATION_SCOUT_SPOTIFY_CONFIG_URL")
        .unwrap_or_else(|_| DEFAULT_SPOTIFY_CONFIG_URL.to_string());
    spotify_proxy_config(&config_url)
}

pub fn spotify_app_config_from_values(
    client_id: &str,
    redirect_uri: Option<&str>,
) -> Option<SpotifyAppConfig> {
    let client_id = client_id.trim();
    if client_id.is_empty() {
        return None;
    }
    let redirect_uri = redirect_uri
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_SPOTIFY_REDIRECT_URI);
    Some(SpotifyAppConfig {
        client_id: client_id.to_string(),
        redirect_uri: spotify_loopback_redirect_base(redirect_uri),
    })
}

pub fn spotify_proxy_config(config_url: &str) -> Option<SpotifyAppConfig> {
    let config_url = config_url.trim();
    if config_url.is_empty() {
        return None;
    }
    let response = reqwest::blocking::Client::new()
        .get(config_url)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .ok()?;
    let payload: Value = serde_json::from_str(&response).ok()?;
    let client_id = payload
        .get("clientId")
        .and_then(Value::as_str)
        .unwrap_or("");
    let redirect_uri = payload.get("redirectUri").and_then(Value::as_str);
    spotify_app_config_from_values(client_id, redirect_uri)
}

pub fn local_env_value(path: &Path, name: &str) -> String {
    let Ok(text) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let lines = text.lines().collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        let stripped = line.trim();
        if let Some(value) = stripped.strip_prefix(&format!("{name}=")) {
            return trim_env_value(value);
        }
        if stripped
            .to_ascii_lowercase()
            .starts_with(&format!("{}:", name.to_ascii_lowercase()))
        {
            if let Some((_, value)) = stripped.split_once(':') {
                return trim_env_value(value);
            }
        }
        if stripped.eq_ignore_ascii_case(name) && index + 1 < lines.len() {
            return trim_env_value(lines[index + 1]);
        }
    }
    String::new()
}

pub fn spotify_loopback_redirect_base(redirect_uri: &str) -> String {
    let Ok(parsed) = Url::parse(redirect_uri) else {
        return redirect_uri.to_string();
    };
    let host = parsed.host_str().unwrap_or_default();
    if parsed.scheme() != "http" || !matches!(host, "127.0.0.1" | "::1" | "localhost") {
        return redirect_uri.to_string();
    }
    let normalized_host = if host == "localhost" {
        "127.0.0.1"
    } else {
        host
    };
    let netloc = if normalized_host.contains(':') {
        format!("[{normalized_host}]")
    } else {
        normalized_host.to_string()
    };
    format!("http://{}{}", netloc, parsed.path())
}

fn trim_env_value(value: &str) -> String {
    value.trim().trim_matches(['"', '\'']).to_string()
}
