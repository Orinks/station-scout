from __future__ import annotations

import random
from collections.abc import Iterable

import requests

from station_scout.models import Station

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
        if country:
            params["country"] = country
        if language:
            params["language"] = language
        if tag:
            params["tag"] = tag

        rows = self._get_json("/json/stations/search", params=params)
        return [Station.from_api(row) for row in rows if isinstance(row, dict)]

    def click_url(self, stationuuid: str) -> str:
        payload = self._get_json(f"/json/url/{stationuuid}")
        if not isinstance(payload, dict) or str(payload.get("ok")).lower() != "true":
            raise RadioBrowserError("Radio Browser did not return a playable URL.")
        return str(payload.get("url") or "")

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

