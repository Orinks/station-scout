import datetime as dt

import pytest

from station_scout.lastfm import (
    LastFmClient,
    LastFmError,
    LastFmProxyClient,
    LastFmScrobbleCache,
    sign_lastfm_request,
)
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


def test_lastfm_browser_auth_flow_gets_token_and_session() -> None:
    client = LastFmClient(api_key="key", api_secret="secret", session_key="")
    fake = FakeSession(
        [
            "<lfm status='ok'><token>abc</token></lfm>",
            "<lfm status='ok'><session><name>listener</name><key>session</key></session></lfm>",
        ]
    )
    client.session = fake  # type: ignore[assignment]

    token = client.request_token()
    session_key, username = client.create_session(token)

    assert token == "abc"
    assert client.auth_url(token) == "https://www.last.fm/api/auth/?api_key=key&token=abc"
    assert session_key == "session"
    assert username == "listener"


def test_lastfm_proxy_flow_gets_token_and_session() -> None:
    client = LastFmProxyClient(proxy_url="https://example.test/api/lastfm")
    fake = FakeJsonSession(
        [
            {"token": "abc", "authUrl": "https://www.last.fm/api/auth/?token=abc"},
            {"sessionKey": "session", "username": "listener"},
        ]
    )
    client.session = fake  # type: ignore[assignment]

    token, auth_url = client.request_token()
    session_key, username = client.create_session(token)

    assert token == "abc"
    assert auth_url == "https://www.last.fm/api/auth/?token=abc"
    assert session_key == "session"
    assert username == "listener"
    assert fake.posts[0]["url"] == "https://example.test/api/lastfm/token"
    assert fake.posts[1]["json"] == {"token": "abc"}


def test_lastfm_proxy_scrobbles_radio_tracks() -> None:
    client = LastFmProxyClient(proxy_url="https://example.test/api/lastfm", session_key="session")
    fake = FakeJsonSession([{"ok": True}, {"ok": True}])
    client.session = fake  # type: ignore[assignment]
    entry = TrackEntry("Artist", "Song", "Artist - Song", dt.datetime(2026, 5, 9, 17, 0))

    client.update_now_playing(entry)
    client.scrobble(entry)

    assert fake.posts[0]["url"] == "https://example.test/api/lastfm/now-playing"
    assert fake.posts[0]["json"] == {
        "sessionKey": "session",
        "artist": "Artist",
        "track": "Song",
    }
    assert fake.posts[1]["json"]["timestamp"] == str(int(entry.timestamp.timestamp()))


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
    def __init__(self, response_text: str | list[str]) -> None:
        self.responses = [response_text] if isinstance(response_text, str) else response_text
        self.posts: list[dict[str, str]] = []

    def post(self, url: str, *, data: dict[str, str], timeout: float):
        self.posts.append(data)
        return FakeResponse(self.responses.pop(0))


class FakeResponse:
    def __init__(self, text: str) -> None:
        self.text = text

    def raise_for_status(self) -> None:
        return None


class FakeJsonSession:
    def __init__(self, payloads: list[dict[str, object]]) -> None:
        self.payloads = payloads
        self.posts: list[dict[str, object]] = []

    def post(self, url: str, *, json: dict[str, str], timeout: float):
        self.posts.append({"url": url, "json": json, "timeout": timeout})
        return FakeJsonResponse(self.payloads.pop(0))


class FakeJsonResponse:
    ok = True
    status_code = 200

    def __init__(self, payload: dict[str, object]) -> None:
        self.payload = payload

    def json(self) -> dict[str, object]:
        return self.payload
