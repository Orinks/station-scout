from __future__ import annotations

import random
from collections.abc import Iterable

import requests

from station_scout.models import BrowseFacet, BrowseFacetKind, Station

DEFAULT_SERVERS = (
    "https://de1.api.radio-browser.info",
    "https://nl1.api.radio-browser.info",
)


class RadioBrowserError(RuntimeError):
    pass


class RadioBrowserClient:
    def __init__(
        self,
        base_urls: Iterable[str] = DEFAULT_SERVERS,
        *,
        timeout: float = 12.0,
        user_agent: str = "StationScout/0.1 (+https://github.com/Orinks/station-scout)",
    ) -> None:
        self.base_urls = tuple(base_urls)
        self.timeout = timeout
        self.session = requests.Session()
        self.session.headers.update({"User-Agent": user_agent})

    def search(
        self,
        *,
        name: str = "",
        country: str = "",
        language: str = "",
        tag: str = "",
        limit: int = 50,
    ) -> list[Station]:
        params: dict[str, object] = {
            "hidebroken": "true",
            "limit": max(1, min(limit, 100)),
            "order": "clicktrend",
            "reverse": "true",
        }
        if name:
            params["name"] = name
        if len(country) == 2 and country.isalpha():
            params["countrycode"] = country.upper()
        elif country:
            params["country"] = country
        if language:
            params["language"] = language
        if tag:
            params["tag"] = tag

        rows = self._get_json("/json/stations/search", params=params)
        return [Station.from_api(row) for row in rows if isinstance(row, dict)]

    def browse_facets(self, kind: BrowseFacetKind, *, limit: int = 50) -> list[BrowseFacet]:
        paths = {
            "genre": "/json/tags",
            "location": "/json/countries",
            "language": "/json/languages",
        }
        params: dict[str, object] = {
            "hidebroken": "true",
            "limit": max(1, min(limit, 500)),
            "order": "stationcount",
            "reverse": "true",
        }
        rows = self._get_json(paths[kind], params=params)
        if not isinstance(rows, list):
            return []
        facets = [
            facet
            for row in rows
            if isinstance(row, dict)
            for facet in [self._clean_facet(BrowseFacet.from_api(kind, row))]
            if facet is not None
        ]
        return facets[: params["limit"]]

    def browse_stations(
        self,
        facet: BrowseFacet,
        *,
        limit: int = 50,
    ) -> list[Station]:
        match facet.kind:
            case "genre":
                return discovery_sorted(self.search(tag=facet.query_value(), limit=limit))
            case "location":
                return discovery_sorted(self.search(country=facet.query_value(), limit=limit))
            case "language":
                return discovery_sorted(self.search(language=facet.query_value(), limit=limit))

    def click_url(self, stationuuid: str) -> str:
        payload = self._get_json(f"/json/url/{stationuuid}")
        if not isinstance(payload, dict) or str(payload.get("ok")).lower() != "true":
            raise RadioBrowserError("Radio Browser did not return a playable URL.")
        return str(payload.get("url") or "")

    def _clean_facet(self, facet: BrowseFacet) -> BrowseFacet | None:
        name = " ".join(facet.name.strip().strip("\"'").split())
        if not name or name.startswith("#") or not any(char.isalnum() for char in name):
            return None
        code = facet.code.strip().upper() if facet.kind == "location" else facet.code.strip()
        return BrowseFacet(
            kind=facet.kind,
            name=name,
            station_count=facet.station_count,
            code=code,
        )

    def _get_json(self, path: str, *, params: dict[str, object] | None = None) -> object:
        errors: list[str] = []
        for base_url in random.sample(self.base_urls, k=len(self.base_urls)):
            url = f"{base_url.rstrip('/')}{path}"
            try:
                response = self.session.get(url, params=params, timeout=self.timeout)
                response.raise_for_status()
                return response.json()
            except (requests.RequestException, ValueError) as exc:
                errors.append(f"{url}: {exc}")
        raise RadioBrowserError("; ".join(errors) or "Radio Browser request failed.")


def discovery_sorted(stations: Iterable[Station]) -> list[Station]:
    return sorted(
        stations,
        key=lambda station: (
            -station.discovery_quality_score(),
            -station.bitrate,
            -station.clicktrend,
            -station.clickcount,
            station.name.casefold(),
        ),
    )
