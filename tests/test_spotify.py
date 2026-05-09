from station_scout.spotify import SpotifyClient, build_spotify_auth_request


def test_spotify_auth_request_uses_pkce_and_playlist_scopes() -> None:
    request = build_spotify_auth_request(
        client_id="client",
        redirect_uri="http://127.0.0.1:49152/spotify/callback",
    )

    assert "https://accounts.spotify.com/authorize?" in request.url
    assert "client_id=client" in request.url
    assert "redirect_uri=http%3A%2F%2F127.0.0.1%3A49152%2Fspotify%2Fcallback" in request.url
    assert "code_challenge_method=S256" in request.url
    assert "playlist-modify-private" in request.url
    assert request.state
    assert request.code_verifier


def test_spotify_exchanges_code_with_pkce_verifier() -> None:
    client = SpotifyClient(client_id="client", redirect_uri="http://callback")
    fake = FakeSession(
        {
            "access_token": "access",
            "refresh_token": "refresh",
            "expires_in": 3600,
        }
    )
    client.session = fake  # type: ignore[assignment]

    token_set = client.exchange_code(code="code", code_verifier="verifier")

    assert token_set.access_token == "access"
    assert token_set.refresh_token == "refresh"
    assert fake.posts[0]["grant_type"] == "authorization_code"
    assert fake.posts[0]["code_verifier"] == "verifier"


def test_spotify_refresh_preserves_refresh_token_when_not_returned() -> None:
    client = SpotifyClient(client_id="client", redirect_uri="http://callback")
    fake = FakeSession({"access_token": "new-access", "expires_in": 3600})
    client.session = fake  # type: ignore[assignment]

    token_set = client.refresh_access_token(refresh_token="old-refresh")

    assert token_set.access_token == "new-access"
    assert token_set.refresh_token == "old-refresh"
    assert fake.posts[0]["grant_type"] == "refresh_token"


def test_spotify_creates_playlist_from_matched_tracks() -> None:
    client = SpotifyClient(client_id="client", redirect_uri="http://callback")
    fake = FakeSession({})
    fake.gets = [
        {"id": "user"},
        {"tracks": {"items": [{"uri": "spotify:track:one"}]}},
        {"tracks": {"items": []}},
    ]
    fake.posts = [
        {
            "id": "playlist",
            "external_urls": {"spotify": "https://open.spotify.com/playlist/playlist"},
        }
    ]
    client.session = fake  # type: ignore[assignment]

    result = client.create_playlist_from_tracks(
        access_token="access",
        name="Show",
        tracks=[("Artist", "Song"), ("Missing", "Song")],
    )

    assert result.matched_tracks == 1
    assert result.skipped_tracks == 1
    assert result.url == "https://open.spotify.com/playlist/playlist"
    assert fake.json_posts[1]["uris"] == ["spotify:track:one"]


class FakeSession:
    def __init__(self, payload: dict[str, object]) -> None:
        self.payload = payload
        self.posts: list[dict[str, str]] = []
        self.json_posts: list[dict[str, object]] = []
        self.gets: list[dict[str, object]] = []

    def post(
        self,
        url: str,
        *,
        data: dict[str, str] | None = None,
        json: dict[str, object] | None = None,
        headers: dict[str, str] | None = None,
        timeout: float,
    ):
        if data is not None:
            self.posts.append(data)
        if json is not None:
            self.json_posts.append(json)
        if self.posts and "id" in self.posts[0]:
            return FakeResponse(self.posts.pop(0))
        return FakeResponse(self.payload)

    def get(
        self,
        url: str,
        *,
        headers: dict[str, str],
        params: dict[str, object] | None = None,
        timeout: float,
    ):
        return FakeResponse(self.gets.pop(0))


class FakeResponse:
    def __init__(self, payload: dict[str, object]) -> None:
        self.payload = payload

    def raise_for_status(self) -> None:
        return None

    def json(self) -> dict[str, object]:
        return self.payload
