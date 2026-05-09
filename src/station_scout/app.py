from __future__ import annotations

import datetime as dt
import threading
import webbrowser
from collections.abc import Callable

import requests
import wx
import wx.adv

from station_scout.direct_streams import station_from_direct_url, streamurl_search_url
from station_scout.integrations_config import lastfm_app_config, spotify_app_config
from station_scout.lastfm import LastFmClient, LastFmError, LastFmScrobbleCache
from station_scout.models import Station, TuneTimer
from station_scout.notifications import create_notifier
from station_scout.player import RadioPlayer
from station_scout.radio_browser import RadioBrowserClient, RadioBrowserError
from station_scout.schedule import due_timers
from station_scout.storage import SettingsStore, add_unique_station
from station_scout.spotify import SpotifyClient, build_spotify_auth_request
from station_scout.tracklog import IcyMetadataReader, TrackSessionLog, parse_stream_title


APP_TITLE = "Station Scout"


def _describe_control(control: wx.Window, label: str) -> None:
    control.SetToolTip(label)
    control.SetHelpText(label)


def playback_window_title(now_playing: str = "") -> str:
    now_playing = " ".join(now_playing.split())
    if not now_playing:
        return APP_TITLE
    return f"{now_playing} - {APP_TITLE}"


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
        self.metadata_stop_event = threading.Event()
        self.track_stop_at = ""
        self.track_reader = IcyMetadataReader()
        self.lastfm_client = self._create_lastfm_client()
        self.lastfm_cache = LastFmScrobbleCache(self.store.path.parent / "lastfm-scrobble-cache.jsonl")
        self.player = RadioPlayer(
            on_playing=self._on_player_started,
            on_stopped=self._on_player_stopped,
            on_error=self._on_player_error,
        )

        self._build_menu()
        self._build_controls()
        self._bind_events()
        self._build_accelerators()
        self._build_tray()
        self._refresh_saved_lists()
        self._set_status("Ready.")
        self.name_input.SetFocus()

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
        playback_menu.Append(self.ID_ADD_TIMER, "Add tune-in timer\tCtrl+T")
        playback_menu.Append(self.ID_START_TRACKING, "Start tracking\tCtrl+Shift+T")
        playback_menu.Append(self.ID_STOP_TRACKING, "Stop tracking\tCtrl+Shift+S")
        menu_bar.Append(playback_menu, "&Playback")
        self.SetMenuBar(menu_bar)

    def _build_controls(self) -> None:
        panel = wx.Panel(self)
        root = wx.BoxSizer(wx.VERTICAL)

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

        body = wx.BoxSizer(wx.HORIZONTAL)
        station_results = wx.BoxSizer(wx.VERTICAL)
        station_results_label = wx.StaticText(panel, label="Stations")
        self.station_list = wx.ListCtrl(panel, style=wx.LC_REPORT | wx.LC_SINGLE_SEL)
        _describe_control(self.station_list, "Stations")
        self.station_list.AppendColumn("Station", width=240)
        self.station_list.AppendColumn("Location", width=180)
        self.station_list.AppendColumn("Language", width=150)
        self.station_list.AppendColumn("Quality", width=170)
        self.station_list.AppendColumn("Source", width=140)
        station_results.Add(station_results_label, 0, wx.BOTTOM, 4)
        station_results.Add(self.station_list, 1, wx.EXPAND)

        side = wx.BoxSizer(wx.VERTICAL)
        now_playing_label = wx.StaticText(panel, label="Now playing")
        self.now_playing = wx.StaticText(panel, label="Nothing playing")
        self.now_playing.Wrap(280)
        favorites_label = wx.StaticText(panel, label="Favorites")
        self.favorites_list = wx.ListBox(panel, name="Favorites")
        _describe_control(self.favorites_list, "Favorites")
        recents_label = wx.StaticText(panel, label="Recent stations")
        self.recents_list = wx.ListBox(panel, name="Recent stations")
        _describe_control(self.recents_list, "Recent stations")
        timers_label = wx.StaticText(panel, label="Timers")
        self.timers_list = wx.ListBox(panel, name="Tune-in timers")
        _describe_control(self.timers_list, "Tune-in timers")
        side.Add(now_playing_label, 0, wx.BOTTOM, 4)
        side.Add(self.now_playing, 0, wx.EXPAND | wx.BOTTOM, 8)
        side.Add(favorites_label, 0, wx.BOTTOM, 4)
        side.Add(self.favorites_list, 1, wx.EXPAND | wx.BOTTOM, 8)
        side.Add(recents_label, 0, wx.BOTTOM, 4)
        side.Add(self.recents_list, 1, wx.EXPAND | wx.BOTTOM, 8)
        side.Add(timers_label, 0, wx.BOTTOM, 4)
        side.Add(self.timers_list, 1, wx.EXPAND)

        body.Add(station_results, 2, wx.EXPAND | wx.RIGHT, 10)
        body.Add(side, 1, wx.EXPAND)

        self.CreateStatusBar()
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
        self.Bind(wx.EVT_MENU, self._on_add_timer, id=self.ID_ADD_TIMER)
        self.Bind(wx.EVT_MENU, self._on_start_tracking, id=self.ID_START_TRACKING)
        self.Bind(wx.EVT_MENU, lambda _event: self.stop_tracking(), id=self.ID_STOP_TRACKING)
        self.Bind(wx.EVT_BUTTON, self._on_search, self.search_button)
        self.Bind(wx.EVT_BUTTON, self._on_direct_stream_fallback, self.direct_stream_button)
        self.Bind(wx.EVT_BUTTON, self._on_playback_button, self.play_button)
        self.Bind(wx.EVT_BUTTON, self._on_add_favorite, self.favorite_button)
        self.Bind(wx.EVT_BUTTON, self._on_open_website, self.website_button)
        self.Bind(wx.EVT_BUTTON, self._on_add_timer, self.timer_button)
        self.Bind(wx.EVT_BUTTON, self._on_start_tracking, self.start_tracking_button)
        self.Bind(wx.EVT_BUTTON, lambda _event: self.stop_tracking(), self.stop_tracking_button)
        self.station_list.Bind(wx.EVT_LIST_ITEM_ACTIVATED, self._on_play_selected)
        self.favorites_list.Bind(wx.EVT_LISTBOX_DCLICK, lambda _event: self._play_saved("favorites"))
        self.recents_list.Bind(wx.EVT_LISTBOX_DCLICK, lambda _event: self._play_saved("recents"))
        self.Bind(wx.EVT_CHAR_HOOK, self._on_char_hook)
        self.Bind(wx.EVT_CLOSE, self._on_close)

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
        if self.player.is_playing() or self.player.is_paused():
            self.toggle_pause()
            return
        self._on_play_selected(_event)

    def _on_focus_search(self, _event: wx.Event) -> None:
        self.name_input.SetFocus()
        self.name_input.SelectAll()

    def _on_settings(self, _event: wx.Event) -> None:
        dialog = SettingsDialog(self)
        try:
            dialog.ShowModal()
        finally:
            dialog.Destroy()

    def play_station(self, station: Station) -> None:
        if not station.url_resolved:
            self._show_error(RadioBrowserError("Selected station does not have a stream URL."))
            return
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
        if self.player.play(url) and self.current_station:
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
        self.player.stop()
        self._set_status("Stopped.")
        self._refresh_playback_button()
        self._set_playback_title()

    def toggle_pause(self) -> None:
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
        level = self.player.change_volume(delta)
        self._set_status(f"Volume {round(level * 100)} percent.")

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
        self.tray.RemoveIcon()
        self.tray.Destroy()
        event.Skip()

    def _set_status(self, text: str) -> None:
        self.SetStatusText(text)

    def _set_playback_title(self, now_playing: str = "") -> None:
        self.SetTitle(playback_window_title(now_playing))

    def _refresh_playback_button(self) -> None:
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
                    if entry:
                        wx.CallAfter(self._on_stream_title, station.stationuuid, entry)
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
        line = entry.display_line()
        self._set_playback_title(line)
        self.now_playing.SetLabel(f"{line}\n{self.current_station.name}")
        if self.track_session and self.track_session.add(entry):
            self._send_lastfm(entry)
            self._set_status(f"Tracked: {line}")

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
        elif not self.player.is_playing() and not self.player.is_paused():
            self.play_station(station)

    def stop_tracking(self) -> None:
        if self.track_session:
            self._set_status(f"Tracking saved to {self.track_session.path}")
            self._notify("Playlist tracking stopped", self.track_session.show_name or self.track_session.station.name)
        self.track_session = None
        self.track_stop_at = ""

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
        client = LastFmClient(api_key=config.api_key, api_secret=config.api_secret, session_key="")
        try:
            session_key, username = client.create_session(self.state.lastfm_pending_token)
        except (LastFmError, requests.RequestException) as exc:
            self._show_error(exc)
            return
        self.state.lastfm_session_key = session_key
        self.state.lastfm_username = username
        self.state.lastfm_enabled = True
        self.state.lastfm_pending_token = ""
        self.store.save(self.state)
        self.lastfm_client = self._create_lastfm_client()
        self._set_status(f"Connected Last.fm as {username or 'your account'}.")

    def _on_connect_spotify(self, _event: wx.Event) -> None:
        config = spotify_app_config()
        if not config:
            self._set_status("Station Scout is missing its Spotify client ID.")
            return
        auth_request = build_spotify_auth_request(
            client_id=config.client_id,
            redirect_uri=config.redirect_uri,
        )
        self.state.spotify_auth_state = auth_request.state
        self.state.spotify_code_verifier = auth_request.code_verifier
        self.store.save(self.state)
        webbrowser.open(auth_request.url)
        self._set_status("Approve Station Scout in Spotify. Callback handling comes next.")

    def _finish_spotify_with_code(self, code: str, state: str) -> None:
        config = spotify_app_config()
        if not config or state != self.state.spotify_auth_state:
            self._set_status("Spotify authorization state did not match.")
            return
        client = SpotifyClient(client_id=config.client_id, redirect_uri=config.redirect_uri)
        token_set = client.exchange_code(code=code, code_verifier=self.state.spotify_code_verifier)
        self.state.spotify_access_token = token_set.access_token
        self.state.spotify_refresh_token = token_set.refresh_token
        self.state.spotify_token_expires_at = token_set.expires_at
        self.state.spotify_enabled = True
        self.state.spotify_auth_state = ""
        self.state.spotify_code_verifier = ""
        self.store.save(self.state)
        self._set_status("Connected Spotify.")

    def _send_lastfm(self, entry) -> None:
        if not self.lastfm_client:
            return
        try:
            self.lastfm_client.update_now_playing(entry)
            self.lastfm_client.scrobble(entry)
        except (LastFmError, requests.RequestException):
            self.lastfm_cache.append(entry)

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
        lastfm_buttons = wx.BoxSizer(wx.HORIZONTAL)
        self.connect_lastfm_button = wx.Button(self, label="Connect Last.fm")
        self.finish_lastfm_button = wx.Button(self, label="Finish Last.fm")
        lastfm_buttons.Add(self.connect_lastfm_button, 0, wx.RIGHT, 8)
        lastfm_buttons.Add(self.finish_lastfm_button, 0)
        lastfm_box.Add(self.lastfm_status, 0, wx.EXPAND | wx.ALL, 12)
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
        self.Bind(wx.EVT_CHAR_HOOK, self._on_char_hook)
        self._refresh()
        self.log_folder_button.SetFocus()

    def _refresh(self) -> None:
        state = self.parent_frame.state
        self.log_folder_value.SetValue(str(self.parent_frame.store.track_log_folder(state)))
        if state.lastfm_enabled:
            self.lastfm_status.SetLabel(f"Connected as {state.lastfm_username or 'your account'}.")
        elif state.lastfm_pending_token:
            self.lastfm_status.SetLabel("Waiting for browser approval. Choose Finish Last.fm afterward.")
        else:
            self.lastfm_status.SetLabel("Not connected.")
        self.spotify_status.SetLabel("Connected." if state.spotify_enabled else "Not connected.")

    def _on_choose_log_folder(self, _event: wx.Event) -> None:
        self.parent_frame._on_choose_log_folder(_event)
        self._refresh()

    def _on_connect_lastfm(self, _event: wx.Event) -> None:
        self.parent_frame._on_connect_lastfm(_event)
        self._refresh()

    def _on_finish_lastfm(self, _event: wx.Event) -> None:
        self.parent_frame._on_finish_lastfm(_event)
        self._refresh()

    def _on_connect_spotify(self, _event: wx.Event) -> None:
        self.parent_frame._on_connect_spotify(_event)
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
