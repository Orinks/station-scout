from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlparse, urlunparse

import requests

DEFAULT_LASTFM_PROXY_URL = "https://station-scout-lastfm-proxy.vercel.app/api/lastfm"
DEFAULT_SPOTIFY_CONFIG_URL = "https://station-scout-lastfm-proxy.vercel.app/api/spotify/config"


@dataclass(frozen=True, slots=True)
class LastFmAppConfig:
    api_key: str = ""
    api_secret: str = ""
    proxy_url: str = ""


@dataclass(frozen=True, slots=True)
class SpotifyAppConfig:
    client_id: str
    redirect_uri: str = "http://127.0.0.1/spotify/callback"


def lastfm_app_config() -> LastFmAppConfig | None:
    proxy_url = os.environ.get("STATION_SCOUT_LASTFM_PROXY_URL", DEFAULT_LASTFM_PROXY_URL).strip().rstrip("/")
    if proxy_url:
        return LastFmAppConfig(proxy_url=proxy_url)

    api_key = os.environ.get("STATION_SCOUT_LASTFM_API_KEY", "").strip()
    api_secret = os.environ.get("STATION_SCOUT_LASTFM_API_SECRET", "").strip()
    if not api_key or not api_secret:
        return None
    return LastFmAppConfig(api_key=api_key, api_secret=api_secret)


def spotify_app_config() -> SpotifyAppConfig | None:
    client_id = (
        os.environ.get("STATION_SCOUT_SPOTIFY_CLIENT_ID", "").strip()
        or _local_env_value("STATION_SCOUT_SPOTIFY_CLIENT_ID")
        or _local_env_value("Spotify client ID")
    )
    redirect_uri = os.environ.get(
        "STATION_SCOUT_SPOTIFY_REDIRECT_URI",
        "http://127.0.0.1/spotify/callback",
    ).strip()
    if client_id:
        return SpotifyAppConfig(client_id=client_id, redirect_uri=_spotify_loopback_redirect_base(redirect_uri))
    return _spotify_proxy_config()


def _spotify_proxy_config() -> SpotifyAppConfig | None:
    config_url = os.environ.get("STATION_SCOUT_SPOTIFY_CONFIG_URL", DEFAULT_SPOTIFY_CONFIG_URL).strip()
    if not config_url:
        return None
    try:
        response = requests.get(config_url, timeout=8)
        response.raise_for_status()
        payload = response.json()
    except (requests.RequestException, ValueError):
        return None
    client_id = str(payload.get("clientId") or "").strip()
    redirect_uri = str(payload.get("redirectUri") or "http://127.0.0.1/spotify/callback").strip()
    if not client_id:
        return None
    return SpotifyAppConfig(client_id=client_id, redirect_uri=_spotify_loopback_redirect_base(redirect_uri))


def _local_env_value(name: str) -> str:
    path = Path.home() / ".env.local"
    if not path.exists():
        return ""
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return ""
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith(f"{name}="):
            return stripped.split("=", 1)[1].strip().strip("\"'")
        if stripped.lower().startswith(f"{name.lower()}:"):
            return stripped.split(":", 1)[1].strip().strip("\"'")
        if stripped.lower() == name.lower() and index + 1 < len(lines):
            return lines[index + 1].strip().strip("\"'")
    return ""


def _spotify_loopback_redirect_base(redirect_uri: str) -> str:
    parsed = urlparse(redirect_uri)
    if parsed.scheme != "http" or parsed.hostname not in {"127.0.0.1", "::1", "localhost"}:
        return redirect_uri
    host = "127.0.0.1" if parsed.hostname == "localhost" else parsed.hostname
    netloc = f"[{host}]" if ":" in host else host
    return urlunparse((parsed.scheme, netloc, parsed.path or "/", "", "", ""))
