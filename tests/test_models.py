from station_scout.models import Station, TuneTimer


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
    timer = TuneTimer(stationuuid="abc", station_name="Example", time="07:30")

    assert TuneTimer.from_json(timer.to_json()) == timer

