from station_scout.app import _is_network_url


def test_is_network_url_matches_stream_schemes() -> None:
    assert _is_network_url("https://example.test/live.mp3")
    assert _is_network_url("http://example.test/live")
    assert _is_network_url("icy://example.test/live")
    assert not _is_network_url("C:/Music/example.mp3")
