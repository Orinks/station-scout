from pathlib import Path

from station_scout.models import Station, TuneTimer
from station_scout.storage import AppState, SettingsStore, add_unique_station


def test_store_round_trips_state(tmp_path: Path) -> None:
    station = Station(stationuuid="abc", name="Example", url_resolved="https://example.test")
    state = AppState(
        favorites=[station],
        recents=[station],
        timers=[TuneTimer("abc", "Example", "18:00")],
        volume=0.35,
        track_log_folder=str(tmp_path / "logs"),
        lastfm_enabled=True,
        lastfm_scrobble_enabled=False,
        lastfm_username="listener",
        lastfm_session_key="session",
        spotify_enabled=True,
        spotify_access_token="access",
        spotify_refresh_token="refresh",
        spotify_token_expires_at=123,
    )
    credentials = FakeCredentialStore()
    store = SettingsStore(tmp_path / "settings.json", credentials=credentials)

    store.save(state)
    loaded = store.load()

    assert loaded.favorites == [station]
    assert loaded.recents == [station]
    assert loaded.timers == [TuneTimer("abc", "Example", "18:00")]
    assert loaded.volume == 0.35
    assert loaded.track_log_folder == str(tmp_path / "logs")
    assert loaded.lastfm_enabled
    assert not loaded.lastfm_scrobble_enabled
    assert loaded.lastfm_username == "listener"
    assert loaded.lastfm_session_key == "session"
    assert loaded.spotify_enabled
    assert loaded.spotify_access_token == "access"
    assert loaded.spotify_refresh_token == "refresh"
    assert credentials.values["lastfm_session_key"] == "session"

    settings_text = store.path.read_text(encoding="utf-8")
    assert "session" not in settings_text
    assert "access" not in settings_text
    assert "refresh" not in settings_text


def test_store_migrates_plaintext_secrets_to_keyring(tmp_path: Path) -> None:
    settings_path = tmp_path / "settings.json"
    settings_path.write_text(
        """
        {
          "lastfm_enabled": true,
          "lastfm_session_key": "old-session",
          "lastfm_pending_token": "old-token",
          "spotify_enabled": true,
          "spotify_access_token": "old-access",
          "spotify_refresh_token": "old-refresh",
          "spotify_code_verifier": "old-verifier"
        }
        """,
        encoding="utf-8",
    )
    credentials = FakeCredentialStore()
    store = SettingsStore(settings_path, credentials=credentials)

    loaded = store.load()

    assert loaded.lastfm_session_key == "old-session"
    assert loaded.spotify_refresh_token == "old-refresh"
    assert credentials.values["spotify_code_verifier"] == "old-verifier"

    settings_text = settings_path.read_text(encoding="utf-8")
    assert "old-session" not in settings_text
    assert "old-refresh" not in settings_text


def test_add_unique_station_moves_existing_to_front() -> None:
    first = Station(stationuuid="1", name="One", url_resolved="https://one.test")
    second = Station(stationuuid="2", name="Two", url_resolved="https://two.test")

    assert add_unique_station([first, second], second) == [second, first]


def test_track_log_folder_defaults_next_to_settings_file(tmp_path: Path) -> None:
    store = SettingsStore(tmp_path / "settings.json")

    assert store.track_log_folder(AppState()) == tmp_path / "track sessions"


def test_lastfm_scrobbling_defaults_on_for_loaded_state(tmp_path: Path) -> None:
    settings_path = tmp_path / "settings.json"
    settings_path.write_text('{"lastfm_enabled": true}', encoding="utf-8")
    store = SettingsStore(settings_path, credentials=FakeCredentialStore())

    loaded = store.load()

    assert loaded.lastfm_scrobble_enabled


def test_volume_defaults_and_clamps_loaded_values(tmp_path: Path) -> None:
    settings_path = tmp_path / "settings.json"
    settings_path.write_text('{"volume": 1.75}', encoding="utf-8")
    store = SettingsStore(settings_path, credentials=FakeCredentialStore())

    assert store.load().volume == 1.0

    settings_path.write_text('{"volume": -0.25}', encoding="utf-8")

    assert store.load().volume == 0.0


def test_volume_defaults_to_full_when_missing_or_invalid(tmp_path: Path) -> None:
    settings_path = tmp_path / "settings.json"
    settings_path.write_text("{}", encoding="utf-8")
    store = SettingsStore(settings_path, credentials=FakeCredentialStore())

    assert store.load().volume == 1.0

    settings_path.write_text('{"volume": "loud"}', encoding="utf-8")

    assert store.load().volume == 1.0


class FakeCredentialStore:
    def __init__(self) -> None:
        self.values: dict[str, str] = {}

    def get(self, name: str) -> str:
        return self.values.get(name, "")

    def set(self, name: str, value: str) -> None:
        if value:
            self.values[name] = value
        else:
            self.delete(name)

    def delete(self, name: str) -> None:
        self.values.pop(name, None)
