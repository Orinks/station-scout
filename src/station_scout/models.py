from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
from typing import Literal


BrowseFacetKind = Literal["genre", "location", "language"]


@dataclass(frozen=True, slots=True)
class BrowseFacet:
    kind: BrowseFacetKind
    name: str
    station_count: int = 0
    code: str = ""

    @classmethod
    def from_api(cls, kind: BrowseFacetKind, payload: dict[str, object]) -> "BrowseFacet":
        code_key = "iso_3166_1" if kind == "location" else "iso_639"
        return cls(
            kind=kind,
            name=str(payload.get("name") or ""),
            station_count=_int(payload.get("stationcount")),
            code=str(payload.get(code_key) or ""),
        )

    def query_value(self) -> str:
        if self.kind == "location" and self.code:
            return self.code
        return self.name


@dataclass(frozen=True, slots=True)
class Station:
    stationuuid: str
    name: str
    url_resolved: str
    homepage: str = ""
    favicon: str = ""
    tags: str = ""
    country: str = ""
    countrycode: str = ""
    state: str = ""
    language: str = ""
    codec: str = ""
    bitrate: int = 0
    hls: int = 0
    lastcheckok: int = 0
    votes: int = 0
    clickcount: int = 0
    clicktrend: int = 0
    source: str = "Radio Browser"

    @classmethod
    def from_api(cls, payload: dict[str, object]) -> "Station":
        return cls(
            stationuuid=str(payload.get("stationuuid") or ""),
            name=str(payload.get("name") or "Unknown station"),
            url_resolved=str(payload.get("url_resolved") or payload.get("url") or ""),
            homepage=str(payload.get("homepage") or ""),
            favicon=str(payload.get("favicon") or ""),
            tags=str(payload.get("tags") or ""),
            country=str(payload.get("country") or ""),
            countrycode=str(payload.get("countrycode") or ""),
            state=str(payload.get("state") or ""),
            language=str(payload.get("language") or ""),
            codec=str(payload.get("codec") or ""),
            bitrate=_int(payload.get("bitrate")),
            hls=_int(payload.get("hls")),
            lastcheckok=_int(payload.get("lastcheckok")),
            votes=_int(payload.get("votes")),
            clickcount=_int(payload.get("clickcount")),
            clicktrend=_int(payload.get("clicktrend")),
            source=str(payload.get("source") or "Radio Browser"),
        )

    @classmethod
    def direct_stream(
        cls,
        *,
        name: str,
        url: str,
        homepage: str = "",
        source: str = "StreamURL.link",
    ) -> "Station":
        digest = sha256(f"{source}\0{name}\0{url}".encode("utf-8")).hexdigest()[:24]
        return cls(
            stationuuid=f"direct-{digest}",
            name=name,
            url_resolved=url,
            homepage=homepage,
            source=source,
            lastcheckok=1,
        )

    def subtitle(self) -> str:
        parts = [part for part in (self.country, self.language, self.tags) if part]
        return " | ".join(parts[:3])

    def quality_label(self) -> str:
        bits = []
        if self.lastcheckok:
            bits.append("working")
        if self.codec:
            bits.append(self.codec)
        if self.bitrate:
            bits.append(f"{self.bitrate} kbps")
        if self.hls:
            bits.append("HLS")
        if self.source != "Radio Browser":
            bits.append(self.source)
        return ", ".join(bits) or "stream details unavailable"

    def discovery_quality_score(self) -> int:
        score = 0
        if self.lastcheckok:
            score += 30

        if self.bitrate >= 320:
            score += 30
        elif self.bitrate >= 192:
            score += 25
        elif self.bitrate >= 128:
            score += 20
        elif self.bitrate >= 64:
            score += 10
        elif self.bitrate > 0:
            score += 5

        codec = self.codec.casefold().replace(" ", "")
        if codec in {"flac", "alac"}:
            score += 20
        elif codec in {"opus", "aac", "aac+", "heaac", "vorbis", "ogg"}:
            score += 15
        elif codec == "mp3":
            score += 10
        elif codec:
            score += 5

        if self.hls:
            score += 5
        if self.clicktrend > 0:
            score += min(15, self.clicktrend * 3)
        elif self.clickcount > 0:
            score += min(10, self.clickcount // 100)
        if self.votes > 0:
            score += min(5, self.votes // 100)
        return min(score, 100)

    def discovery_quality_label(self) -> str:
        score = self.discovery_quality_score()
        if score >= 80:
            tier = "excellent"
        elif score >= 60:
            tier = "good"
        elif score >= 40:
            tier = "fair"
        else:
            tier = "limited"
        return f"{tier} discovery match, {score}/100, {self.quality_label()}"

    def to_json(self) -> dict[str, object]:
        return {
            "stationuuid": self.stationuuid,
            "name": self.name,
            "url_resolved": self.url_resolved,
            "homepage": self.homepage,
            "favicon": self.favicon,
            "tags": self.tags,
            "country": self.country,
            "countrycode": self.countrycode,
            "state": self.state,
            "language": self.language,
            "codec": self.codec,
            "bitrate": self.bitrate,
            "hls": self.hls,
            "lastcheckok": self.lastcheckok,
            "votes": self.votes,
            "clickcount": self.clickcount,
            "clicktrend": self.clicktrend,
            "source": self.source,
        }


@dataclass(frozen=True, slots=True)
class TuneTimer:
    stationuuid: str
    station_name: str
    time: str
    enabled: bool = True
    auto_play: bool = True
    end_time: str = ""
    show_name: str = ""
    track_playlist: bool = False

    def to_json(self) -> dict[str, object]:
        return {
            "stationuuid": self.stationuuid,
            "station_name": self.station_name,
            "time": self.time,
            "enabled": self.enabled,
            "auto_play": self.auto_play,
            "end_time": self.end_time,
            "show_name": self.show_name,
            "track_playlist": self.track_playlist,
        }

    @classmethod
    def from_json(cls, payload: dict[str, object]) -> "TuneTimer":
        return cls(
            stationuuid=str(payload.get("stationuuid") or ""),
            station_name=str(payload.get("station_name") or "Unknown station"),
            time=str(payload.get("time") or "09:00"),
            enabled=bool(payload.get("enabled", True)),
            auto_play=bool(payload.get("auto_play", True)),
            end_time=str(payload.get("end_time") or ""),
            show_name=str(payload.get("show_name") or ""),
            track_playlist=bool(payload.get("track_playlist", False)),
        )


def _int(value: object) -> int:
    try:
        return int(value or 0)
    except (TypeError, ValueError):
        return 0
