from station_scout.spotify import SpotifyClient, build_spotify_auth_request


def test_spotify_auth_request_uses_pkce_and_playlist_scopes() -> None:
    request = build_spotify_auth_request(
        client_id="client",
        redirect_uri="http://127.0.0.1:8765/spotify/callback",
    )

    assert "https://accounts.spotify.com/authorize?" in request.url
    assert "client_id=client" in request.url
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


class FakeSession:
    def __init__(self, payload: dict[str, object]) -> None:
        self.payload = payload
        self.posts: list[dict[str, str]] = []

    def post(self, url: str, *, data: dict[str, str], timeout: float):
        self.posts.append(data)
        return FakeResponse(self.payload)


class FakeResponse:
    def __init__(self, payload: dict[str, object]) -> None:
        self.payload = payload

    def raise_for_status(self) -> None:
        return None

    def json(self) -> dict[str, object]:
        return self.payload
