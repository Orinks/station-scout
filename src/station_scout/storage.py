from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

import keyring
from keyring.errors import KeyringError, PasswordDeleteError

from station_scout.models import Station, TuneTimer

KEYRING_SERVICE = "Station Scout"
SECRET_FIELDS = (
    "lastfm_session_key",
    "lastfm_pending_token",
    "spotify_access_token",
    "spotify_refresh_token",
    "spotify_code_verifier",
)


@dataclass(slots=True)
class AppState:
    favorites: list[Station] = field(default_factory=list)
    recents: list[Station] = field(default_factory=list)
    timers: list[TuneTimer] = field(default_factory=list)
    volume: float = 1.0
    track_log_folder: str = ""
    lastfm_enabled: bool = False
    lastfm_scrobble_enabled: bool = True
    lastfm_username: str = ""
    lastfm_session_key: str = ""
    lastfm_pending_token: str = ""
    spotify_enabled: bool = False
    spotify_access_token: str = ""
    spotify_refresh_token: str = ""
    spotify_token_expires_at: int = 0
    spotify_auth_state: str = ""
    spotify_code_verifier: str = ""


class CredentialStore:
    def __init__(self, *, service_name: str = KEYRING_SERVICE) -> None:
        self.service_name = service_name

    def get(self, name: str) -> str:
        try:
            return keyring.get_password(self.service_name, name) or ""
        except KeyringError:
            return ""

    def set(self, name: str, value: str) -> None:
        try:
            if value:
                keyring.set_password(self.service_name, name, value)
            else:
                self.delete(name)
        except KeyringError:
            return

    def delete(self, name: str) -> None:
        try:
            keyring.delete_password(self.service_name, name)
        except (KeyringError, PasswordDeleteError):
            return


class SettingsStore:
    def __init__(self, path: Path | None = None, credentials: CredentialStore | None = None) -> None:
        self.path = path or Path.home() / "AppData" / "Roaming" / "Station Scout" / "station_scout.json"
        self.credentials = credentials or CredentialStore()

    def load(self) -> AppState:
        if not self.path.exists():
            return self._load_secrets(AppState())
        try:
            payload = json.loads(self.path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return self._load_secrets(AppState())

        state = AppState(
            favorites=_stations(payload.get("favorites")),
            recents=_stations(payload.get("recents")),
            timers=_timers(payload.get("timers")),
            volume=_volume(payload.get("volume")),
            track_log_folder=str(payload.get("track_log_folder") or ""),
            lastfm_enabled=bool(payload.get("lastfm_enabled", False)),
            lastfm_scrobble_enabled=bool(payload.get("lastfm_scrobble_enabled", True)),
            lastfm_username=str(payload.get("lastfm_username") or ""),
            spotify_enabled=bool(payload.get("spotify_enabled", False)),
            spotify_token_expires_at=int(payload.get("spotify_token_expires_at") or 0),
            spotify_auth_state=str(payload.get("spotify_auth_state") or ""),
        )
        migrated = self._migrate_plaintext_secrets(payload)
        state = self._load_secrets(state)
        if migrated:
            self.save(state)
        return state

    def save(self, state: AppState) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._save_secrets(state)
        payload = {
            "favorites": [station.to_json() for station in state.favorites],
            "recents": [station.to_json() for station in state.recents],
            "timers": [timer.to_json() for timer in state.timers],
            "volume": _volume(state.volume),
            "track_log_folder": state.track_log_folder,
            "lastfm_enabled": state.lastfm_enabled,
            "lastfm_scrobble_enabled": state.lastfm_scrobble_enabled,
            "lastfm_username": state.lastfm_username,
            "spotify_enabled": state.spotify_enabled,
            "spotify_token_expires_at": state.spotify_token_expires_at,
            "spotify_auth_state": state.spotify_auth_state,
        }
        self.path.write_text(json.dumps(payload, indent=2), encoding="utf-8")

    def default_track_log_folder(self) -> Path:
        return self.path.parent / "track sessions"

    def track_log_folder(self, state: AppState) -> Path:
        return Path(state.track_log_folder).expanduser() if state.track_log_folder else self.default_track_log_folder()

    def _load_secrets(self, state: AppState) -> AppState:
        for field_name in SECRET_FIELDS:
            setattr(state, field_name, self.credentials.get(field_name))
        return state

    def _save_secrets(self, state: AppState) -> None:
        for field_name in SECRET_FIELDS:
            self.credentials.set(field_name, getattr(state, field_name))

    def _migrate_plaintext_secrets(self, payload: dict[str, object]) -> bool:
        migrated = False
        for field_name in SECRET_FIELDS:
            value = str(payload.get(field_name) or "")
            if value and not self.credentials.get(field_name):
                self.credentials.set(field_name, value)
                migrated = True
        return migrated


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


def _volume(value: object) -> float:
    try:
        return max(0.0, min(1.0, float(value)))
    except (TypeError, ValueError):
        return 1.0
