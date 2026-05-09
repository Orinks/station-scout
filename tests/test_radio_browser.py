from __future__ import annotations

from station_scout.radio_browser import RadioBrowserClient


class FakeResponse:
    def __init__(self, payload: object) -> None:
        self.payload = payload

    def raise_for_status(self) -> None:
        return None

    def json(self) -> object:
        return self.payload


class FakeSession:
    def __init__(self) -> None:
        self.headers: dict[str, str] = {}
        self.calls: list[tuple[str, dict[str, object] | None]] = []

    def get(self, url: str, *, params: dict[str, object] | None = None, timeout: float = 0) -> FakeResponse:
        self.calls.append((url, params))
        return FakeResponse([{"stationuuid": "abc", "name": "Example", "url": "https://stream.test"}])


def test_search_sends_filters_and_hides_broken_stations() -> None:
    client = RadioBrowserClient(base_urls=["https://example.test"])
    fake = FakeSession()
    client.session = fake  # type: ignore[assignment]

    stations = client.search(name="jazz", country="US", language="english", tag="news", limit=25)

    assert stations[0].name == "Example"
    assert fake.calls == [
        (
            "https://example.test/json/stations/search",
            {
                "hidebroken": "true",
                "limit": 25,
                "order": "clicktrend",
                "reverse": "true",
                "name": "jazz",
                "countrycode": "US",
                "language": "english",
                "tag": "news",
            },
        )
    ]


def test_search_treats_two_letter_country_as_country_code() -> None:
    client = RadioBrowserClient(base_urls=["https://example.test"])
    fake = FakeSession()
    client.session = fake  # type: ignore[assignment]

    client.search(country="us")

    assert fake.calls[0][1]["countrycode"] == "US"
    assert "country" not in fake.calls[0][1]
