from __future__ import annotations

import os
from dataclasses import dataclass

DEFAULT_LASTFM_PROXY_URL = "https://station-scout-lastfm-proxy.vercel.app/api/lastfm"


@dataclass(frozen=True, slots=True)
class LastFmAppConfig:
    api_key: str = ""
    api_secret: str = ""
    proxy_url: str = ""


@dataclass(frozen=True, slots=True)
class SpotifyAppConfig:
    client_id: str
    redirect_uri: str = "http://127.0.0.1:8765/spotify/callback"


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
    client_id = os.environ.get("STATION_SCOUT_SPOTIFY_CLIENT_ID", "").strip()
    redirect_uri = os.environ.get(
        "STATION_SCOUT_SPOTIFY_REDIRECT_URI",
        "http://127.0.0.1:8765/spotify/callback",
    ).strip()
    if not client_id:
        return None
    return SpotifyAppConfig(client_id=client_id, redirect_uri=redirect_uri)
