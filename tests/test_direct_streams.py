import pytest

from station_scout.direct_streams import station_from_direct_url, streamurl_search_url


def test_streamurl_search_url_uses_station_query() -> None:
    assert streamurl_search_url("BBC Radio 1") == "https://streamurl.link/s/BBC%20Radio%201/"


def test_station_from_direct_url_builds_playable_station() -> None:
    station = station_from_direct_url(name="Example FM", url="https://streams.example.org/live.m3u8")

    assert station.name == "Example FM"
    assert station.url_resolved == "https://streams.example.org/live.m3u8"
    assert station.source == "Direct stream URL"


def test_station_from_direct_url_rejects_plain_search_text() -> None:
    with pytest.raises(ValueError):
        station_from_direct_url(name="Example FM", url="Example FM")
