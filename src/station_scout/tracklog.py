from __future__ import annotations

import datetime as dt
import re
import time
from dataclasses import dataclass
from pathlib import Path

import requests

from station_scout.models import Station


INVALID_TITLE_MARKERS = (
    "advert",
    "commercial",
    "promo",
    "sweeper",
    "jingle",
    "station id",
    "unknown",
)


@dataclass(frozen=True, slots=True)
class TrackEntry:
    artist: str
    title: str
    raw: str
    timestamp: dt.datetime
    uncertain: bool = False

    def display_line(self) -> str:
        line = f"{self.artist} - {self.title}" if self.artist and self.title else self.raw
        return f"{line} ?" if self.uncertain else line


def parse_stream_title(raw_title: str, *, now: dt.datetime | None = None) -> TrackEntry | None:
    raw = _clean(raw_title)
    if not raw:
        return None
    lowered = raw.lower()
    uncertain = any(marker in lowered for marker in INVALID_TITLE_MARKERS)

    artist = ""
    title = ""
    for separator in (" - ", " – ", " — ", " / "):
        if separator in raw:
            artist, title = [part.strip() for part in raw.split(separator, 1)]
            break

    if not artist or not title:
        match = re.match(r"(?P<title>.+?)\s+by\s+(?P<artist>.+)", raw, re.IGNORECASE)
        if match:
            artist = match.group("artist").strip()
            title = match.group("title").strip()

    if not artist or not title:
        uncertain = True
        artist = ""
        title = ""

    return TrackEntry(
        artist=artist,
        title=title,
        raw=raw,
        timestamp=now or dt.datetime.now(),
        uncertain=uncertain,
    )


class TrackSessionLog:
    def __init__(
        self,
        *,
        root: Path,
        station: Station,
        show_name: str = "",
        started_at: dt.datetime | None = None,
    ) -> None:
        self.station = station
        self.show_name = show_name.strip()
        self.started_at = started_at or dt.datetime.now()
        self.path = root / f"{_slug(self.show_name or station.name)}-{self.started_at:%Y-%m-%d-%H%M}.txt"
        self.entries: list[TrackEntry] = []
        self._seen: set[str] = set()

    def add(self, entry: TrackEntry) -> bool:
        key = entry.display_line().lower()
        if key in self._seen:
            return False
        self._seen.add(key)
        self.entries.append(entry)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.path.open("a", encoding="utf-8") as file:
            file.write(entry.display_line() + "\n")
        return True


class IcyMetadataReader:
    def __init__(self, *, timeout: float = 15.0) -> None:
        self.timeout = timeout

    def titles(self, url: str, *, stop_after: float | None = None):
        deadline = time.monotonic() + stop_after if stop_after else None
        with requests.get(
            url,
            headers={"Icy-MetaData": "1", "User-Agent": "StationScout/0.1"},
            stream=True,
            timeout=self.timeout,
        ) as response:
            response.raise_for_status()
            metaint = int(response.headers.get("icy-metaint") or 0)
            if metaint <= 0:
                return
            stream = response.raw
            while deadline is None or time.monotonic() < deadline:
                stream.read(metaint)
                length_byte = stream.read(1)
                if not length_byte:
                    return
                metadata_length = length_byte[0] * 16
                metadata = stream.read(metadata_length).decode("utf-8", errors="ignore")
                title = _extract_stream_title(metadata)
                if title:
                    yield title


def _extract_stream_title(metadata: str) -> str:
    match = re.search(r"StreamTitle='(?P<title>.*?)';", metadata)
    return match.group("title") if match else ""


def _clean(value: str) -> str:
    value = re.sub(r"\s+", " ", value.replace("\x00", " ")).strip()
    value = re.sub(r"^\d+\.\s*", "", value)
    return value.strip(" -")


def _slug(value: str) -> str:
    slug = re.sub(r"[^a-zA-Z0-9]+", "-", value).strip("-").lower()
    return slug or "station-scout-session"
