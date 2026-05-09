from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

from station_scout.models import Station, TuneTimer


@dataclass(slots=True)
class AppState:
    favorites: list[Station] = field(default_factory=list)
    recents: list[Station] = field(default_factory=list)
    timers: list[TuneTimer] = field(default_factory=list)
    track_log_folder: str = ""
    lastfm_enabled: bool = False
    lastfm_username: str = ""
    lastfm_session_key: str = ""
    lastfm_pending_token: str = ""
    spotify_enabled: bool = False
    spotify_access_token: str = ""
    spotify_refresh_token: str = ""
    spotify_token_expires_at: int = 0
    spotify_auth_state: str = ""
    spotify_code_verifier: str = ""


class SettingsStore:
    def __init__(self, path: Path | None = None) -> None:
        self.path = path or Path.home() / "AppData" / "Roaming" / "Station Scout" / "station_scout.json"

    def load(self) -> AppState:
        if not self.path.exists():
            return AppState()
        try:
            payload = json.loads(self.path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return AppState()

        return AppState(
            favorites=_stations(payload.get("favorites")),
            recents=_stations(payload.get("recents")),
            timers=_timers(payload.get("timers")),
            track_log_folder=str(payload.get("track_log_folder") or ""),
            lastfm_enabled=bool(payload.get("lastfm_enabled", False)),
            lastfm_username=str(payload.get("lastfm_username") or ""),
            lastfm_session_key=str(payload.get("lastfm_session_key") or ""),
            lastfm_pending_token=str(payload.get("lastfm_pending_token") or ""),
            spotify_enabled=bool(payload.get("spotify_enabled", False)),
            spotify_access_token=str(payload.get("spotify_access_token") or ""),
            spotify_refresh_token=str(payload.get("spotify_refresh_token") or ""),
            spotify_token_expires_at=int(payload.get("spotify_token_expires_at") or 0),
            spotify_auth_state=str(payload.get("spotify_auth_state") or ""),
            spotify_code_verifier=str(payload.get("spotify_code_verifier") or ""),
        )

    def save(self, state: AppState) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "favorites": [station.to_json() for station in state.favorites],
            "recents": [station.to_json() for station in state.recents],
            "timers": [timer.to_json() for timer in state.timers],
            "track_log_folder": state.track_log_folder,
            "lastfm_enabled": state.lastfm_enabled,
            "lastfm_username": state.lastfm_username,
            "lastfm_session_key": state.lastfm_session_key,
            "lastfm_pending_token": state.lastfm_pending_token,
            "spotify_enabled": state.spotify_enabled,
            "spotify_access_token": state.spotify_access_token,
            "spotify_refresh_token": state.spotify_refresh_token,
            "spotify_token_expires_at": state.spotify_token_expires_at,
            "spotify_auth_state": state.spotify_auth_state,
            "spotify_code_verifier": state.spotify_code_verifier,
        }
        self.path.write_text(json.dumps(payload, indent=2), encoding="utf-8")

    def default_track_log_folder(self) -> Path:
        return self.path.parent / "track sessions"

    def track_log_folder(self, state: AppState) -> Path:
        return Path(state.track_log_folder).expanduser() if state.track_log_folder else self.default_track_log_folder()


def add_unique_station(stations: list[Station], station: Station, *, limit: int | None = None) -> list[Station]:
    updated = [item for item in stations if item.stationuuid != station.stationuuid]
    updated.insert(0, station)
    return updated[:limit] if limit else updated


def _stations(value: object) -> list[Station]:
    if not isinstance(value, list):
        return []
    return [Station.from_api(item) for item in value if isinstance(item, dict)]


def _timers(value: object) -> list[TuneTimer]:
    if not isinstance(value, list):
        return []
    return [TuneTimer.from_json(item) for item in value if isinstance(item, dict)]
