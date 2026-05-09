from station_scout.app import APP_TITLE, playback_window_title


def test_playback_window_title_defaults_to_app_title() -> None:
    assert playback_window_title() == APP_TITLE


def test_playback_window_title_prefixes_now_playing_text() -> None:
    assert playback_window_title("Transmission FM\nGina's Organized Chaos") == (
        "Transmission FM Gina's Organized Chaos - Station Scout"
    )
