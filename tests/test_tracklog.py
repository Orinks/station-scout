import datetime as dt

from station_scout.models import Station
from station_scout.tracklog import TrackEntry, TrackSessionLog, parse_stream_title


def test_parse_artist_title_stream_title() -> None:
    entry = parse_stream_title("Kate Bush - Running Up That Hill", now=dt.datetime(2026, 5, 9, 17, 0))

    assert entry is not None
    assert entry.artist == "Kate Bush"
    assert entry.title == "Running Up That Hill"
    assert entry.display_line() == "Kate Bush - Running Up That Hill"


def test_parse_title_by_artist_format() -> None:
    entry = parse_stream_title("Running Up That Hill by Kate Bush")

    assert entry is not None
    assert entry.display_line() == "Kate Bush - Running Up That Hill"


def test_parse_artist_title_from_provider_metadata_blob() -> None:
    entry = parse_stream_title(
        'Justin Bieber - text="Yukon" song_spot="M" MediaBaseId="3142175" '
        'itunesTrackId="0" amgTrackId="-1" amgArtistId="0" TAID="0" TPID="339293406"'
    )

    assert entry is not None
    assert entry.artist == "Justin Bieber"
    assert entry.title == "Yukon"
    assert entry.display_line() == "Justin Bieber - Yukon"


def test_unknown_or_bad_metadata_is_kept_but_marked_uncertain() -> None:
    entry = parse_stream_title("Transmission FM Station ID")

    assert entry is not None
    assert entry.uncertain
    assert entry.display_line() == "Transmission FM Station ID ?"


def test_show_session_file_omits_station_name(tmp_path) -> None:
    station = Station(stationuuid="abc", name="Transmission FM", url_resolved="https://example.test")
    session = TrackSessionLog(
        root=tmp_path,
        station=station,
        show_name="Gina's Organized Chaos",
        started_at=dt.datetime(2026, 5, 13, 17, 0),
    )

    session.add(TrackEntry("Kate Bush", "Running Up That Hill", "raw", dt.datetime.now()))

    assert session.path.name == "gina-s-organized-chaos-2026-05-13-1700.txt"
    assert session.path.read_text(encoding="utf-8") == "Kate Bush - Running Up That Hill\n"


def test_session_deduplicates_repeated_metadata(tmp_path) -> None:
    station = Station(stationuuid="abc", name="Transmission FM", url_resolved="https://example.test")
    session = TrackSessionLog(root=tmp_path, station=station)
    entry = TrackEntry("Kate Bush", "Running Up That Hill", "raw", dt.datetime.now())

    assert session.add(entry)
    assert not session.add(entry)
    assert session.path.read_text(encoding="utf-8").count("Kate Bush") == 1
