from __future__ import annotations

import datetime as dt
import http.server
import threading
import time
import webbrowser
from collections.abc import Callable
from urllib.parse import parse_qs, urlparse, urlunparse

import requests
import wx
import wx.adv

from station_scout.cast import (
    CastDevice,
    discover_chromecasts,
    discover_sonos,
    change_chromecast_volume,
    change_sonos_volume,
    play_on_chromecast,
    play_on_sonos,
    stop_chromecast,
    stop_sonos,
    toggle_chromecast_pause,
    toggle_sonos_pause,
)
from station_scout.direct_streams import station_from_direct_url, streamurl_search_url
from station_scout.integrations_config import lastfm_app_config, spotify_app_config
from station_scout.lastfm import LastFmClient, LastFmError, LastFmProxyClient, LastFmScrobbleCache
from station_scout.models import Station, TuneTimer
from station_scout.notifications import create_notifier
from station_scout.player import RadioPlayer
from station_scout.radio_browser import RadioBrowserClient, RadioBrowserError
from station_scout.schedule import due_timers
from station_scout.storage import AppState, SettingsStore, add_unique_station
from station_scout.spotify import SpotifyClient, SpotifyPlaylistResult, build_spotify_auth_request
from station_scout.tracklog import IcyMetadataReader, TrackEntry, TrackSessionLog, parse_stream_title


APP_TITLE = "Station Scout"


def station_scout_ui_blueprint() -> dict[str, tuple[str, ...] | str]:
    """Return the intended AccessiWeather-style main-window structure."""
    return {
        "title": APP_TITLE,
        "status_fields": ("Status", "Playback"),
        "initial_focus": "Favorites",
        "sections": (
            "Station",
            "Now Playing",
            "Search Radio Browser",
            "Stations",
            "Saved Stations",
            "Tune-in Timers",
            "Recent Events",
        ),
        "primary_actions": (
            "Play selected",
            "Add favorite",
            "Open website",
            "Add tune-in timer",
            "Start tracking",
            "Stop tracking",
        ),
        "list_names": (
            "Station search results",
            "Favorite stations",
            "Recently played stations",
            "Tune-in timers",
        ),
    }


def _describe_control(control: wx.Window, label: str) -> None:
    control.SetToolTip(label)
    control.SetHelpText(label)


def playback_window_title(now_playing: str = "") -> str:
    now_playing = " ".join(now_playing.split())
    if not now_playing:
        return APP_TITLE
    return f"{now_playing} - {APP_TITLE}"


def lastfm_settings_view_state(state: AppState) -> tuple[str, str, bool]:
    if state.lastfm_enabled:
        return (
            f"Connected as {state.lastfm_username or 'your account'}.",
            "Reconnect Last.fm",
            False,
        )
    if state.lastfm_pending_token:
        return (
            "Waiting for browser approval. Choose Finish Last.fm afterward.",
            "Restart Last.fm",
            True,
        )
    return "Not connected.", "Connect Last.fm", False


def lastfm_track_key(entry) -> str:
    return f"{entry.artist}\0{entry.title}".casefold()


def should_scrobble_lastfm(state: AppState, entry, sent_tracks: set[str]) -> bool:
    if not state.lastfm_enabled or not state.lastfm_scrobble_enabled:
        return False
    if entry.uncertain or not entry.artist or not entry.title:
        return False
    return lastfm_track_key(entry) not in sent_tracks


def should_show_stream_title(entry) -> bool:
    return bool(entry.artist and entry.title and not entry.uncertain)


def spotify_playlist_tracks(entries: list[TrackEntry]) -> list[tuple[str, str]]:
    tracks: list[tuple[str, str]] = []
    seen: set[str] = set()
    for entry in entries:
        if entry.uncertain or not entry.artist or not entry.title:
            continue
        key = lastfm_track_key(entry)
        if key in seen:
            continue
        seen.add(key)
        tracks.append((entry.artist, entry.title))
    return tracks


