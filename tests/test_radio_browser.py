from __future__ import annotations

from station_scout.models import BrowseFacet, Station
from station_scout.radio_browser import RadioBrowserClient, discovery_sorted


class FakeResponse:
    def __init__(self, payload: object) -> None:
        self.payload = payload

    def raise_for_status(self) -> None:
        return None

    def json(self) -> object:
        return self.payload


class FakeSession:
    def __init__(self, payload: object | None = None) -> None:
        self.headers: dict[str, str] = {}
        self.calls: list[tuple[str, dict[str, object] | None]] = []
        self.payload = payload or [{"stationuuid": "abc", "name": "Example", "url": "https://stream.test"}]

    def get(self, url: str, *, params: dict[str, object] | None = None, timeout: float = 0) -> FakeResponse:
        self.calls.append((url, params))
        return FakeResponse(self.payload)


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


def test_browse_facets_loads_clean_count_ordered_genres() -> None:
    client = RadioBrowserClient(base_urls=["https://example.test"])
    fake = FakeSession(
        [
            {"name": " jazz ", "stationcount": "120"},
            {"name": "#english", "stationcount": 15},
            {"name": "\"classic rock\"", "stationcount": "80"},
            {"name": "!!!", "stationcount": 1},
        ]
    )
    client.session = fake  # type: ignore[assignment]

    facets = client.browse_facets("genre", limit=25)

    assert facets == [
        BrowseFacet(kind="genre", name="jazz", station_count=120),
        BrowseFacet(kind="genre", name="classic rock", station_count=80),
    ]
    assert fake.calls == [
        (
            "https://example.test/json/tags",
            {
                "hidebroken": "true",
                "limit": 25,
                "order": "stationcount",
                "reverse": "true",
            },
        )
    ]


def test_browse_facets_preserves_country_codes_for_locations() -> None:
    client = RadioBrowserClient(base_urls=["https://example.test"])
    fake = FakeSession([{"name": "United States", "iso_3166_1": "us", "stationcount": "200"}])
    client.session = fake  # type: ignore[assignment]

    facets = client.browse_facets("location")

    assert facets == [
        BrowseFacet(kind="location", name="United States", station_count=200, code="US")
    ]


def test_browse_stations_maps_facets_to_existing_search() -> None:
    client = RadioBrowserClient(base_urls=["https://example.test"])
    fake = FakeSession()
    client.session = fake  # type: ignore[assignment]

    client.browse_stations(BrowseFacet(kind="genre", name="jazz"), limit=10)
    client.browse_stations(BrowseFacet(kind="location", name="United States", code="US"), limit=10)
    client.browse_stations(BrowseFacet(kind="language", name="english"), limit=10)

    assert fake.calls[0][1]["tag"] == "jazz"
    assert fake.calls[1][1]["countrycode"] == "US"
    assert fake.calls[2][1]["language"] == "english"


def test_discovery_sorted_prefers_quality_and_popularity_signals() -> None:
    stations = [
        Station.from_api(
            {
                "stationuuid": "weak",
                "name": "Weak",
                "url": "https://weak.test",
                "lastcheckok": 1,
                "bitrate": 64,
                "codec": "MP3",
                "clicktrend": 0,
            }
        ),
        Station.from_api(
            {
                "stationuuid": "strong",
                "name": "Strong",
                "url": "https://strong.test",
                "lastcheckok": 1,
                "bitrate": 192,
                "codec": "AAC",
                "clicktrend": 5,
            }
        ),
    ]

    assert discovery_sorted([]) == []
    sorted_names = [station.name for station in discovery_sorted(stations)]
    assert sorted_names == ["Strong", "Weak"]
