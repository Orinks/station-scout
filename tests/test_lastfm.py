import datetime as dt

import pytest

from station_scout.lastfm import LastFmClient, LastFmError, LastFmScrobbleCache, sign_lastfm_request
from station_scout.tracklog import TrackEntry


def test_sign_lastfm_request_sorts_params_and_appends_secret() -> None:
    signature = sign_lastfm_request({"track": "Song", "artist": "Artist", "api_key": "key"}, "secret")

    assert signature == "76b2da4c153c8e14e28bae1c9c7d90d7"


def test_scrobble_uses_chosen_by_user_false_for_radio_tracks() -> None:
    client = LastFmClient(api_key="key", api_secret="secret", session_key="session")
    fake = FakeSession("<lfm status='ok'></lfm>")
    client.session = fake  # type: ignore[assignment]
    entry = TrackEntry("Artist", "Song", "Artist - Song", dt.datetime(2026, 5, 9, 17, 0))

    client.scrobble(entry)

    assert fake.posts[0]["method"] == "track.scrobble"
    assert fake.posts[0]["chosenByUser"] == "0"
    assert fake.posts[0]["artist"] == "Artist"
    assert fake.posts[0]["track"] == "Song"


def test_scrobble_skips_uncertain_metadata() -> None:
    client = LastFmClient(api_key="key", api_secret="secret", session_key="session")
    fake = FakeSession("<lfm status='ok'></lfm>")
    client.session = fake  # type: ignore[assignment]

    client.scrobble(TrackEntry("", "", "Station ID", dt.datetime.now(), uncertain=True))

    assert fake.posts == []


def test_lastfm_failure_reports_retryability() -> None:
    client = LastFmClient(api_key="key", api_secret="secret", session_key="session")
    client.session = FakeSession("<lfm status='failed'><error code='16'>Try later</error></lfm>")  # type: ignore[assignment]

    with pytest.raises(LastFmError, match="retryable"):
        client.scrobble(TrackEntry("Artist", "Song", "raw", dt.datetime.now()))


def test_scrobble_cache_skips_uncertain_entries(tmp_path) -> None:
    cache = LastFmScrobbleCache(tmp_path / "cache.jsonl")

    cache.append(TrackEntry("", "", "Station ID", dt.datetime.now(), uncertain=True))

    assert not cache.path.exists()


class FakeSession:
    def __init__(self, response_text: str) -> None:
        self.response_text = response_text
        self.posts: list[dict[str, str]] = []

    def post(self, url: str, *, data: dict[str, str], timeout: float):
        self.posts.append(data)
        return FakeResponse(self.response_text)


class FakeResponse:
    def __init__(self, text: str) -> None:
        self.text = text

    def raise_for_status(self) -> None:
        return None