class StationScoutFrame(wx.Frame):
    def __init__(self) -> None:
        super().__init__(None, title=APP_TITLE, size=(980, 680))
        self.client = RadioBrowserClient()
        self.store = SettingsStore()
        self.state = self.store.load()
        self.results: list[Station] = []
        self.current_station: Station | None = None
        self.timer_fired_today: set[tuple[str, str, str]] = set()
        self.notifier = create_notifier(parent=self)
        self.track_session: TrackSessionLog | None = None
        self.lastfm_sent_tracks: set[str] = set()
        self.metadata_stop_event = threading.Event()
        self.track_stop_at = ""
        self.track_reader = IcyMetadataReader()
        self.chromecast_device: CastDevice | None = None
        self.sonos_device: CastDevice | None = None
        self.spotify_callback_server: http.server.ThreadingHTTPServer | None = None
        self.spotify_auth_redirect_uri = ""
        self.settings_dialog: SettingsDialog | None = None
        self.lastfm_client = self._create_lastfm_client()
        self.lastfm_cache = LastFmScrobbleCache(self.store.path.parent / "lastfm-scrobble-cache.jsonl")
        self.player = RadioPlayer(
            on_playing=self._on_player_started,
            on_stopped=self._on_player_stopped,
            on_error=self._on_player_error,
        )
        self.player.set_volume(self.state.volume)

        self._build_menu()
        self._build_controls()
        self._bind_events()
        self._build_accelerators()
        self._build_tray()
        self._refresh_saved_lists()
        self._set_status("Ready.")
        wx.CallAfter(self._set_initial_focus)

        self.timer = wx.Timer(self)
        self.Bind(wx.EVT_TIMER, self._on_tick, self.timer)
        self.timer.Start(30_000)

    def _build_menu(self) -> None:
        self.ID_FOCUS_SEARCH = wx.NewIdRef()
        self.ID_PLAY_SELECTED = wx.NewIdRef()
        self.ID_TOGGLE_PAUSE = wx.NewIdRef()
        self.ID_STOP_PLAYBACK = wx.NewIdRef()
        self.ID_VOLUME_UP = wx.NewIdRef()
        self.ID_VOLUME_DOWN = wx.NewIdRef()
        self.ID_PLAY_ON_COMPUTER = wx.NewIdRef()
        self.ID_CAST_CHROMECAST = wx.NewIdRef()
        self.ID_CAST_SONOS = wx.NewIdRef()
        self.ID_ADD_TIMER = wx.NewIdRef()
        self.ID_SETTINGS = wx.NewIdRef()
        self.ID_START_TRACKING = wx.NewIdRef()
        self.ID_STOP_TRACKING = wx.NewIdRef()

        menu_bar = wx.MenuBar()
        file_menu = wx.Menu()
        file_menu.Append(self.ID_SETTINGS, "Settings...\tCtrl+,")
        file_menu.AppendSeparator()
        file_menu.Append(wx.ID_EXIT, "Quit\tAlt+F4")
        menu_bar.Append(file_menu, "&File")

        playback_menu = wx.Menu()
        playback_menu.Append(self.ID_FOCUS_SEARCH, "Focus search\tCtrl+F")
        playback_menu.Append(self.ID_PLAY_SELECTED, "Play selected\tCtrl+P")
        playback_menu.Append(self.ID_TOGGLE_PAUSE, "Play/Pause\tCtrl+Space")
        playback_menu.Append(self.ID_STOP_PLAYBACK, "Stop\tCtrl+S")
        playback_menu.Append(self.ID_VOLUME_UP, "Volume up\tCtrl+Up")
        playback_menu.Append(self.ID_VOLUME_DOWN, "Volume down\tCtrl+Down")
        playback_menu.AppendSeparator()
        playback_menu.Append(self.ID_PLAY_ON_COMPUTER, "Play on this computer")
        playback_menu.Append(self.ID_CAST_SONOS, "Play on Sonos...")
        playback_menu.Append(self.ID_CAST_CHROMECAST, "Cast to Chromecast...")
        playback_menu.AppendSeparator()
        playback_menu.Append(self.ID_ADD_TIMER, "Add tune-in timer\tCtrl+T")
        playback_menu.Append(self.ID_START_TRACKING, "Start tracking\tCtrl+Shift+T")
        playback_menu.Append(self.ID_STOP_TRACKING, "Stop tracking\tCtrl+Shift+S")
        menu_bar.Append(playback_menu, "&Playback")
        self.SetMenuBar(menu_bar)

    def _build_controls(self) -> None:
        panel = wx.Panel(self)
        root = wx.BoxSizer(wx.VERTICAL)

        station_row = wx.BoxSizer(wx.HORIZONTAL)
        station_row.Add(wx.StaticText(panel, label="Station:"), 0, wx.ALIGN_CENTER_VERTICAL | wx.RIGHT, 8)
        self.station_choice = wx.Choice(panel, name="Station selection")
        _describe_control(self.station_choice, "Station selection")
        station_row.Add(self.station_choice, 1, wx.EXPAND | wx.RIGHT, 8)
        self.refresh_saved_button = wx.Button(panel, label="Refresh saved")
        station_row.Add(self.refresh_saved_button, 0)

        now_playing_box = wx.StaticBoxSizer(wx.VERTICAL, panel, "Now Playing")
        self.now_playing = wx.StaticText(panel, label="Nothing playing")
        self.now_playing.Wrap(760)
        _describe_control(self.now_playing, "Now playing")
        now_playing_box.Add(self.now_playing, 0, wx.EXPAND | wx.ALL, 8)

        search_box = wx.StaticBoxSizer(wx.VERTICAL, panel, "Search Radio Browser")

        form = wx.FlexGridSizer(2, 4, 6, 8)
        form.AddGrowableCol(1)
        form.AddGrowableCol(3)

        self.name_label = wx.StaticText(panel, label="Name")
        self.name_input = wx.TextCtrl(panel, name="Station name or call letters")
        self.country_label = wx.StaticText(panel, label="Country")
        self.country_input = wx.TextCtrl(panel, name="Country")
        self.language_label = wx.StaticText(panel, label="Language")
        self.language_input = wx.TextCtrl(panel, name="Language")
        self.tag_label = wx.StaticText(panel, label="Tag")
        self.tag_input = wx.TextCtrl(panel, name="Tag")
        for label, control, accessible_label in (
            (self.name_label, self.name_input, "Station name or call letters"),
            (self.country_label, self.country_input, "Country"),
            (self.language_label, self.language_input, "Language"),
            (self.tag_label, self.tag_input, "Tag"),
        ):
            _describe_control(control, accessible_label)
            form.Add(label, 0, wx.ALIGN_CENTER_VERTICAL)
            form.Add(control, 1, wx.EXPAND)
        search_box.Add(form, 0, wx.EXPAND | wx.ALL, 8)

        search_buttons = wx.BoxSizer(wx.HORIZONTAL)
        self.search_button = wx.Button(panel, label="Search")
        self.direct_stream_button = wx.Button(panel, label="Find direct stream URL")
        for button in (self.search_button, self.direct_stream_button):
            search_buttons.Add(button, 0, wx.RIGHT, 8)
        search_box.Add(search_buttons, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 8)

        actions_box = wx.StaticBoxSizer(wx.HORIZONTAL, panel, "Station actions")
        self.play_button = wx.Button(panel, label="Play selected")
        self.favorite_button = wx.Button(panel, label="Add favorite")
        self.website_button = wx.Button(panel, label="Open website")
        self.timer_button = wx.Button(panel, label="Add tune-in timer")
        self.start_tracking_button = wx.Button(panel, label="Start tracking")
        self.stop_tracking_button = wx.Button(panel, label="Stop tracking")
        for button in (
            self.play_button,
            self.favorite_button,
            self.website_button,
            self.timer_button,
            self.start_tracking_button,
            self.stop_tracking_button,
        ):
            actions_box.Add(button, 0, wx.RIGHT, 8)

        body = wx.BoxSizer(wx.VERTICAL)
        station_results = wx.StaticBoxSizer(wx.VERTICAL, panel, "Station Search Results")
        self.station_list = wx.ListCtrl(panel, style=wx.LC_REPORT | wx.LC_SINGLE_SEL)
        _describe_control(self.station_list, "Station search results")
        self.station_list.AppendColumn("Station", width=240)
        self.station_list.AppendColumn("Location", width=180)
        self.station_list.AppendColumn("Language", width=150)
        self.station_list.AppendColumn("Quality", width=170)
        self.station_list.AppendColumn("Source", width=140)
        station_results.Add(self.station_list, 1, wx.EXPAND | wx.ALL, 8)

        saved_box = wx.StaticBoxSizer(wx.HORIZONTAL, panel, "Saved Stations")
        favorites_col = wx.BoxSizer(wx.VERTICAL)
        favorites_col.Add(wx.StaticText(panel, label="Favorite stations:"), 0, wx.BOTTOM, 4)
        self.favorites_list = wx.ListBox(panel, name="Favorite stations")
        _describe_control(self.favorites_list, "Favorite stations")
        favorites_col.Add(self.favorites_list, 1, wx.EXPAND)
        recents_col = wx.BoxSizer(wx.VERTICAL)
        recents_col.Add(wx.StaticText(panel, label="Recently played stations:"), 0, wx.BOTTOM, 4)
        self.recents_list = wx.ListBox(panel, name="Recently played stations")
        _describe_control(self.recents_list, "Recently played stations")
        recents_col.Add(self.recents_list, 1, wx.EXPAND)
        saved_box.Add(favorites_col, 1, wx.EXPAND | wx.ALL, 8)
        saved_box.Add(recents_col, 1, wx.EXPAND | wx.TOP | wx.RIGHT | wx.BOTTOM, 8)

        lower = wx.BoxSizer(wx.HORIZONTAL)
        timers_box = wx.StaticBoxSizer(wx.VERTICAL, panel, "Tune-in Timers")
        self.timers_list = wx.ListBox(panel, name="Tune-in timers")
        _describe_control(self.timers_list, "Tune-in timers")
        timers_box.Add(self.timers_list, 1, wx.EXPAND | wx.ALL, 8)
        events_box = wx.StaticBoxSizer(wx.VERTICAL, panel, "Recent Events")
        self.events_display = wx.TextCtrl(
            panel,
            style=wx.TE_MULTILINE | wx.TE_READONLY | wx.TE_RICH2,
            name="Recent events",
        )
        _describe_control(self.events_display, "Recent events")
        events_box.Add(self.events_display, 1, wx.EXPAND | wx.ALL, 8)
        lower.Add(timers_box, 1, wx.EXPAND | wx.RIGHT, 10)
        lower.Add(events_box, 1, wx.EXPAND)

        body.Add(station_results, 2, wx.EXPAND | wx.BOTTOM, 10)
        body.Add(saved_box, 1, wx.EXPAND | wx.BOTTOM, 10)
        body.Add(lower, 1, wx.EXPAND)

        self.CreateStatusBar(2)
        self.GetStatusBar().SetStatusWidths([-2, -1])
        self.SetMinSize((820, 720))
        self.SetSize((980, 760))
        root.Add(station_row, 0, wx.EXPAND | wx.ALL, 10)
        root.Add(now_playing_box, 0, wx.EXPAND | wx.LEFT | wx.RIGHT | wx.BOTTOM, 10)
        root.Add(search_box, 0, wx.EXPAND | wx.ALL, 10)
        root.Add(actions_box, 0, wx.EXPAND | wx.LEFT | wx.RIGHT | wx.BOTTOM, 10)
        root.Add(body, 1, wx.EXPAND | wx.LEFT | wx.RIGHT, 10)
        panel.SetSizer(root)

    def _bind_events(self) -> None:
        self.Bind(wx.EVT_MENU, self._on_settings, id=self.ID_SETTINGS)
        self.Bind(wx.EVT_MENU, lambda _event: self.Close(), id=wx.ID_EXIT)
        self.Bind(wx.EVT_MENU, self._on_focus_search, id=self.ID_FOCUS_SEARCH)
        self.Bind(wx.EVT_MENU, self._on_play_selected, id=self.ID_PLAY_SELECTED)
        self.Bind(wx.EVT_MENU, lambda _event: self.toggle_pause(), id=self.ID_TOGGLE_PAUSE)
        self.Bind(wx.EVT_MENU, lambda _event: self.stop_playback(), id=self.ID_STOP_PLAYBACK)
        self.Bind(wx.EVT_MENU, lambda _event: self.adjust_volume(0.1), id=self.ID_VOLUME_UP)
        self.Bind(wx.EVT_MENU, lambda _event: self.adjust_volume(-0.1), id=self.ID_VOLUME_DOWN)
        self.Bind(wx.EVT_MENU, self._on_play_on_computer, id=self.ID_PLAY_ON_COMPUTER)
        self.Bind(wx.EVT_MENU, self._on_cast_sonos, id=self.ID_CAST_SONOS)
        self.Bind(wx.EVT_MENU, self._on_cast_chromecast, id=self.ID_CAST_CHROMECAST)
        self.Bind(wx.EVT_MENU, self._on_add_timer, id=self.ID_ADD_TIMER)
        self.Bind(wx.EVT_MENU, self._on_start_tracking, id=self.ID_START_TRACKING)
        self.Bind(wx.EVT_MENU, lambda _event: self.stop_tracking(), id=self.ID_STOP_TRACKING)
        self.Bind(wx.EVT_BUTTON, self._on_search, self.search_button)
        self.Bind(wx.EVT_BUTTON, self._on_direct_stream_fallback, self.direct_stream_button)
        self.Bind(wx.EVT_BUTTON, lambda _event: self._refresh_saved_lists(), self.refresh_saved_button)
        self.Bind(wx.EVT_BUTTON, self._on_playback_button, self.play_button)
        self.Bind(wx.EVT_BUTTON, self._on_add_favorite, self.favorite_button)
        self.Bind(wx.EVT_BUTTON, self._on_open_website, self.website_button)
        self.Bind(wx.EVT_BUTTON, self._on_add_timer, self.timer_button)
        self.Bind(wx.EVT_BUTTON, self._on_start_tracking, self.start_tracking_button)
        self.Bind(wx.EVT_BUTTON, lambda _event: self.stop_tracking(), self.stop_tracking_button)
        self.station_list.Bind(wx.EVT_LIST_ITEM_ACTIVATED, self._on_play_selected)
        self.station_choice.Bind(wx.EVT_CHOICE, self._on_station_choice)
        self.favorites_list.Bind(wx.EVT_LISTBOX_DCLICK, lambda _event: self._play_saved("favorites"))
        self.recents_list.Bind(wx.EVT_LISTBOX_DCLICK, lambda _event: self._play_saved("recents"))
        self.Bind(wx.EVT_CHAR_HOOK, self._on_char_hook)
        self.Bind(wx.EVT_CLOSE, self._on_close)

    def _set_initial_focus(self) -> None:
        self.favorites_list.SetFocus()

    def _build_accelerators(self) -> None:
        self.SetAcceleratorTable(
            wx.AcceleratorTable(
                [
                    (wx.ACCEL_CTRL, ord("F"), self.ID_FOCUS_SEARCH),
                    (wx.ACCEL_CTRL, ord("P"), self.ID_PLAY_SELECTED),
                    (wx.ACCEL_CTRL, wx.WXK_SPACE, self.ID_TOGGLE_PAUSE),
                    (wx.ACCEL_CTRL, ord("S"), self.ID_STOP_PLAYBACK),
                    (wx.ACCEL_CTRL, wx.WXK_UP, self.ID_VOLUME_UP),
                    (wx.ACCEL_CTRL, wx.WXK_DOWN, self.ID_VOLUME_DOWN),
                    (wx.ACCEL_CTRL, ord("T"), self.ID_ADD_TIMER),
                    (wx.ACCEL_CTRL, ord(","), self.ID_SETTINGS),
                    (wx.ACCEL_CTRL | wx.ACCEL_SHIFT, ord("T"), self.ID_START_TRACKING),
                    (wx.ACCEL_CTRL | wx.ACCEL_SHIFT, ord("S"), self.ID_STOP_TRACKING),
                ]
            )
        )

    def _build_tray(self) -> None:
        self.tray = wx.adv.TaskBarIcon()
        icon = wx.ArtProvider.GetIcon(wx.ART_FIND, wx.ART_OTHER, (16, 16))
        self.tray.SetIcon(icon, APP_TITLE)
        self.tray.Bind(wx.adv.EVT_TASKBAR_LEFT_DCLICK, lambda _event: self._show_from_tray())
        self.tray.Bind(wx.adv.EVT_TASKBAR_RIGHT_UP, self._on_tray_menu)

    def _on_tray_menu(self, event: wx.Event) -> None:
        menu = wx.Menu()
        show_id = wx.NewIdRef()
        stop_id = wx.NewIdRef()
        quit_id = wx.NewIdRef()
        menu.Append(show_id, "Show Station Scout")
        menu.Append(self.ID_TOGGLE_PAUSE, "Play/Pause")
        menu.Append(stop_id, "Stop playback")
        menu.AppendSeparator()
        menu.Append(quit_id, "Quit")
        self.tray.Bind(wx.EVT_MENU, lambda _event: self._show_from_tray(), id=show_id)
        self.tray.Bind(wx.EVT_MENU, lambda _event: self.toggle_pause(), id=self.ID_TOGGLE_PAUSE)
        self.tray.Bind(wx.EVT_MENU, lambda _event: self.stop_playback(), id=stop_id)
        self.tray.Bind(wx.EVT_MENU, lambda _event: self.Close(), id=quit_id)
        self.tray.PopupMenu(menu)
        menu.Destroy()
        event.Skip()

    def _on_search(self, _event: wx.Event) -> None:
        self._set_status("Searching Radio Browser...")
        self.search_button.Disable()
        self._run_background(
            lambda: self.client.search(
                name=self.name_input.GetValue().strip(),
                country=self.country_input.GetValue().strip(),
                language=self.language_input.GetValue().strip(),
                tag=self.tag_input.GetValue().strip(),
            ),
            self._show_results,
            self._show_error,
        )

    def _show_results(self, stations: list[Station]) -> None:
        self.search_button.Enable()
        self.results = stations
        self.station_list.DeleteAllItems()
        for station in stations:
            index = self.station_list.InsertItem(self.station_list.GetItemCount(), station.name)
            self.station_list.SetItem(index, 1, station.country or station.countrycode)
            self.station_list.SetItem(index, 2, station.language)
            self.station_list.SetItem(index, 3, station.quality_label())
            self.station_list.SetItem(index, 4, station.source)
        if stations:
            self.station_list.Select(0)
            self.station_list.Focus(0)
            self.station_list.SetFocus()
        self._set_status(f"{len(stations)} stations found.")

    def _on_direct_stream_fallback(self, _event: wx.Event) -> None:
        dialog = DirectStreamDialog(self, self.name_input.GetValue().strip())
        try:
            if dialog.ShowModal() == wx.ID_OK:
                try:
                    station = dialog.station()
                except ValueError as exc:
                    self._show_error(exc)
                    return
                self._show_results([station])
                self._set_status("Direct stream URL added. Press Enter or Play selected to tune in.")
        finally:
            dialog.Destroy()

    def _on_play_selected(self, _event: wx.Event) -> None:
        station = self._selected_station()
        if station:
            self.play_station(station)

    def _on_playback_button(self, _event: wx.Event) -> None:
        if self._remote_output_device() and self.current_station:
            self.toggle_pause()
            return
        if self.player.is_playing() or self.player.is_paused():
            self.toggle_pause()
            return
        self._on_play_selected(_event)

    def _on_focus_search(self, _event: wx.Event) -> None:
        self.name_input.SetFocus()
        self.name_input.SelectAll()

    def _on_settings(self, _event: wx.Event) -> None:
        dialog = SettingsDialog(self)
        self.settings_dialog = dialog
        try:
            dialog.ShowModal()
        finally:
            self.settings_dialog = None
            dialog.Destroy()

    def play_station(self, station: Station) -> None:
        if not station.url_resolved:
            self._show_error(RadioBrowserError("Selected station does not have a stream URL."))
            return
        if self.track_session and self.current_station and self.current_station.stationuuid != station.stationuuid:
            if self.track_session.add_station(station):
                self._set_status(f"Tracking continued on {station.name}.")
        self.current_station = station
        self.state.recents = add_unique_station(self.state.recents, station, limit=20)
        self.store.save(self.state)
        self._refresh_saved_lists()
        self.now_playing.SetLabel(f"{station.name}\n{station.subtitle()}\n{station.quality_label()}")
        self._set_playback_title(station.name)
        self._set_status(f"Loading {station.name}...")
        self._notify("Station Scout", f"Tuning in to {station.name}")
        if station.source == "Radio Browser":
            self._play_url(station.url_resolved)
            self._record_radio_browser_click(station)
        else:
            self._play_url(station.url_resolved)

    def _play_url(self, url: str) -> None:
        if not url:
            self._show_error(RadioBrowserError("Radio Browser did not return a stream URL."))
            return
        if self.current_station:
            self.current_station = Station.from_api({**self.current_station.to_json(), "url_resolved": url})
        if not self.current_station:
            return
        if self.sonos_device:
            device = self.sonos_device
            station = self.current_station
            self.player.stop(notify=False)
            self._set_status(f"Playing {station.name} on Sonos: {device.name}...")
            self._run_background(
                lambda: play_on_sonos(device, station, url),
                lambda _result: self._on_sonos_started(station, device),
                self._on_player_error,
            )
            return
        if self.chromecast_device:
            device = self.chromecast_device
            station = self.current_station
            self.player.stop(notify=False)
            self._set_status(f"Casting {station.name} to {device.name}...")
            self._run_background(
                lambda: play_on_chromecast(device, station, url),
                lambda _result: self._on_chromecast_started(station, device),
                self._on_player_error,
            )
            return
        if self.player.play(url):
            self._start_metadata_monitor(self.current_station)

    def _record_radio_browser_click(self, station: Station) -> None:
        def worker() -> None:
            try:
                url = self.client.click_url(station.stationuuid)
            except RadioBrowserError as exc:
                wx.CallAfter(self._set_status, f"Playing; Radio Browser click report failed: {exc}")
                return
            if url:
                wx.CallAfter(self._update_current_station_url, station.stationuuid, url)

        threading.Thread(target=worker, daemon=True).start()

    def _update_current_station_url(self, stationuuid: str, url: str) -> None:
        if self.current_station and self.current_station.stationuuid == stationuuid:
            self.current_station = Station.from_api({**self.current_station.to_json(), "url_resolved": url})

    def _on_player_started(self) -> None:
        self._set_status(f"Playing {self.current_station.name if self.current_station else 'stream'}.")
        self._refresh_playback_button()

    def _on_chromecast_started(self, station: Station, device: CastDevice) -> None:
        self._set_status(f"Casting {station.name} to {device.name}.")
        self._start_metadata_monitor(station)
        self._refresh_playback_button()

    def _on_sonos_started(self, station: Station, device: CastDevice) -> None:
        self._set_status(f"Playing {station.name} on Sonos: {device.name}.")
        self._start_metadata_monitor(station)
        self._refresh_playback_button()

    def _on_player_stopped(self) -> None:
        self._set_status("Stopped.")
        self._refresh_playback_button()
        self._set_playback_title()

    def _on_player_error(self, message: str) -> None:
        self._stop_metadata_monitor()
        self._refresh_playback_button()
        self._set_playback_title()
        self._show_error(RadioBrowserError(message))

    def stop_playback(self) -> None:
        self._stop_metadata_monitor()
        if self.sonos_device:
            device = self.sonos_device
            self._run_background(lambda: stop_sonos(device), lambda _result: None, self._show_error)
        if self.chromecast_device:
            device = self.chromecast_device
            self._run_background(lambda: stop_chromecast(device), lambda _result: None, self._show_error)
        self.player.stop()
        self._set_status("Stopped.")
        self._refresh_playback_button()
        self._set_playback_title()

    def toggle_pause(self) -> None:
        if self.sonos_device:
            device = self.sonos_device
            self._run_background(
                lambda: toggle_sonos_pause(device),
                lambda paused: self._on_remote_pause_changed(bool(paused)),
                self._show_error,
            )
            return
        if self.chromecast_device:
            device = self.chromecast_device
            self._run_background(
                lambda: toggle_chromecast_pause(device),
                lambda paused: self._on_remote_pause_changed(bool(paused)),
                self._show_error,
            )
            return
        if not self.current_station:
            station = self._selected_station()
            if station:
                self.play_station(station)
            else:
                self._set_status("Choose a station first.")
            return
        if self.player.toggle_pause():
            if self.player.is_paused():
                self._set_status("Paused.")
            else:
                self._set_status(f"Playing {self.current_station.name}.")
            self._refresh_playback_button()
        else:
            self.play_station(self.current_station)

    def adjust_volume(self, delta: float) -> None:
        if self.sonos_device:
            device = self.sonos_device
            self._run_background(
                lambda: change_sonos_volume(device, delta),
                self._on_remote_volume_changed,
                self._show_error,
            )
            return
        if self.chromecast_device:
            device = self.chromecast_device
            self._run_background(
                lambda: change_chromecast_volume(device, delta),
                self._on_remote_volume_changed,
                self._show_error,
            )
            return
        level = self.player.change_volume(delta)
        self.state.volume = level
        self.store.save(self.state)
        self._set_status(f"Volume {round(level * 100)} percent.")

    def _on_remote_pause_changed(self, paused: bool) -> None:
        self._set_status("Paused." if paused else f"Playing {self.current_station.name if self.current_station else 'stream'}.")
        self._refresh_playback_button()

    def _on_remote_volume_changed(self, result: object) -> None:
        try:
            level = float(result)
        except (TypeError, ValueError):
            return
        self._set_status(f"Volume {round(level * 100)} percent.")

    def _on_play_on_computer(self, _event: wx.Event) -> None:
        previous_device = self.chromecast_device
        previous_sonos = self.sonos_device
        self.chromecast_device = None
        self.sonos_device = None
        if previous_device:
            self._run_background(lambda: stop_chromecast(previous_device), lambda _result: None, self._show_error)
        if previous_sonos:
            self._run_background(lambda: stop_sonos(previous_sonos), lambda _result: None, self._show_error)
        self._set_status("Output set to this computer.")
        if self.current_station:
            self.play_station(self.current_station)

    def _on_cast_sonos(self, _event: wx.Event) -> None:
        self._set_status("Searching for Sonos speakers...")
        self._run_background(discover_sonos, self._choose_sonos, self._show_error)

    def _on_cast_chromecast(self, _event: wx.Event) -> None:
        self._set_status("Searching for Chromecasts...")
        self._run_background(discover_chromecasts, self._choose_chromecast, self._show_error)

    def _choose_sonos(self, result: object) -> None:
        devices = result if isinstance(result, list) else []
        if not devices:
            self._set_status("No Sonos speakers found on this network.")
            return
        dialog = RemoteOutputDialog(self, devices, title="Play on Sonos", label="Sonos speaker")
        try:
            if dialog.ShowModal() != wx.ID_OK:
                self._set_status("Sonos output unchanged.")
                return
            self.sonos_device = dialog.selected_device()
            self.chromecast_device = None
        finally:
            dialog.Destroy()
        if not self.sonos_device:
            self._set_status("Sonos output unchanged.")
            return
        self._set_status(f"Output set to Sonos: {self.sonos_device.name}")
        station = self.current_station or self._selected_station()
        if station:
            self.play_station(station)

    def _choose_chromecast(self, result: object) -> None:
        devices = result if isinstance(result, list) else []
        if not devices:
            self._set_status("No Chromecasts found on this network.")
            return
        dialog = RemoteOutputDialog(self, devices, title="Cast to Chromecast", label="Chromecast")
        try:
            if dialog.ShowModal() != wx.ID_OK:
                self._set_status("Chromecast output unchanged.")
                return
            self.chromecast_device = dialog.selected_device()
            self.sonos_device = None
        finally:
            dialog.Destroy()
        if not self.chromecast_device:
            self._set_status("Chromecast output unchanged.")
            return
        self._set_status(f"Output set to Chromecast: {self.chromecast_device.name}")
        station = self.current_station or self._selected_station()
        if station:
            self.play_station(station)

    def _on_add_favorite(self, _event: wx.Event) -> None:
        station = self._selected_station() or self.current_station
        if not station:
            self._set_status("Choose a station first.")
            return
        self.state.favorites = add_unique_station(self.state.favorites, station)
        self.store.save(self.state)
        self._refresh_saved_lists()
        self._set_status(f"Added favorite: {station.name}")

    def _on_open_website(self, _event: wx.Event) -> None:
        station = self._selected_station() or self.current_station
        if station and station.homepage:
            webbrowser.open(station.homepage)
            self._set_status(f"Opened website for {station.name}.")
        else:
            self._set_status("No website is listed for this station.")

    def _on_add_timer(self, _event: wx.Event) -> None:
        station = self._selected_station() or self.current_station
        if not station:
            self._set_status("Choose a station first.")
            return
        dialog = TimerDialog(self, station.name)
        try:
            if dialog.ShowModal() == wx.ID_OK:
                timer = TuneTimer(
                    stationuuid=station.stationuuid,
                    station_name=station.name,
                    time=dialog.time_value(),
                    auto_play=dialog.auto_play.GetValue(),
                    end_time=dialog.end_time_value(),
                    show_name=dialog.show_name.GetValue().strip(),
                    track_playlist=dialog.track_playlist.GetValue(),
                )
                self.state.timers.append(timer)
                self.store.save(self.state)
                self._refresh_saved_lists()
                self._set_status(f"Timer added for {station.name} at {timer.time}.")
        finally:
            dialog.Destroy()

    def _on_tick(self, _event: wx.Event) -> None:
        for timer in due_timers(self.state.timers, dt.datetime.now(), self.timer_fired_today):
            self._fire_timer(timer)

    def _fire_timer(self, timer: TuneTimer) -> None:
        self._notify("Tune-in timer", f"{timer.station_name} is scheduled now.")
        station = self._find_station(timer.stationuuid)
        if timer.auto_play and station:
            self.play_station(station)
        if timer.track_playlist and station:
            self.start_tracking(station, show_name=timer.show_name, stop_at=timer.end_time)

    def _find_station(self, stationuuid: str) -> Station | None:
        for group in (self.state.favorites, self.state.recents, self.results):
            for station in group:
                if station.stationuuid == stationuuid:
                    return station
        return None

    def _play_saved(self, list_name: str) -> None:
        source = self.state.favorites if list_name == "favorites" else self.state.recents
        control = self.favorites_list if list_name == "favorites" else self.recents_list
        selection = control.GetSelection()
        if selection != wx.NOT_FOUND and selection < len(source):
            self.play_station(source[selection])

    def _on_station_choice(self, _event: wx.Event) -> None:
        stations = self.state.favorites
        selection = self.station_choice.GetSelection()
        if selection != wx.NOT_FOUND and selection < len(stations):
            self.play_station(stations[selection])

    def _on_char_hook(self, event: wx.KeyEvent) -> None:
        if event.GetKeyCode() == wx.WXK_RETURN:
            focus = wx.Window.FindFocus()
            if focus == self.station_list:
                self._on_play_selected(event)
                return
            if focus == self.favorites_list:
                self._play_saved("favorites")
                return
            if focus == self.recents_list:
                self._play_saved("recents")
                return
        event.Skip()

    def _selected_station(self) -> Station | None:
        index = self.station_list.GetFirstSelected()
        if index == -1 or index >= len(self.results):
            return None
        return self.results[index]

    def _refresh_saved_lists(self) -> None:
        self.favorites_list.Set([station.name for station in self.state.favorites])
        self.recents_list.Set([station.name for station in self.state.recents])
        self.station_choice.Set([station.name for station in self.state.favorites])
        if self.state.favorites:
            self.station_choice.SetSelection(0)
        self.timers_list.Set(
            [
                (
                    f"{timer.time}"
                    f"{f' to {timer.end_time}' if timer.end_time else ''}"
                    f" - {timer.show_name or timer.station_name}"
                    f"{' - tracking' if timer.track_playlist else ''}"
                    f"{' - auto play' if timer.auto_play else ''}"
                )
                for timer in self.state.timers
            ]
        )

    def _show_from_tray(self) -> None:
        self.Show()
        self.Raise()

    def _on_close(self, event: wx.CloseEvent) -> None:
        self.store.save(self.state)
        self.timer.Stop()
        self._stop_spotify_callback_server()
        self.tray.RemoveIcon()
        self.tray.Destroy()
        event.Skip()

    def _set_status(self, text: str) -> None:
        self.SetStatusText(text, 0)
        if hasattr(self, "events_display"):
            self.events_display.AppendText(f"{text}\n")

    def _set_playback_title(self, now_playing: str = "") -> None:
        self.SetTitle(playback_window_title(now_playing))
        self.SetStatusText(now_playing or "No playback", 1)

    def _refresh_playback_button(self) -> None:
        if self.sonos_device and self.current_station:
            self.play_button.SetLabel("Play/Pause")
            return
        if self.chromecast_device and self.current_station:
            self.play_button.SetLabel("Play/Pause")
            return
        if self.player.is_paused():
            self.play_button.SetLabel("Resume")
        elif self.player.is_playing():
            self.play_button.SetLabel("Pause")
        else:
            self.play_button.SetLabel("Play selected")

    def _start_metadata_monitor(self, station: Station) -> None:
        self._stop_metadata_monitor()
        self.metadata_stop_event = threading.Event()
        stop_event = self.metadata_stop_event

        def worker() -> None:
            try:
                for raw_title in self.track_reader.titles(station.url_resolved):
                    if stop_event.is_set():
                        break
                    entry = parse_stream_title(raw_title)
                    if entry and should_show_stream_title(entry):
                        wx.CallAfter(self._on_stream_title, station.stationuuid, entry)
                    elif entry:
                        wx.CallAfter(self._on_ignored_stream_title, station.stationuuid, entry.raw)
                    else:
                        wx.CallAfter(self._on_ignored_stream_title, station.stationuuid, raw_title)
            except (requests.RequestException, ValueError) as exc:
                wx.CallAfter(self._set_status, f"No stream title metadata: {exc}")

        threading.Thread(target=worker, daemon=True).start()

    def _stop_metadata_monitor(self) -> None:
        self.metadata_stop_event.set()

    def _on_stream_title(self, stationuuid: str, entry) -> None:
        if not self.current_station or self.current_station.stationuuid != stationuuid:
            return
        if self.track_stop_at and _stop_time_reached(self.track_stop_at):
            self.stop_tracking()
        if not should_show_stream_title(entry):
            self._on_ignored_stream_title(stationuuid, entry.raw)
            return
        line = entry.display_line()
        self._set_playback_title(line)
        self.now_playing.SetLabel(f"{line}\n{self.current_station.name}")
        if should_scrobble_lastfm(self.state, entry, self.lastfm_sent_tracks):
            self._send_lastfm(entry)
            self.lastfm_sent_tracks.add(lastfm_track_key(entry))
        if self.track_session and self.track_session.add(entry, station=self.current_station):
            self._set_status(f"Tracked: {line}")

    def _on_ignored_stream_title(self, stationuuid: str, raw_title: str) -> None:
        if not self.current_station or self.current_station.stationuuid != stationuuid:
            return
        if hasattr(self, "events_display"):
            self.events_display.AppendText(f"Ignored stream title metadata: {raw_title}\n")

    def _on_start_tracking(self, _event: wx.Event) -> None:
        station = self.current_station or self._selected_station()
        if not station:
            self._set_status("Choose or play a station first.")
            return
        self.start_tracking(station)

    def start_tracking(
        self,
        station: Station,
        *,
        show_name: str = "",
        stop_at: str = "",
    ) -> None:
        self.track_session = None
        log_root = self.store.track_log_folder(self.state)
        self.track_session = TrackSessionLog(root=log_root, station=station, show_name=show_name)
        self.track_stop_at = stop_at
        self._set_status(f"Tracking songs to {self.track_session.path}")
        self._notify("Playlist tracking started", show_name or station.name)
        if self.current_station is None or self.current_station.stationuuid != station.stationuuid:
            self.play_station(station)
        elif not self._remote_output_device() and not self.player.is_playing() and not self.player.is_paused():
            self.play_station(station)

    def _remote_output_device(self) -> CastDevice | None:
        return self.sonos_device or self.chromecast_device

    def stop_tracking(self) -> None:
        session = self.track_session
        if session:
            self._set_status(f"Tracking saved to {session.path}")
            self._notify("Playlist tracking stopped", session.show_name or session.station.name)
        self.track_session = None
        self.track_stop_at = ""
        if session and self.state.spotify_enabled and spotify_playlist_tracks(session.entries):
            prompt = wx.MessageDialog(
                self,
                "Create a private Spotify playlist from this tracking session?",
                "Create Spotify playlist",
                wx.YES_NO | wx.NO_DEFAULT | wx.ICON_QUESTION,
            )
            try:
                create_playlist = prompt.ShowModal() == wx.ID_YES
            finally:
                prompt.Destroy()
            if create_playlist:
                self._create_spotify_playlist_from_session(session)

    def _on_choose_log_folder(self, _event: wx.Event) -> None:
        dialog = wx.DirDialog(
            self,
            "Choose playlist tracking log folder",
            str(self.store.track_log_folder(self.state)),
        )
        try:
            if dialog.ShowModal() == wx.ID_OK:
                self.state.track_log_folder = dialog.GetPath()
                self.store.save(self.state)
                self._set_status(f"Track logs will be saved to {self.state.track_log_folder}")
        finally:
            dialog.Destroy()

    def _create_lastfm_client(self) -> LastFmClient | None:
        config = lastfm_app_config()
        if not config or not self.state.lastfm_session_key or not self.state.lastfm_enabled:
            return None
        if config.proxy_url:
            return LastFmProxyClient(
                proxy_url=config.proxy_url,
                session_key=self.state.lastfm_session_key,
            )
        return LastFmClient(
            api_key=config.api_key,
            api_secret=config.api_secret,
            session_key=self.state.lastfm_session_key,
        )

    def _on_connect_lastfm(self, _event: wx.Event) -> None:
        config = lastfm_app_config()
        if not config:
            self._set_status("Station Scout is missing its Last.fm app credentials.")
            return
        if config.proxy_url:
            client = LastFmProxyClient(proxy_url=config.proxy_url)
            try:
                token, auth_url = client.request_token()
            except (LastFmError, requests.RequestException) as exc:
                self._show_error(exc)
                return
            self.state.lastfm_pending_token = token
            self.store.save(self.state)
            webbrowser.open(auth_url)
            self._set_status("Approve Station Scout in Last.fm, then choose Finish Last.fm.")
            return

        client = LastFmClient(api_key=config.api_key, api_secret=config.api_secret, session_key="")
        try:
            token = client.request_token()
        except (LastFmError, requests.RequestException) as exc:
            self._show_error(exc)
            return
        self.state.lastfm_pending_token = token
        self.store.save(self.state)
        webbrowser.open(client.auth_url(token))
        self._set_status("Approve Station Scout in Last.fm, then choose Finish Last.fm.")

    def _on_finish_lastfm(self, _event: wx.Event) -> None:
        config = lastfm_app_config()
        if not config or not self.state.lastfm_pending_token:
            self._set_status("Start Last.fm connection first.")
            return
        if config.proxy_url:
            client = LastFmProxyClient(proxy_url=config.proxy_url)
        else:
            client = LastFmClient(api_key=config.api_key, api_secret=config.api_secret, session_key="")
        try:
            session_key, username = client.create_session(self.state.lastfm_pending_token)
        except (LastFmError, requests.RequestException) as exc:
            self._show_error(exc)
            return
        self.state.lastfm_session_key = session_key
        self.state.lastfm_username = username
        self.state.lastfm_enabled = True
        self.state.lastfm_scrobble_enabled = True
        self.state.lastfm_pending_token = ""
        self.store.save(self.state)
        self.lastfm_client = self._create_lastfm_client()
        self._set_status(f"Connected Last.fm as {username or 'your account'}.")

    def _on_connect_spotify(self, _event: wx.Event) -> None:
        config = spotify_app_config()
        if not config:
            self._set_status("Station Scout is missing its Spotify client ID.")
            return
        redirect_uri = self._start_spotify_callback_server(config.redirect_uri)
        if not redirect_uri:
            return
        auth_request = build_spotify_auth_request(
            client_id=config.client_id,
            redirect_uri=redirect_uri,
        )
        self.spotify_auth_redirect_uri = redirect_uri
        self.state.spotify_auth_state = auth_request.state
        self.state.spotify_code_verifier = auth_request.code_verifier
        self.store.save(self.state)
        webbrowser.open(auth_request.url)
        self._set_status("Approve Station Scout in Spotify. This window will finish automatically.")

    def _finish_spotify_with_code(self, code: str, state: str) -> None:
        config = spotify_app_config()
        if not config or state != self.state.spotify_auth_state:
            self._set_status("Spotify authorization state did not match.")
            return
        redirect_uri = self.spotify_auth_redirect_uri or config.redirect_uri
        client = SpotifyClient(client_id=config.client_id, redirect_uri=redirect_uri)
        token_set = client.exchange_code(code=code, code_verifier=self.state.spotify_code_verifier)
        self.state.spotify_access_token = token_set.access_token
        self.state.spotify_refresh_token = token_set.refresh_token
        self.state.spotify_token_expires_at = token_set.expires_at
        self.state.spotify_enabled = True
        self.state.spotify_auth_state = ""
        self.state.spotify_code_verifier = ""
        self.spotify_auth_redirect_uri = ""
        self.store.save(self.state)
        self._set_status("Connected Spotify.")
        self._refresh_settings_dialog()

    def _refresh_settings_dialog(self) -> None:
        if self.settings_dialog:
            self.settings_dialog.refresh()

    def _start_spotify_callback_server(self, redirect_uri: str) -> str:
        self._stop_spotify_callback_server()
        parsed = urlparse(redirect_uri)
        host = parsed.hostname or "127.0.0.1"
        port = parsed.port or 0
        path = parsed.path or "/"
        frame = self

        class SpotifyCallbackHandler(http.server.BaseHTTPRequestHandler):
            def do_GET(self) -> None:  # noqa: N802
                request = urlparse(self.path)
                if request.path != path:
                    self.send_error(404)
                    return
                query = parse_qs(request.query)
                code = query.get("code", [""])[0]
                state = query.get("state", [""])[0]
                error = query.get("error", [""])[0]
                if error:
                    wx.CallAfter(frame._set_status, f"Spotify authorization failed: {error}")
                    wx.CallAfter(frame._stop_spotify_callback_server)
                    self._respond("Spotify authorization failed. You can close this tab.")
                    return
                if not code or not state:
                    self.send_error(400)
                    return
                wx.CallAfter(frame._finish_spotify_with_code, code, state)
                wx.CallAfter(frame._stop_spotify_callback_server)
                self._respond("Station Scout is connected to Spotify. You can close this tab.")

            def log_message(self, _format: str, *_args: object) -> None:
                return

            def _respond(self, body: str) -> None:
                payload = body.encode("utf-8")
                self.send_response(200)
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

        try:
            self.spotify_callback_server = http.server.ThreadingHTTPServer((host, port), SpotifyCallbackHandler)
        except OSError as exc:
            self._show_error(exc)
            return ""
        threading.Thread(target=self.spotify_callback_server.serve_forever, daemon=True).start()
        actual_port = int(self.spotify_callback_server.server_address[1])
        return urlunparse((parsed.scheme or "http", f"{host}:{actual_port}", path, "", "", ""))

    def _stop_spotify_callback_server(self) -> None:
        if not self.spotify_callback_server:
            return
        self.spotify_callback_server.shutdown()
        self.spotify_callback_server.server_close()
        self.spotify_callback_server = None
        self.spotify_auth_redirect_uri = ""

    def _send_lastfm(self, entry) -> None:
        if not self.lastfm_client:
            return
        try:
            self.lastfm_client.update_now_playing(entry)
            self.lastfm_client.scrobble(entry)
        except (LastFmError, requests.RequestException):
            self.lastfm_cache.append(entry)

    def _create_spotify_playlist_from_session(self, session: TrackSessionLog) -> None:
        config = spotify_app_config()
        tracks = spotify_playlist_tracks(session.entries)
        if not config or not self.state.spotify_enabled or not self.state.spotify_access_token:
            return
        if not tracks:
            self._set_status("No Spotify-ready artist/title tracks were tracked.")
            return
        name = session.show_name or f"{session.station.name} {session.started_at:%Y-%m-%d}"
        client = SpotifyClient(client_id=config.client_id, redirect_uri=config.redirect_uri)

        def work() -> SpotifyPlaylistResult:
            if self.state.spotify_refresh_token and self.state.spotify_token_expires_at <= int(time.time()) + 60:
                token_set = client.refresh_access_token(refresh_token=self.state.spotify_refresh_token)
                self.state.spotify_access_token = token_set.access_token
                self.state.spotify_refresh_token = token_set.refresh_token
                self.state.spotify_token_expires_at = token_set.expires_at
                self.store.save(self.state)
            return client.create_playlist_from_tracks(
                access_token=self.state.spotify_access_token,
                name=name,
                tracks=tracks,
                public=False,
            )

        def on_success(result: object) -> None:
            playlist = result if isinstance(result, SpotifyPlaylistResult) else None
            if not playlist:
                return
            detail = f"{playlist.matched_tracks} songs"
            if playlist.skipped_tracks:
                detail += f", {playlist.skipped_tracks} not found"
            if playlist.url:
                self._set_status(f"Created Spotify playlist: {playlist.name} ({detail}) {playlist.url}")
            else:
                self._set_status(f"Created Spotify playlist: {playlist.name} ({detail})")

        self._set_status("Creating Spotify playlist...")
        self._run_background(work, on_success, self._show_error)

    def _show_error(self, exc: BaseException) -> None:
        self.search_button.Enable()
        self._set_status(str(exc))
        self._notify("Station Scout error", str(exc))


    def _notify(self, title: str, message: str) -> None:
        self.notifier.notify(title, message)

    def _run_background(
        self,
        work: Callable[[], object],
        on_success: Callable[[object], None],
        on_error: Callable[[BaseException], None],
    ) -> None:
        def runner() -> None:
            try:
                result = work()
            except BaseException as exc:
                wx.CallAfter(on_error, exc)
            else:
                wx.CallAfter(on_success, result)

        threading.Thread(target=runner, daemon=True).start()


