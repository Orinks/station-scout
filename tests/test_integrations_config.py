from station_scout.integrations_config import DEFAULT_LASTFM_PROXY_URL, lastfm_app_config, spotify_app_config


def test_lastfm_defaults_to_station_scout_proxy(monkeypatch) -> None:
    monkeypatch.delenv("STATION_SCOUT_LASTFM_PROXY_URL", raising=False)
    monkeypatch.delenv("STATION_SCOUT_LASTFM_API_KEY", raising=False)
    monkeypatch.delenv("STATION_SCOUT_LASTFM_API_SECRET", raising=False)

    config = lastfm_app_config()

    assert config is not None
    assert config.proxy_url == DEFAULT_LASTFM_PROXY_URL


def test_lastfm_proxy_url_can_be_overridden(monkeypatch) -> None:
    monkeypatch.setenv("STATION_SCOUT_LASTFM_PROXY_URL", "https://example.test/api/lastfm/")

    config = lastfm_app_config()

    assert config is not None
    assert config.proxy_url == "https://example.test/api/lastfm"


def test_spotify_reads_client_id_from_home_env_local(monkeypatch, tmp_path) -> None:
    monkeypatch.delenv("STATION_SCOUT_SPOTIFY_CLIENT_ID", raising=False)
    monkeypatch.setattr("station_scout.integrations_config.Path.home", lambda: tmp_path)
    (tmp_path / ".env.local").write_text("Spotify client ID: client-from-file\n", encoding="utf-8")

    config = spotify_app_config()

    assert config is not None
    assert config.client_id == "client-from-file"


def test_spotify_reads_public_config_from_proxy(monkeypatch, tmp_path) -> None:
    monkeypatch.delenv("STATION_SCOUT_SPOTIFY_CLIENT_ID", raising=False)
    monkeypatch.setattr("station_scout.integrations_config.Path.home", lambda: tmp_path)
    monkeypatch.setattr(
        "station_scout.integrations_config.requests.get",
        lambda url, timeout: FakeResponse(
            {
                "clientId": "client-from-proxy",
                "redirectUri": "http://127.0.0.1:8765/spotify/callback",
            }
        ),
    )

    config = spotify_app_config()

    assert config is not None
    assert config.client_id == "client-from-proxy"
    assert config.redirect_uri == "http://127.0.0.1/spotify/callback"


class FakeResponse:
    def __init__(self, payload: dict[str, object]) -> None:
        self.payload = payload

    def raise_for_status(self) -> None:
        return None

    def json(self) -> dict[str, object]:
        return self.payload
