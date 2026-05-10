import datetime as dt

from station_scout.app import (
    APP_TITLE,
    lastfm_settings_view_state,
    lastfm_track_key,
    playback_window_title,
    should_scrobble_lastfm,
    should_show_stream_title,
    spotify_playlist_tracks,
    station_scout_ui_blueprint,
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


def test_uncertain_station_id_metadata_does_not_replace_now_playing() -> None:
    entry = TrackEntry(
        "Philly's #1 Hit Music Station",
        "Q102",
        "raw",
        dt.datetime.now(),
        uncertain=True,
    )

    assert not should_show_stream_title(entry)


def test_station_scout_ui_blueprint_adapts_accessiweather_sections() -> None:
    blueprint = station_scout_ui_blueprint()

    assert blueprint["title"] == APP_TITLE
    assert blueprint["status_fields"] == ("Status", "Playback")
    assert blueprint["initial_focus"] == "Favorites"
    assert blueprint["sections"] == (
        "Station",
        "Now Playing",
        "Search Radio Browser",
        "Stations",
        "Saved Stations",
        "Tune-in Timers",
        "Recent Events",
    )


def test_station_scout_ui_blueprint_keeps_station_actions_compact() -> None:
    blueprint = station_scout_ui_blueprint()

    assert blueprint["primary_actions"] == (
        "Play selected",
        "Add favorite",
        "Open website",
        "Add tune-in timer",
        "Start tracking",
        "Stop tracking",
    )


def test_station_scout_ui_blueprint_uses_distinct_accessible_list_names() -> None:
    blueprint = station_scout_ui_blueprint()

    assert blueprint["list_names"] == (
        "Station search results",
        "Favorite stations",
        "Recently played stations",
        "Tune-in timers",
    )