class DirectStreamDialog(wx.Dialog):
    def __init__(self, parent: wx.Window, initial_name: str = "") -> None:
        super().__init__(parent, title="Find direct stream URL")
        root = wx.BoxSizer(wx.VERTICAL)

        form = wx.FlexGridSizer(3, 2, 6, 8)
        form.AddGrowableCol(1)
        name_label = wx.StaticText(self, label="Name")
        self.name_input = wx.TextCtrl(self, value=initial_name)
        website_label = wx.StaticText(self, label="Website")
        self.website_input = wx.TextCtrl(self)
        url_label = wx.StaticText(self, label="Direct stream URL")
        self.url_input = wx.TextCtrl(self)
        for label, control, description in (
            (name_label, self.name_input, "Station name or call letters"),
            (website_label, self.website_input, "Station website"),
            (url_label, self.url_input, "Direct stream URL"),
        ):
            _describe_control(control, description)
            form.Add(label, 0, wx.ALIGN_CENTER_VERTICAL)
            form.Add(control, 1, wx.EXPAND)

        buttons = wx.BoxSizer(wx.HORIZONTAL)
        self.open_search_button = wx.Button(self, label="Open direct URL search")
        buttons.Add(self.open_search_button, 0, wx.RIGHT, 8)

        root.Add(form, 0, wx.EXPAND | wx.ALL, 12)
        root.Add(buttons, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(self.CreateStdDialogButtonSizer(wx.OK | wx.CANCEL), 0, wx.EXPAND | wx.ALL, 12)
        self.SetSizerAndFit(root)
        self.SetMinSize((520, self.GetSize().height))

        self.Bind(wx.EVT_BUTTON, self._on_open_search, self.open_search_button)
        self.Bind(wx.EVT_CHAR_HOOK, self._on_char_hook)
        self.name_input.SetFocus()

    def station(self) -> Station:
        return station_from_direct_url(
            name=self.name_input.GetValue(),
            url=self.url_input.GetValue(),
            homepage=self.website_input.GetValue(),
        )

    def _on_open_search(self, _event: wx.Event) -> None:
        query = self.name_input.GetValue().strip()
        if query:
            webbrowser.open(streamurl_search_url(query))
            return
        self.name_input.SetFocus()

    def _on_char_hook(self, event: wx.KeyEvent) -> None:
        if event.GetKeyCode() == wx.WXK_ESCAPE:
            self.EndModal(wx.ID_CANCEL)
            return
        event.Skip()


class RemoteOutputDialog(wx.Dialog):
    def __init__(self, parent: wx.Window, devices: list[CastDevice], *, title: str, label: str) -> None:
        super().__init__(parent, title=title)
        self.devices = devices
        root = wx.BoxSizer(wx.VERTICAL)
        device_label = wx.StaticText(self, label=label)
        self.device_list = wx.ListBox(self, choices=[device.name for device in devices], name=label)
        _describe_control(self.device_list, label)
        if devices:
            self.device_list.SetSelection(0)
        root.Add(device_label, 0, wx.LEFT | wx.RIGHT | wx.TOP, 12)
        root.Add(self.device_list, 1, wx.EXPAND | wx.ALL, 12)
        root.Add(self.CreateStdDialogButtonSizer(wx.OK | wx.CANCEL), 0, wx.EXPAND | wx.ALL, 12)
        self.SetSizerAndFit(root)
        self.SetMinSize((420, 260))
        self.Bind(wx.EVT_LISTBOX_DCLICK, lambda _event: self.EndModal(wx.ID_OK), self.device_list)
        self.Bind(wx.EVT_CHAR_HOOK, self._on_char_hook)
        self.device_list.SetFocus()

    def selected_device(self) -> CastDevice | None:
        selection = self.device_list.GetSelection()
        if selection == wx.NOT_FOUND or selection >= len(self.devices):
            return None
        return self.devices[selection]

    def _on_char_hook(self, event: wx.KeyEvent) -> None:
        if event.GetKeyCode() == wx.WXK_ESCAPE:
            self.EndModal(wx.ID_CANCEL)
            return
        event.Skip()


class SettingsDialog(wx.Dialog):
    def __init__(self, parent: StationScoutFrame) -> None:
        super().__init__(parent, title="Station Scout settings")
        self.parent_frame = parent
        root = wx.BoxSizer(wx.VERTICAL)

        storage_box = wx.StaticBoxSizer(wx.VERTICAL, self, "Playlist logs")
        folder_row = wx.BoxSizer(wx.HORIZONTAL)
        folder_label = wx.StaticText(self, label="Folder")
        self.log_folder_value = wx.TextCtrl(self, style=wx.TE_READONLY)
        _describe_control(self.log_folder_value, "Playlist log folder")
        self.log_folder_button = wx.Button(self, label="Choose folder")
        folder_row.Add(folder_label, 0, wx.ALIGN_CENTER_VERTICAL | wx.RIGHT, 8)
        folder_row.Add(self.log_folder_value, 1, wx.EXPAND | wx.RIGHT, 8)
        folder_row.Add(self.log_folder_button, 0)
        storage_box.Add(folder_row, 0, wx.EXPAND | wx.ALL, 12)

        lastfm_box = wx.StaticBoxSizer(wx.VERTICAL, self, "Last.fm")
        self.lastfm_status = wx.StaticText(self)
        self.lastfm_scrobble_checkbox = wx.CheckBox(self, label="Scrobble played songs to Last.fm")
        lastfm_buttons = wx.BoxSizer(wx.HORIZONTAL)
        self.connect_lastfm_button = wx.Button(self, label="Connect Last.fm")
        self.finish_lastfm_button = wx.Button(self, label="Finish Last.fm")
        lastfm_buttons.Add(self.connect_lastfm_button, 0, wx.RIGHT, 8)
        lastfm_buttons.Add(self.finish_lastfm_button, 0)
        lastfm_box.Add(self.lastfm_status, 0, wx.EXPAND | wx.ALL, 12)
        lastfm_box.Add(self.lastfm_scrobble_checkbox, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        lastfm_box.Add(lastfm_buttons, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)

        spotify_box = wx.StaticBoxSizer(wx.VERTICAL, self, "Spotify")
        self.spotify_status = wx.StaticText(self)
        self.connect_spotify_button = wx.Button(self, label="Connect Spotify")
        spotify_box.Add(self.spotify_status, 0, wx.EXPAND | wx.ALL, 12)
        spotify_box.Add(self.connect_spotify_button, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)

        root.Add(storage_box, 0, wx.EXPAND | wx.ALL, 12)
        root.Add(lastfm_box, 0, wx.EXPAND | wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(spotify_box, 0, wx.EXPAND | wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(self.CreateStdDialogButtonSizer(wx.OK), 0, wx.EXPAND | wx.ALL, 12)
        self.SetSizerAndFit(root)
        self.SetMinSize((520, self.GetSize().height))

        self.Bind(wx.EVT_BUTTON, self._on_choose_log_folder, self.log_folder_button)
        self.Bind(wx.EVT_BUTTON, self._on_connect_lastfm, self.connect_lastfm_button)
        self.Bind(wx.EVT_BUTTON, self._on_finish_lastfm, self.finish_lastfm_button)
        self.Bind(wx.EVT_BUTTON, self._on_connect_spotify, self.connect_spotify_button)
        self.Bind(wx.EVT_CHECKBOX, self._on_toggle_lastfm_scrobbling, self.lastfm_scrobble_checkbox)
        self.Bind(wx.EVT_CHAR_HOOK, self._on_char_hook)
        self._refresh()
        self.log_folder_button.SetFocus()

    def _refresh(self) -> None:
        state = self.parent_frame.state
        self.log_folder_value.SetValue(str(self.parent_frame.store.track_log_folder(state)))
        lastfm_status, connect_label, show_finish = lastfm_settings_view_state(state)
        self.lastfm_status.SetLabel(lastfm_status)
        self.connect_lastfm_button.SetLabel(connect_label)
        if show_finish:
            self.finish_lastfm_button.Show()
        else:
            self.finish_lastfm_button.Hide()
        self.lastfm_scrobble_checkbox.SetValue(state.lastfm_scrobble_enabled)
        self.lastfm_scrobble_checkbox.Enable(state.lastfm_enabled)
        self.spotify_status.SetLabel("Connected." if state.spotify_enabled else "Not connected.")
        self.Layout()
        self.Fit()

    def _on_choose_log_folder(self, _event: wx.Event) -> None:
        self.parent_frame._on_choose_log_folder(_event)
        self._refresh()

    def _on_connect_lastfm(self, _event: wx.Event) -> None:
        self.parent_frame._on_connect_lastfm(_event)
        self._refresh()

    def _on_finish_lastfm(self, _event: wx.Event) -> None:
        self.parent_frame._on_finish_lastfm(_event)
        self._refresh()

    def _on_toggle_lastfm_scrobbling(self, _event: wx.Event) -> None:
        self.parent_frame.state.lastfm_scrobble_enabled = self.lastfm_scrobble_checkbox.GetValue()
        self.parent_frame.store.save(self.parent_frame.state)
        status = "enabled" if self.parent_frame.state.lastfm_scrobble_enabled else "disabled"
        self.parent_frame._set_status(f"Last.fm scrobbling {status}.")

    def _on_connect_spotify(self, _event: wx.Event) -> None:
        self.parent_frame._on_connect_spotify(_event)
        self.refresh()

    def refresh(self) -> None:
        self._refresh()

    def _on_char_hook(self, event: wx.KeyEvent) -> None:
        if event.GetKeyCode() == wx.WXK_ESCAPE:
            self.EndModal(wx.ID_CANCEL)
            return
        event.Skip()


class TimerDialog(wx.Dialog):
    def __init__(self, parent: wx.Window, station_name: str) -> None:
        super().__init__(parent, title=f"Add timer for {station_name}")
        root = wx.BoxSizer(wx.VERTICAL)
        row = wx.BoxSizer(wx.HORIZONTAL)
        row.Add(wx.StaticText(self, label="Time"), 0, wx.ALIGN_CENTER_VERTICAL | wx.RIGHT, 8)
        self.time_picker = wx.adv.TimePickerCtrl(self)
        row.Add(self.time_picker, 0)
        self.auto_play = wx.CheckBox(self, label="Automatically start playback")
        self.auto_play.SetValue(True)
        self.track_playlist = wx.CheckBox(self, label="Track songs for this show")
        show_name_label = wx.StaticText(self, label="Show name")
        self.show_name = wx.TextCtrl(self, name="Show name")
        _describe_control(self.show_name, "Show name")
        end_row = wx.BoxSizer(wx.HORIZONTAL)
        end_row.Add(wx.StaticText(self, label="End time"), 0, wx.ALIGN_CENTER_VERTICAL | wx.RIGHT, 8)
        self.end_time_picker = wx.adv.TimePickerCtrl(self)
        _describe_control(self.time_picker, "Timer start time")
        _describe_control(self.end_time_picker, "Timer end time")
        end_row.Add(self.end_time_picker, 0)
        root.Add(row, 0, wx.ALL, 12)
        root.Add(self.auto_play, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(self.track_playlist, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(show_name_label, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 4)
        root.Add(self.show_name, 0, wx.EXPAND | wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(end_row, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(self.CreateStdDialogButtonSizer(wx.OK | wx.CANCEL), 0, wx.EXPAND | wx.ALL, 12)
        self.SetSizerAndFit(root)
        self.Bind(wx.EVT_CHAR_HOOK, self._on_char_hook)
        self.time_picker.SetFocus()

    def time_value(self) -> str:
        value = self.time_picker.GetValue()
        return f"{value.GetHour():02d}:{value.GetMinute():02d}"

    def end_time_value(self) -> str:
        value = self.end_time_picker.GetValue()
        return f"{value.GetHour():02d}:{value.GetMinute():02d}"

    def _on_char_hook(self, event: wx.KeyEvent) -> None:
        if event.GetKeyCode() == wx.WXK_ESCAPE:
            self.EndModal(wx.ID_CANCEL)
            return
        event.Skip()


def _stop_time_reached(stop_at: str) -> bool:
    if not stop_at:
        return False
    return dt.datetime.now().strftime("%H:%M") >= stop_at


class StationScoutApp(wx.App):
    def OnInit(self) -> bool:
        self.SetAppName(APP_TITLE)
        frame = StationScoutFrame()
        frame.Show()
        return True


def main() -> None:
    app = StationScoutApp(False)
    app.MainLoop()


if __name__ == "__main__":
    main()
