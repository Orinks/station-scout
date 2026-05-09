from pathlib import Path

from station_scout.models import Station, TuneTimer
from station_scout.storage import AppState, SettingsStore, add_unique_station


def test_store_round_trips_state(tmp_path: Path) -> None:
    station = Station(stationuuid="abc", name="Example", url_resolved="https://example.test")
    state = AppState(favorites=[station], recents=[station], timers=[TuneTimer("abc", "Example", "18:00")])
    store = SettingsStore(tmp_path / "settings.json")

    store.save(state)
    loaded = store.load()

    assert loaded.favorites == [station]
    assert loaded.recents == [station]
    assert loaded.timers == [TuneTimer("abc", "Example", "18:00")]


def test_add_unique_station_moves_existing_to_front() -> None:
    first = Station(stationuuid="1", name="One", url_resolved="https://one.test")
    second = Station(stationuuid="2", name="Two", url_resolved="https://two.test")

    assert add_unique_station([first, second], second) == [second, first]

