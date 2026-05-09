from station_scout.integrations_config import DEFAULT_LASTFM_PROXY_URL, lastfm_app_config


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
