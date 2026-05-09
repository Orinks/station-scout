from __future__ import annotations

import datetime as dt
import threading
import webbrowser
from collections.abc import Callable

import requests
import wx
import wx.adv
import wx.media

from station_scout.direct_streams import station_from_direct_url, streamurl_search_url
from station_scout.integrations_config import lastfm_app_config, spotify_app_config
from station_scout.lastfm import LastFmClient, LastFmError, LastFmScrobbleCache
from station_scout.models import Station, TuneTimer
from station_scout.notifications import create_notifier
from station_scout.radio_browser import RadioBrowserClient, RadioBrowserError
from station_scout.schedule import due_timers
from station_scout.storage import SettingsStore, add_unique_station
from station_scout.spotify import SpotifyClient, build_spotify_auth_request
from station_scout.tracklog import IcyMetadataReader, TrackSessionLog, parse_stream_title


APP_TITLE = "Station Scout"


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
        self.track_stop_event = threading.Event()
        self.track_reader = IcyMetadataReader()
        self.lastfm_client = self._create_lastfm_client()
        self.lastfm_cache = LastFmScrobbleCache(self.store.path.parent / "lastfm-scrobble-cache.jsonl")

        self._build_controls()
        self._bind_events()
        self._build_tray()
        self._refresh_saved_lists()
        self._set_status("Ready.")

        self.timer = wx.Timer(self)
        self.Bind(wx.EVT_TIMER, self._on_tick, self.timer)
        self.timer.Start(30_000)

    def _build_controls(self) -> None:
        panel = wx.Panel(self)
        root = wx.BoxSizer(wx.VERTICAL)

        search_box = wx.StaticBoxSizer(wx.VERTICAL, panel, "Search stations")
        source_row = wx.BoxSizer(wx.HORIZONTAL)
        self.search_source = wx.Choice(
            panel,
            choices=["Radio Browser", "Direct stream URL search"],
            name="Search source",
        )
        self.search_source.SetSelection(0)
        source_row.Add(wx.StaticText(panel, label="Source"), 0, wx.ALIGN_CENTER_VERTICAL | wx.RIGHT, 8)
        source_row.Add(self.search_source, 0)
        search_box.Add(source_row, 0, wx.EXPAND | wx.ALL, 8)

        form = wx.FlexGridSizer(2, 4, 6, 8)
        form.AddGrowableCol(1)
        form.AddGrowableCol(3)

        self.name_input = wx.TextCtrl(panel, name="Station name or call letters")
        self.country_input = wx.TextCtrl(panel, name="Country")
        self.language_input = wx.TextCtrl(panel, name="Language")
        self.tag_input = wx.TextCtrl(panel, name="Tag")
        self.name_label = wx.StaticText(panel, label="Name")
        self.country_label = wx.StaticText(panel, label="Country")
        self.language_label = wx.StaticText(panel, label="Language")
        self.tag_label = wx.StaticText(panel, label="Tag")
        for label, control in (
            (self.name_label, self.name_input),
            (self.country_label, self.country_input),
            (self.language_label, self.language_input),
            (self.tag_label, self.tag_input),
        ):
            form.Add(label, 0, wx.ALIGN_CENTER_VERTICAL)
            form.Add(control, 1, wx.EXPAND)
        search_box.Add(form, 0, wx.EXPAND | wx.ALL, 8)

        buttons = wx.BoxSizer(wx.HORIZONTAL)
        self.search_button = wx.Button(panel, label="Search")
        self.open_direct_search_button = wx.Button(panel, label="Open direct URL search")
        self.play_button = wx.Button(panel, label="Play selected")
        self.stop_button = wx.Button(panel, label="Stop")
        self.favorite_button = wx.Button(panel, label="Add favorite")
        self.website_button = wx.Button(panel, label="Open website")
        self.timer_button = wx.Button(panel, label="Add tune-in timer")
        self.start_tracking_button = wx.Button(panel, label="Start tracking")
        self.stop_tracking_button = wx.Button(panel, label="Stop tracking")
        self.log_folder_button = wx.Button(panel, label="Choose log folder")
        self.connect_lastfm_button = wx.Button(panel, label="Connect Last.fm")
        self.finish_lastfm_button = wx.Button(panel, label="Finish Last.fm")
        self.connect_spotify_button = wx.Button(panel, label="Connect Spotify")
        for button in (
            self.search_button,
            self.open_direct_search_button,
            self.play_button,
            self.stop_button,
            self.favorite_button,
            self.website_button,
            self.timer_button,
            self.start_tracking_button,
            self.stop_tracking_button,
            self.log_folder_button,
            self.connect_lastfm_button,
            self.finish_lastfm_button,
            self.connect_spotify_button,
        ):
            buttons.Add(button, 0, wx.RIGHT, 8)
        search_box.Add(buttons, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 8)

        body = wx.BoxSizer(wx.HORIZONTAL)
        self.station_list = wx.ListCtrl(panel, style=wx.LC_REPORT | wx.LC_SINGLE_SEL)
        self.station_list.AppendColumn("Station", width=240)
        self.station_list.AppendColumn("Location", width=180)
        self.station_list.AppendColumn("Language", width=150)
        self.station_list.AppendColumn("Quality", width=170)
        self.station_list.AppendColumn("Source", width=140)

        side = wx.BoxSizer(wx.VERTICAL)
        self.now_playing = wx.StaticText(panel, label="Nothing playing")
        self.now_playing.Wrap(280)
        self.media = wx.media.MediaCtrl(panel, style=wx.SIMPLE_BORDER)
        self.favorites_list = wx.ListBox(panel, name="Favorites")
        self.recents_list = wx.ListBox(panel, name="Recent stations")
        self.timers_list = wx.ListBox(panel, name="Tune-in timers")
        side.Add(wx.StaticText(panel, label="Now playing"), 0, wx.BOTTOM, 4)
        side.Add(self.now_playing, 0, wx.EXPAND | wx.BOTTOM, 8)
        side.Add(self.media, 0, wx.EXPAND | wx.BOTTOM, 12)
        side.Add(wx.StaticText(panel, label="Favorites"), 0, wx.BOTTOM, 4)
        side.Add(self.favorites_list, 1, wx.EXPAND | wx.BOTTOM, 8)
        side.Add(wx.StaticText(panel, label="Recent stations"), 0, wx.BOTTOM, 4)
        side.Add(self.recents_list, 1, wx.EXPAND | wx.BOTTOM, 8)
        side.Add(wx.StaticText(panel, label="Timers"), 0, wx.BOTTOM, 4)
        side.Add(self.timers_list, 1, wx.EXPAND)

        body.Add(self.station_list, 2, wx.EXPAND | wx.RIGHT, 10)
        body.Add(side, 1, wx.EXPAND)

        self.status = wx.StaticText(panel, label="")
        root.Add(search_box, 0, wx.EXPAND | wx.ALL, 10)
        root.Add(body, 1, wx.EXPAND | wx.LEFT | wx.RIGHT, 10)
        root.Add(self.status, 0, wx.EXPAND | wx.ALL, 10)
        panel.SetSizer(root)

    def _bind_events(self) -> None:
        self.Bind(wx.EVT_BUTTON, self._on_search, self.search_button)
        self.Bind(wx.EVT_BUTTON, self._on_open_direct_search, self.open_direct_search_button)
        self.Bind(wx.EVT_BUTTON, self._on_play_selected, self.play_button)
        self.Bind(wx.EVT_BUTTON, lambda _event: self.stop_playback(), self.stop_button)
        self.Bind(wx.EVT_BUTTON, self._on_add_favorite, self.favorite_button)
        self.Bind(wx.EVT_BUTTON, self._on_open_website, self.website_button)
        self.Bind(wx.EVT_BUTTON, self._on_add_timer, self.timer_button)
        self.Bind(wx.EVT_BUTTON, self._on_start_tracking, self.start_tracking_button)
        self.Bind(wx.EVT_BUTTON, lambda _event: self.stop_tracking(), self.stop_tracking_button)
        self.Bind(wx.EVT_BUTTON, self._on_choose_log_folder, self.log_folder_button)
        self.Bind(wx.EVT_BUTTON, self._on_connect_lastfm, self.connect_lastfm_button)
        self.Bind(wx.EVT_BUTTON, self._on_finish_lastfm, self.finish_lastfm_button)
        self.Bind(wx.EVT_BUTTON, self._on_connect_spotify, self.connect_spotify_button)
        self.Bind(wx.EVT_CHOICE, self._on_search_source_changed, self.search_source)
        self.station_list.Bind(wx.EVT_LIST_ITEM_ACTIVATED, self._on_play_selected)
        self.favorites_list.Bind(wx.EVT_LISTBOX_DCLICK, lambda _event: self._play_saved("favorites"))
        self.recents_list.Bind(wx.EVT_LISTBOX_DCLICK, lambda _event: self._play_saved("recents"))
        self.Bind(wx.EVT_CLOSE, self._on_close)

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
        menu.Append(stop_id, "Stop playback")
        menu.AppendSeparator()
        menu.Append(quit_id, "Quit")
        self.tray.Bind(wx.EVT_MENU, lambda _event: self._show_from_tray(), id=show_id)
        self.tray.Bind(wx.EVT_MENU, lambda _event: self.stop_playback(), id=stop_id)
        self.tray.Bind(wx.EVT_MENU, lambda _event: self.Close(), id=quit_id)
        self.tray.PopupMenu(menu)
        menu.Destroy()
        event.Skip()

    def _on_search(self, _event: wx.Event) -> None:
        source = self.search_source.GetStringSelection()
        self._set_status(f"Searching {source}...")
        self.search_button.Disable()
        if source == "Direct stream URL search":
            try:
                station = station_from_direct_url(
                    name=self.name_input.GetValue().strip(),
                    url=self.tag_input.GetValue().strip(),
                    homepage=self.country_input.GetValue().strip(),
                )
            except ValueError as exc:
                self._show_error(exc)
            else:
                self._show_results([station])
        else:
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
        self._set_status(f"{len(stations)} stations found.")

    def _on_search_source_changed(self, _event: wx.Event) -> None:
        direct_mode = self.search_source.GetStringSelection() == "Direct stream URL search"
        self.language_input.Enable(not direct_mode)
        if direct_mode:
            self.name_label.SetLabel("Name or call letters")
            self.country_label.SetLabel("Website")
            self.language_label.SetLabel("Language")
            self.tag_label.SetLabel("Direct stream URL")
            self._set_status(
                "Open direct URL search, copy the stream URL, then paste it into Direct stream URL."
            )
        else:
            self.name_label.SetLabel("Name")
            self.country_label.SetLabel("Country")
            self.language_label.SetLabel("Language")
            self.tag_label.SetLabel("Tag")
            self._set_status("Radio Browser search is ready.")

    def _on_open_direct_search(self, _event: wx.Event) -> None:
        query = self.name_input.GetValue().strip()
        if not query:
            self._set_status("Enter a station name or call letters first.")
            return
        webbrowser.open(streamurl_search_url(query))
        self._set_status("Opened StreamURL.link search in your browser.")

    def _on_play_selected(self, _event: wx.Event) -> None:
        station = self._selected_station()
        if station:
            self.play_station(station)

    def play_station(self, station: Station) -> None:
        if not station.url_resolved:
            self._show_error(RadioBrowserError("Selected station does not have a stream URL."))
            return
        self.current_station = station
        self.state.recents = add_unique_station(self.state.recents, station, limit=20)
        self.store.save(self.state)
        self._refresh_saved_lists()
        self.now_playing.SetLabel(f"{station.name}\n{station.subtitle()}\n{station.quality_label()}")
        self._set_status(f"Loading {station.name}...")
        self._notify("Station Scout", f"Tuning in to {station.name}")
        if station.source == "Radio Browser":
            self._run_background(
                lambda: self.client.click_url(station.stationuuid),
                self._play_url,
                self._show_error,
            )
        else:
            self._play_url(station.url_resolved)

    def _play_url(self, url: str) -> None:
        if not url:
            self._show_error(RadioBrowserError("Radio Browser did not return a stream URL."))
            return
        if self.media.Load(url):
            self.media.Play()
            if self.current_station:
                self.current_station = Station.from_api({**self.current_station.to_json(), "url_resolved": url})
            self._set_status("Playing.")
        else:
            self._show_error(RadioBrowserError("wxPython could not load this stream."))

    def stop_playback(self) -> None:
        self.media.Stop()
        self._set_status("Stopped.")

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
        self.status.SetLabel(text)

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
        self.stop_tracking()
        self.track_stop_event.clear()
        log_root = self.store.track_log_folder(self.state)
        self.track_session = TrackSessionLog(root=log_root, station=station, show_name=show_name)
        self._set_status(f"Tracking songs to {self.track_session.path}")
        self._notify("Playlist tracking started", show_name or station.name)

        def worker() -> None:
            try:
                for raw_title in self.track_reader.titles(station.url_resolved):
                    if self.track_stop_event.is_set() or _stop_time_reached(stop_at):
                        break
                    entry = parse_stream_title(raw_title)
                    if entry and self.track_session and self.track_session.add(entry):
                        self._send_lastfm(entry)
                        wx.CallAfter(self._set_status, f"Tracked: {entry.display_line()}")
            except BaseException as exc:
                wx.CallAfter(self._set_status, f"Tracking stopped: {exc}")
            finally:
                wx.CallAfter(self._notify, "Playlist tracking stopped", show_name or station.name)

        threading.Thread(target=worker, daemon=True).start()

    def stop_tracking(self) -> None:
        self.track_stop_event.set()
        if self.track_session:
            self._set_status(f"Tracking saved to {self.track_session.path}")
        self.track_session = None

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
        self.show_name = wx.TextCtrl(self, name="Show name")
        end_row = wx.BoxSizer(wx.HORIZONTAL)
        end_row.Add(wx.StaticText(self, label="End time"), 0, wx.ALIGN_CENTER_VERTICAL | wx.RIGHT, 8)
        self.end_time_picker = wx.adv.TimePickerCtrl(self)
        end_row.Add(self.end_time_picker, 0)
        root.Add(row, 0, wx.ALL, 12)
        root.Add(self.auto_play, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(self.track_playlist, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(wx.StaticText(self, label="Show name"), 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 4)
        root.Add(self.show_name, 0, wx.EXPAND | wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(end_row, 0, wx.LEFT | wx.RIGHT | wx.BOTTOM, 12)
        root.Add(self.CreateSeparatedButtonSizer(wx.OK | wx.CANCEL), 0, wx.EXPAND | wx.ALL, 12)
        self.SetSizerAndFit(root)

    def time_value(self) -> str:
        value = self.time_picker.GetValue()
        return f"{value.GetHour():02d}:{value.GetMinute():02d}"

    def end_time_value(self) -> str:
        value = self.end_time_picker.GetValue()
        return f"{value.GetHour():02d}:{value.GetMinute():02d}"


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
