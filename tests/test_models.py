from station_scout.models import BrowseFacet, Station, TuneTimer


def test_station_normalizes_api_payload() -> None:
    station = Station.from_api(
        {
            "stationuuid": "abc",
            "name": "Example",
            "url": "https://stream.example/live",
            "bitrate": "128",
            "lastcheckok": "1",
        }
    )

    assert station.stationuuid == "abc"
    assert station.url_resolved == "https://stream.example/live"
    assert station.bitrate == 128
    assert "working" in station.quality_label()


def test_timer_round_trips_json() -> None:
    timer = TuneTimer(
        stationuuid="abc",
        station_name="Example",
        time="07:30",
        end_time="10:30",
        show_name="Morning Show",
        track_playlist=True,
    )

    assert TuneTimer.from_json(timer.to_json()) == timer


def test_direct_stream_station_has_stable_id_and_source() -> None:
    station = Station.direct_stream(name="Example", url="https://stream.test/live")

    assert station.stationuuid.startswith("direct-")
    assert station.source == "StreamURL.link"
    assert "StreamURL.link" in station.quality_label()


def test_browse_facet_normalizes_api_payload_and_query_value() -> None:
    facet = BrowseFacet.from_api(
        "location",
        {"name": "United States", "stationcount": "123", "iso_3166_1": "US"},
    )

    assert facet.name == "United States"
    assert facet.station_count == 123
    assert facet.query_value() == "US"


def test_station_discovery_quality_scores_reliable_popular_streams_higher() -> None:
    strong = Station.from_api(
        {
            "stationuuid": "good",
            "name": "Good",
            "url": "https://good.test/live",
            "lastcheckok": 1,
            "bitrate": 320,
            "codec": "AAC",
            "clicktrend": 3,
            "votes": 250,
        }
    )
    weak = Station.from_api(
        {
            "stationuuid": "weak",
            "name": "Weak",
            "url": "https://weak.test/live",
            "lastcheckok": 1,
            "bitrate": 64,
            "codec": "MP3",
            "clicktrend": 0,
        }
    )

    assert strong.discovery_quality_score() > weak.discovery_quality_score()
    assert strong.discovery_quality_label().startswith("excellent discovery match")
