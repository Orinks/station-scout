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
        )

    def save(self, state: AppState) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "favorites": [station.to_json() for station in state.favorites],
            "recents": [station.to_json() for station in state.recents],
            "timers": [timer.to_json() for timer in state.timers],
        }
        self.path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


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

