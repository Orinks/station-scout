from pathlib import Path

from station_scout.models import Station, TuneTimer
from station_scout.storage import AppState, SettingsStore, add_unique_station


def test_store_round_trips_state(tmp_path: Path) -> None:
    station = Station(stationuuid="abc", name="Example", url_resolved="https://example.test")
    state = AppState(
        favorites=[station],
        recents=[station],
        timers=[TuneTimer("abc", "Example", "18:00")],
        track_log_folder=str(tmp_path / "logs"),
        lastfm_api_key="key",
        lastfm_api_secret="secret",
        lastfm_session_key="session",
    )
    store = SettingsStore(tmp_path / "settings.json")

    store.save(state)
    loaded = store.load()

    assert loaded.favorites == [station]
    assert loaded.recents == [station]
    assert loaded.timers == [TuneTimer("abc", "Example", "18:00")]
    assert loaded.track_log_folder == str(tmp_path / "logs")
    assert loaded.lastfm_session_key == "session"


def test_add_unique_station_moves_existing_to_front() -> None:
    first = Station(stationuuid="1", name="One", url_resolved="https://one.test")
    second = Station(stationuuid="2", name="Two", url_resolved="https://two.test")

    assert add_unique_station([first, second], second) == [second, first]


def test_track_log_folder_defaults_next_to_settings_file(tmp_path: Path) -> None:
    store = SettingsStore(tmp_path / "settings.json")

    assert store.track_log_folder(AppState()) == tmp_path / "track sessions"
