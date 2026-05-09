import datetime as dt

from station_scout.app import (
    APP_TITLE,
    lastfm_settings_view_state,
    lastfm_track_key,
    playback_window_title,
    should_scrobble_lastfm,
    spotify_playlist_tracks,
)
from station_scout.storage import AppState
from station_scout.tracklog import TrackEntry


def test_playback_window_title_defaults_to_app_title() -> None:
    assert playback_window_title() == APP_TITLE


def test_playback_window_title_prefixes_now_playing_text() -> None:
    assert playback_window_title("Transmission FM\nGina's Organized Chaos") == (
        "Transmission FM Gina's Organized Chaos - Station Scout"
    )


def test_lastfm_settings_hides_finish_button_when_connected() -> None:
    status, connect_label, show_finish = lastfm_settings_view_state(
        AppState(lastfm_enabled=True, lastfm_username="Orinks", lastfm_pending_token="old")
    )

    assert status == "Connected as Orinks."
    assert connect_label == "Reconnect Last.fm"
    assert not show_finish


def test_lastfm_settings_shows_finish_button_only_while_pending() -> None:
    status, connect_label, show_finish = lastfm_settings_view_state(
        AppState(lastfm_pending_token="pending")
    )

    assert status == "Waiting for browser approval. Choose Finish Last.fm afterward."
    assert connect_label == "Restart Last.fm"
    assert show_finish


def test_lastfm_scrobbling_requires_connected_enabled_clean_new_track() -> None:
    state = AppState(lastfm_enabled=True, lastfm_scrobble_enabled=True)
    sent_tracks: set[str] = set()
    entry = TrackEntry("Artist", "Song", "Artist - Song", dt.datetime.now())

    assert should_scrobble_lastfm(state, entry, sent_tracks)

    sent_tracks.add(lastfm_track_key(entry))

    assert not should_scrobble_lastfm(state, entry, sent_tracks)


def test_lastfm_scrobbling_respects_user_preference() -> None:
    state = AppState(lastfm_enabled=True, lastfm_scrobble_enabled=False)
    entry = TrackEntry("Artist", "Song", "Artist - Song", dt.datetime.now())

    assert not should_scrobble_lastfm(state, entry, set())


def test_spotify_playlist_tracks_filters_uncertain_and_duplicates() -> None:
    now = dt.datetime.now()
    entries = [
        TrackEntry("Artist", "Song", "Artist - Song", now),
        TrackEntry("Artist", "Song", "Artist - Song", now),
        TrackEntry("", "", "Station ID", now, uncertain=True),
    ]

    assert spotify_playlist_tracks(entries) == [("Artist", "Song")]
