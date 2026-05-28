pub const APP_TITLE: &str = "Station Scout";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiBlueprint {
    pub title: &'static str,
    pub status_fields: &'static [&'static str],
    pub initial_focus: &'static str,
    pub first_run_controls: &'static [&'static str],
    pub menu_titles: &'static [&'static str],
    pub menu_items: &'static [&'static str],
    pub sections: &'static [&'static str],
    pub primary_actions: &'static [&'static str],
    pub list_names: &'static [&'static str],
    pub station_activation_lists: &'static [&'static str],
    pub notebook_pages: &'static [&'static str],
    pub now_playing_controls: &'static [&'static str],
    pub now_playing_status_fields: &'static [&'static str],
    pub playback_shortcuts: &'static [&'static str],
    pub playback_speed_choices: &'static [&'static str],
    pub sleep_timer_choices: &'static [&'static str],
    pub per_show_notification_choices: &'static [&'static str],
    pub label_order_pairs: &'static [(&'static str, &'static str)],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiStartupState {
    pub show_first_run_panel: bool,
    pub show_main_workspace: bool,
    pub initial_focus: &'static str,
    pub status: &'static str,
}

pub fn startup_state_for_podcast_subscriptions(has_active_subscriptions: bool) -> UiStartupState {
    if has_active_subscriptions {
        UiStartupState {
            show_first_run_panel: false,
            show_main_workspace: true,
            initial_focus: "Podcast subscriptions",
            status: "Ready.",
        }
    } else {
        UiStartupState {
            show_first_run_panel: true,
            show_main_workspace: true,
            initial_focus: "Favorite stations",
            status: "Ready. No podcast subscriptions yet.",
        }
    }
}

pub fn station_scout_ui_blueprint() -> UiBlueprint {
    UiBlueprint {
        title: APP_TITLE,
        status_fields: &["Status", "Playback"],
        initial_focus: "Favorite stations",
        first_run_controls: &[
            "First run - Subscribe by URL",
            "First run - Import OPML",
            "First run - Search podcasts",
        ],
        menu_titles: &["File", "Playback"],
        menu_items: &[
            "Settings",
            "Quit",
            "Focus search",
            "Play selected",
            "Play/Pause",
            "Stop",
            "Volume up",
            "Volume down",
            "Play on this computer",
            "Play on Sonos",
            "Cast to Chromecast",
            "Add tune-in timer",
            "Start tracking",
            "Stop tracking",
        ],
        sections: &[
            "Station",
            "Now Playing",
            "Search Radio Browser",
            "Browse Stations",
            "Stations",
            "Saved Stations",
            "Tune-in Timers",
            "Recent Events",
            "Podcast Subscriptions",
            "Podcast Search",
            "Episode List",
            "Queue / Up Next",
            "Podcast Inbox",
            "Podcast Settings",
            "Per-show Settings",
            "Last.fm",
            "Show Notes",
            "Transcript",
        ],
        primary_actions: &[
            "Play selected",
            "Add favorite",
            "Open website",
            "Add tune-in timer",
            "Start tracking",
            "Stop tracking",
            "Subscribe by feed URL",
            "Subscribe by URL",
            "Search podcasts",
            "Run podcast search",
            "Import OPML",
            "Export OPML",
            "Check for new episodes now",
            "Per-show settings for selected show",
            "Unsubscribe selected show",
            "Per-show settings",
            "Play selected episode",
            "Add selected episode to queue",
            "Mark selected episode as played",
            "Add episode to queue",
            "Mark episode played",
            "Move selected queue item up",
            "Move selected queue item down",
            "Remove selected queue item",
            "Clear queue",
            "Open selected inbox entry",
            "Clear inbox",
            "Connect Last.fm",
            "Finish Last.fm",
        ],
        list_names: &[
            "Station search results",
            "Browse choices",
            "Favorite stations",
            "Recently played stations",
            "Tune-in timers",
            "Subscribed shows",
            "Podcast subscriptions",
            "Podcast search results",
            "Episode list for selected show",
            "Episode list",
            "Playback queue",
            "Podcast queue",
            "Notification inbox",
            "Podcast inbox",
        ],
        station_activation_lists: &[
            "Station search results",
            "Favorite stations",
            "Recently played stations",
        ],
        notebook_pages: &["Subscriptions", "Episodes", "Queue", "Inbox"],
        now_playing_controls: &["Stop current playback"],
        now_playing_status_fields: &["Now-playing episode title", "Now-playing status"],
        playback_shortcuts: &[
            "Ctrl+F",
            "Ctrl+P",
            "Ctrl+Space",
            "Ctrl+S",
            "Ctrl+Up",
            "Ctrl+Down",
            "Ctrl+T",
            "Ctrl+Shift+T",
            "Ctrl+Shift+S",
        ],
        playback_speed_choices: &["0.8x", "1.0x", "1.25x", "1.5x", "1.75x", "2.0x"],
        sleep_timer_choices: &[
            "Off",
            "5 min",
            "10 min",
            "15 min",
            "30 min",
            "45 min",
            "60 min",
            "End of episode",
        ],
        per_show_notification_choices: &["Always", "Digest", "Never"],
        label_order_pairs: &[
            ("Station", "Station selection"),
            ("Name", "Station name or call letters"),
            ("Country", "Country"),
            ("Language", "Language"),
            ("Tag", "Tag"),
            ("Browse by", "Browse category"),
            ("Choice", "Browse choices"),
            ("Station results", "Station search results"),
            ("Favorite stations", "Favorite stations"),
            ("Recently played stations", "Recently played stations"),
            ("Timers", "Tune-in timers"),
            ("Subscriptions", "Subscribed shows"),
            ("Search term", "Podcast search query"),
            ("Feed URL", "Podcast feed URL"),
            ("Results", "Podcast search results"),
            ("Episodes", "Episode list for selected show"),
            ("Show notes", "Show notes for selected episode"),
            ("Transcript", "Transcript for selected episode"),
            ("Queue (up next)", "Playback queue"),
            ("Inbox", "Notification inbox"),
            ("Notifications", "Per-show notification rule"),
            ("Skip silence", "Skip silence for this show"),
            ("Skip intro (seconds)", "Intro skip seconds"),
            ("Skip outro (seconds)", "Outro skip seconds"),
            ("Polling interval (minutes)", "Polling interval in minutes"),
            ("Quiet hours", "Enable quiet hours"),
            ("Quiet hours start (HH:MM)", "Quiet hours start time"),
            ("Quiet hours end (HH:MM)", "Quiet hours end time"),
            ("Startup", "Start with Windows"),
            ("Default playback speed", "Default playback speed"),
            (
                "Default sleep timer (minutes)",
                "Default sleep timer minutes",
            ),
            ("OpenRouter model", "OpenRouter model"),
            ("Podcast Index API key", "Podcast Index API key"),
            ("Podcast Index API secret", "Podcast Index API secret"),
            ("Last.fm status", "Last.fm connection status"),
            ("Scrobbling", "Scrobble played songs to Last.fm"),
            ("Speed", "Playback speed"),
            ("Sleep timer", "Sleep timer"),
            ("Show notes", "Show notes"),
            ("Transcript", "Transcript"),
        ],
    }
}

pub fn playback_window_title(now_playing: &str) -> String {
    let now_playing = now_playing.split_whitespace().collect::<Vec<_>>().join(" ");
    if now_playing.is_empty() {
        APP_TITLE.to_string()
    } else {
        format!("{now_playing} - {APP_TITLE}")
    }
}

#[cfg(feature = "gui")]
pub mod wx {
    use super::{
        APP_TITLE, UiStartupState, startup_state_for_podcast_subscriptions,
        station_scout_ui_blueprint,
    };
    use crate::lastfm::lastfm_settings_view_state;
    use crate::models::{BrowseFacet, BrowseFacetKind, Station, TuneTimer};
    use crate::player::gstreamer_radio_player;
    use crate::player::{RadioPlayer, play_podcast_episode_stream, play_station_stream};
    use crate::podcasts::discovery::PodcastSearchClient;
    use crate::podcasts::repository::EpisodeRow;
    use crate::radio_browser::RadioBrowserClient;
    use crate::runtime::StationScoutRuntime;
    use crate::storage::system_credential_store;
    use crate::ui_actions::{
        PodcastUiModel, podcast_search_result_label, station_label, timer_label,
    };
    use chrono::{Duration, Utc};
    use std::cell::RefCell;
    use std::fs;
    use std::rc::Rc;
    use wxdragon::prelude::*;

    const MENU_ID_SETTINGS: Id = 32_001;
    const MENU_ID_FOCUS_SEARCH: Id = 32_002;
    const MENU_ID_PLAY_SELECTED: Id = 32_003;
    const MENU_ID_TOGGLE_PAUSE: Id = 32_004;
    const MENU_ID_STOP_PLAYBACK: Id = 32_005;
    const MENU_ID_VOLUME_UP: Id = 32_006;
    const MENU_ID_VOLUME_DOWN: Id = 32_007;
    const MENU_ID_PLAY_ON_COMPUTER: Id = 32_008;
    const MENU_ID_CAST_SONOS: Id = 32_009;
    const MENU_ID_CAST_CHROMECAST: Id = 32_010;
    const MENU_ID_ADD_TIMER: Id = 32_011;
    const MENU_ID_START_TRACKING: Id = 32_012;
    const MENU_ID_STOP_TRACKING: Id = 32_013;
    const VOLUME_STEP: f64 = 0.1;

    #[derive(Clone, Copy)]
    struct MainControls {
        frame: Frame,
        status: StaticText,
    }

    #[derive(Clone, Copy)]
    struct FirstRunControls {
        subscribe: Button,
        import_opml: Button,
        search_podcasts: Button,
    }

    #[derive(Clone, Copy)]
    struct PodcastControls {
        query: TextCtrl,
        feed_url: TextCtrl,
        run_search: Button,
        subscribe: Button,
        check_now: Button,
        unsubscribe: Button,
        play_episode: Button,
        add_episode_to_queue: Button,
        mark_episode_played: Button,
        move_queue_up: Button,
        move_queue_down: Button,
        remove_queue_item: Button,
        clear_queue: Button,
        clear_inbox: Button,
        subscriptions: ListBox,
        search_results: ListBox,
        episodes: ListBox,
        queue: ListBox,
        inbox: ListBox,
    }

    #[derive(Clone, Copy)]
    struct RadioControls {
        name: TextCtrl,
        country: TextCtrl,
        language: TextCtrl,
        tag: TextCtrl,
        search: Button,
        browse_kind: Choice,
        load_browse: Button,
        browse_choices: ListBox,
        play_selected: Button,
        add_favorite: Button,
        open_website: Button,
        add_timer: Button,
        start_tracking: Button,
        stop_tracking: Button,
        station_results: ListCtrl,
        favorites: ListBox,
        recents: ListBox,
        timers: ListBox,
    }

    #[derive(Clone, Copy)]
    struct PlaybackControls {
        title: StaticText,
        status: StaticText,
        stop: Button,
    }

    #[derive(Clone, Copy)]
    struct PodcastSearchControls {
        query: TextCtrl,
        feed_url: TextCtrl,
        run_search: Button,
    }

    #[derive(Clone, Copy)]
    struct PodcastNotebookControls {
        subscribe: Button,
        check_now: Button,
        unsubscribe: Button,
        play_episode: Button,
        add_episode_to_queue: Button,
        mark_episode_played: Button,
        move_queue_up: Button,
        move_queue_down: Button,
        remove_queue_item: Button,
        clear_queue: Button,
        clear_inbox: Button,
        subscriptions: ListBox,
        search_results: ListBox,
        episodes: ListBox,
        queue: ListBox,
        inbox: ListBox,
    }

    #[derive(Default)]
    struct PodcastListState {
        subscription_ids: Vec<i64>,
        episode_rows: Vec<EpisodeRow>,
        episode_ids: Vec<i64>,
        queue_episode_ids: Vec<i64>,
        search_feed_urls: Vec<String>,
    }

    #[derive(Default)]
    struct RadioListState {
        station_results: Vec<Station>,
        browse_facets: Vec<BrowseFacet>,
        favorite_stations: Vec<Station>,
        recent_stations: Vec<Station>,
        active_source: RadioSelectionSource,
    }

    #[derive(Clone, Copy, Eq, PartialEq)]
    enum RadioSelectionSource {
        Any,
        SearchResults,
        Favorites,
        Recents,
    }

    impl Default for RadioSelectionSource {
        fn default() -> Self {
            Self::Favorites
        }
    }

    type SharedRuntime = Rc<RefCell<Option<StationScoutRuntime>>>;
    type SharedPodcastListState = Rc<RefCell<PodcastListState>>;
    type SharedRadioListState = Rc<RefCell<RadioListState>>;
    type SharedPlayer = Rc<RefCell<RadioPlayer>>;

    pub fn run() {
        let (startup, runtime, startup_error) =
            match StationScoutRuntime::open_default(system_credential_store()) {
                Ok(runtime) => (
                    startup_state_for_podcast_subscriptions(
                        runtime.has_active_podcast_subscriptions,
                    ),
                    Some(runtime),
                    None,
                ),
                Err(error) => (
                    startup_state_for_podcast_subscriptions(false),
                    None,
                    Some(error),
                ),
            };
        let runtime = Rc::new(RefCell::new(runtime));
        if let Err(error) =
            wxdragon::main(move |_| build_main_window(startup, runtime, startup_error))
        {
            eprintln!("Station Scout GUI failed: {error}");
        }
    }

    fn build_main_window(
        startup: UiStartupState,
        runtime: SharedRuntime,
        startup_error: Option<String>,
    ) {
        let frame = Frame::builder()
            .with_title(APP_TITLE)
            .with_size(Size::new(1180, 820))
            .build();
        set_top_window(&frame);
        add_menu_bar(&frame);
        frame.create_status_bar(2, 0, wxdragon::id::ID_ANY as i32, "status");

        let panel = Panel::builder(&frame).build();
        let root = BoxSizer::builder(Orientation::Vertical).build();

        let first_run_controls = add_first_run_panel(&panel, &root, startup);
        let (radio_controls, podcast_controls, playback_controls) =
            add_main_workspace(&panel, &root, startup);

        let status = StaticText::builder(&panel)
            .with_label(startup.status)
            .build();
        describe_control(&status, "Status");
        root.add(&status, 0, SizerFlag::All | SizerFlag::Expand, 6);

        panel.set_sizer(root, true);
        let controls = MainControls { frame, status };
        controls.status.set_label(startup.status);
        controls.frame.set_status_text(startup.status, 0);
        let initial_volume = runtime
            .borrow()
            .as_ref()
            .map(|runtime| runtime.state.volume)
            .unwrap_or(1.0);
        let mut radio_player = create_radio_player(controls);
        radio_player.set_volume(initial_volume);
        let player = Rc::new(RefCell::new(radio_player));
        let radio_state = Rc::new(RefCell::new(RadioListState::default()));
        let list_state = Rc::new(RefCell::new(PodcastListState::default()));
        wire_menu_events(
            controls,
            runtime.clone(),
            radio_controls,
            radio_state.clone(),
            player.clone(),
            playback_controls,
        );
        wire_first_run_events(
            controls,
            runtime.clone(),
            first_run_controls,
            podcast_controls,
            list_state.clone(),
        );
        wire_radio_events(
            controls,
            runtime.clone(),
            radio_controls,
            radio_state.clone(),
            player.clone(),
            playback_controls,
        );
        wire_podcast_events(
            controls,
            runtime.clone(),
            podcast_controls,
            list_state.clone(),
            player.clone(),
            playback_controls,
        );
        wire_playback_events(controls, player, playback_controls);
        if let Some(error) = startup_error {
            set_status(controls, &format!("Runtime unavailable: {error}"));
        } else if let Err(error) = refresh_radio_lists(runtime.clone(), radio_controls, radio_state)
        {
            set_status(controls, &format!("Radio refresh failed: {error}"));
        } else if let Err(error) = refresh_podcast_lists(runtime, podcast_controls, list_state) {
            set_status(controls, &format!("Podcast refresh failed: {error}"));
        }
        controls.frame.show(true);
        if startup.initial_focus == "Favorite stations" {
            radio_controls.favorites.set_focus();
        }
    }

    fn create_radio_player(controls: MainControls) -> RadioPlayer {
        match gstreamer_radio_player() {
            Ok(player) => player,
            Err(error) => {
                let message = format!("Audio playback unavailable: {error}");
                controls.status.set_label(&message);
                controls.frame.set_status_text(&message, 0);
                RadioPlayer::without_backend()
            }
        }
    }

    fn wire_menu_events(
        controls: MainControls,
        runtime: SharedRuntime,
        radio_controls: RadioControls,
        radio_state: SharedRadioListState,
        player: SharedPlayer,
        playback_controls: PlaybackControls,
    ) {
        controls.frame.on_menu_selected(move |event| {
            let result = match event.get_id() {
                MENU_ID_SETTINGS => show_settings_dialog(controls, runtime.clone()),
                MENU_ID_FOCUS_SEARCH => {
                    radio_controls.name.set_focus();
                    Ok("Search focused.".to_string())
                }
                MENU_ID_PLAY_SELECTED => play_selected_radio_from_source(
                    runtime.clone(),
                    radio_controls,
                    radio_state.clone(),
                    player.clone(),
                    playback_controls,
                    RadioSelectionSource::Any,
                ),
                MENU_ID_TOGGLE_PAUSE => toggle_playback_pause(controls, player.clone()),
                MENU_ID_STOP_PLAYBACK => stop_playback(controls, player.clone(), playback_controls),
                MENU_ID_VOLUME_UP => {
                    change_playback_volume(controls, runtime.clone(), player.clone(), VOLUME_STEP)
                }
                MENU_ID_VOLUME_DOWN => {
                    change_playback_volume(controls, runtime.clone(), player.clone(), -VOLUME_STEP)
                }
                MENU_ID_PLAY_ON_COMPUTER => Ok("Playback is set to this computer.".to_string()),
                MENU_ID_CAST_SONOS => {
                    Ok("Sonos playback is not ported to the Rust app yet.".to_string())
                }
                MENU_ID_CAST_CHROMECAST => {
                    Ok("Chromecast playback is not ported to the Rust app yet.".to_string())
                }
                MENU_ID_ADD_TIMER => {
                    add_selected_radio_timer(runtime.clone(), radio_controls, radio_state.clone())
                }
                MENU_ID_START_TRACKING => {
                    Ok("Playlist tracking is not ported to the Rust app yet.".to_string())
                }
                MENU_ID_STOP_TRACKING => Ok("Playlist tracking is not running.".to_string()),
                ID_EXIT => {
                    controls.frame.close(true);
                    Ok(String::new())
                }
                _ => Ok(String::new()),
            };
            match result {
                Ok(message) if !message.is_empty() => set_status(controls, &message),
                Ok(_) => {}
                Err(error) => set_status(controls, &error),
            }
        });
    }

    fn wire_first_run_events(
        controls: MainControls,
        runtime: SharedRuntime,
        first_run_controls: FirstRunControls,
        podcast_controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) {
        first_run_controls.subscribe.on_click(move |_| {
            podcast_controls.feed_url.set_focus();
            set_status(
                controls,
                "Enter a podcast feed URL, then choose Subscribe by URL in Subscriptions.",
            );
        });

        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            first_run_controls.import_opml.on_click(move |_| {
                report_action(
                    controls,
                    import_opml_from_dialog(
                        controls,
                        runtime.clone(),
                        podcast_controls,
                        list_state.clone(),
                    ),
                );
            });
        }

        first_run_controls.search_podcasts.on_click(move |_| {
            podcast_controls.query.set_focus();
            set_status(
                controls,
                "Enter search terms, then choose Run podcast search.",
            );
        });
    }

    fn wire_radio_events(
        controls: MainControls,
        runtime: SharedRuntime,
        radio_controls: RadioControls,
        radio_state: SharedRadioListState,
        player: SharedPlayer,
        playback_controls: PlaybackControls,
    ) {
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            radio_controls.search.on_click(move |_| {
                report_action(
                    controls,
                    search_radio(runtime.clone(), radio_controls, radio_state.clone()),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            radio_controls.load_browse.on_click(move |_| {
                report_action(
                    controls,
                    load_radio_browse_choices(runtime.clone(), radio_controls, radio_state.clone()),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            radio_controls
                .browse_choices
                .on_selection_changed(move |_| {
                    report_action(
                        controls,
                        browse_radio_stations(runtime.clone(), radio_controls, radio_state.clone()),
                    );
                });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            radio_controls.add_favorite.on_click(move |_| {
                report_action(
                    controls,
                    add_selected_radio_favorite(
                        runtime.clone(),
                        radio_controls,
                        radio_state.clone(),
                    ),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            let player = player.clone();
            radio_controls.play_selected.on_click(move |_| {
                report_action(
                    controls,
                    play_selected_radio_from_source(
                        runtime.clone(),
                        radio_controls,
                        radio_state.clone(),
                        player.clone(),
                        playback_controls,
                        RadioSelectionSource::Any,
                    ),
                );
            });
        }
        {
            let radio_state = radio_state.clone();
            radio_controls.station_results.on_item_focused(move |_| {
                radio_state.borrow_mut().active_source = RadioSelectionSource::SearchResults;
            });
        }
        {
            let radio_state = radio_state.clone();
            radio_controls.station_results.on_item_selected(move |_| {
                radio_state.borrow_mut().active_source = RadioSelectionSource::SearchResults;
            });
        }
        {
            let radio_state = radio_state.clone();
            radio_controls.favorites.on_set_focus(move |_| {
                radio_state.borrow_mut().active_source = RadioSelectionSource::Favorites;
            });
        }
        {
            let radio_state = radio_state.clone();
            radio_controls.recents.on_set_focus(move |_| {
                radio_state.borrow_mut().active_source = RadioSelectionSource::Recents;
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            let player = player.clone();
            radio_controls.station_results.on_item_activated(move |_| {
                report_action(
                    controls,
                    play_selected_radio_from_source(
                        runtime.clone(),
                        radio_controls,
                        radio_state.clone(),
                        player.clone(),
                        playback_controls,
                        RadioSelectionSource::SearchResults,
                    ),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            let player = player.clone();
            radio_controls.favorites.on_key_down(move |event| {
                if is_enter_key(&event) {
                    report_action(
                        controls,
                        play_selected_radio_from_source(
                            runtime.clone(),
                            radio_controls,
                            radio_state.clone(),
                            player.clone(),
                            playback_controls,
                            RadioSelectionSource::Favorites,
                        ),
                    );
                    event.skip(false);
                } else {
                    event.skip(true);
                }
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            let player = player.clone();
            radio_controls.favorites.on_char(move |event| {
                if is_enter_key(&event) {
                    report_action(
                        controls,
                        play_selected_radio_from_source(
                            runtime.clone(),
                            radio_controls,
                            radio_state.clone(),
                            player.clone(),
                            playback_controls,
                            RadioSelectionSource::Favorites,
                        ),
                    );
                    event.skip(false);
                } else {
                    event.skip(true);
                }
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            let player = player.clone();
            radio_controls.favorites.on_item_double_clicked(move |_| {
                report_action(
                    controls,
                    play_selected_radio_from_source(
                        runtime.clone(),
                        radio_controls,
                        radio_state.clone(),
                        player.clone(),
                        playback_controls,
                        RadioSelectionSource::Favorites,
                    ),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            let player = player.clone();
            radio_controls.recents.on_key_down(move |event| {
                if is_enter_key(&event) {
                    report_action(
                        controls,
                        play_selected_radio_from_source(
                            runtime.clone(),
                            radio_controls,
                            radio_state.clone(),
                            player.clone(),
                            playback_controls,
                            RadioSelectionSource::Recents,
                        ),
                    );
                    event.skip(false);
                } else {
                    event.skip(true);
                }
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            let player = player.clone();
            radio_controls.recents.on_char(move |event| {
                if is_enter_key(&event) {
                    report_action(
                        controls,
                        play_selected_radio_from_source(
                            runtime.clone(),
                            radio_controls,
                            radio_state.clone(),
                            player.clone(),
                            playback_controls,
                            RadioSelectionSource::Recents,
                        ),
                    );
                    event.skip(false);
                } else {
                    event.skip(true);
                }
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            let player = player.clone();
            radio_controls.recents.on_item_double_clicked(move |_| {
                report_action(
                    controls,
                    play_selected_radio_from_source(
                        runtime.clone(),
                        radio_controls,
                        radio_state.clone(),
                        player.clone(),
                        playback_controls,
                        RadioSelectionSource::Recents,
                    ),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            radio_controls.add_timer.on_click(move |_| {
                report_action(
                    controls,
                    add_selected_radio_timer(runtime.clone(), radio_controls, radio_state.clone()),
                );
            });
        }
        {
            let radio_state = radio_state.clone();
            radio_controls.open_website.on_click(move |_| {
                report_action(
                    controls,
                    selected_radio_station(radio_controls, radio_state.clone())
                        .ok_or_else(|| "Select a station first.".to_string())
                        .and_then(|station| {
                            if station.homepage.trim().is_empty() {
                                Err("Selected station has no website.".to_string())
                            } else {
                                Ok(format!("Website: {}", station.homepage))
                            }
                        }),
                );
            });
        }
        radio_controls.start_tracking.on_click(move |_| {
            set_status(
                controls,
                "Playlist tracking is not ported to the Rust app yet.",
            );
        });
        radio_controls.stop_tracking.on_click(move |_| {
            set_status(controls, "Playlist tracking is not running.");
        });
        bind_frame_station_enter_handlers(
            controls,
            runtime,
            radio_controls,
            radio_state,
            player,
            playback_controls,
        );
    }

    fn bind_frame_station_enter_handlers(
        controls: MainControls,
        runtime: SharedRuntime,
        radio_controls: RadioControls,
        radio_state: SharedRadioListState,
        player: SharedPlayer,
        playback_controls: PlaybackControls,
    ) {
        {
            let runtime = runtime.clone();
            let radio_state = radio_state.clone();
            let player = player.clone();
            controls.frame.on_key_down(move |event| {
                if let Some(source) = focused_radio_selection_source(radio_controls, &event) {
                    report_action(
                        controls,
                        play_selected_radio_from_source(
                            runtime.clone(),
                            radio_controls,
                            radio_state.clone(),
                            player.clone(),
                            playback_controls,
                            source,
                        ),
                    );
                    event.skip(false);
                } else {
                    event.skip(true);
                }
            });
        }
        controls.frame.on_char(move |event| {
            if let Some(source) = focused_radio_selection_source(radio_controls, &event) {
                report_action(
                    controls,
                    play_selected_radio_from_source(
                        runtime.clone(),
                        radio_controls,
                        radio_state.clone(),
                        player.clone(),
                        playback_controls,
                        source,
                    ),
                );
                event.skip(false);
            } else {
                event.skip(true);
            }
        });
    }

    fn focused_radio_selection_source(
        controls: RadioControls,
        event: &WindowEventData,
    ) -> Option<RadioSelectionSource> {
        if !is_enter_key(event) {
            return None;
        }
        if controls.station_results.has_focus() {
            Some(RadioSelectionSource::SearchResults)
        } else if controls.favorites.has_focus() {
            Some(RadioSelectionSource::Favorites)
        } else if controls.recents.has_focus() {
            Some(RadioSelectionSource::Recents)
        } else {
            None
        }
    }

    fn search_radio(
        runtime: SharedRuntime,
        controls: RadioControls,
        radio_state: SharedRadioListState,
    ) -> Result<String, String> {
        let stations = {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .search_radio_with_client(
                    &RadioBrowserClient::default(),
                    &controls.name.get_value(),
                    &controls.country.get_value(),
                    &controls.language.get_value(),
                    &controls.tag.get_value(),
                    50,
                )
                .map_err(|error| error.to_string())?
        };
        fill_station_results(controls.station_results, &stations);
        radio_state.borrow_mut().station_results = stations;
        Ok(format!(
            "Found {} radio stations.",
            radio_state.borrow().station_results.len()
        ))
    }

    fn load_radio_browse_choices(
        runtime: SharedRuntime,
        controls: RadioControls,
        radio_state: SharedRadioListState,
    ) -> Result<String, String> {
        let kind = selected_browse_kind(controls);
        let facets = {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .browse_radio_facets_with_client(&RadioBrowserClient::default(), kind, 100)
                .map_err(|error| error.to_string())?
        };
        let labels = facets
            .iter()
            .map(|facet| format!("{}; {} stations", facet.name, facet.station_count))
            .collect::<Vec<_>>();
        fill_list(controls.browse_choices, &labels);
        radio_state.borrow_mut().browse_facets = facets;
        Ok(format!(
            "Loaded {} browse choices.",
            radio_state.borrow().browse_facets.len()
        ))
    }

    fn browse_radio_stations(
        runtime: SharedRuntime,
        controls: RadioControls,
        radio_state: SharedRadioListState,
    ) -> Result<String, String> {
        let facet = {
            let state = radio_state.borrow();
            let index = controls
                .browse_choices
                .get_selection()
                .ok_or_else(|| "Select a browse choice first.".to_string())?
                as usize;
            state
                .browse_facets
                .get(index)
                .cloned()
                .ok_or_else(|| "Select a browse choice first.".to_string())?
        };
        let stations = {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .browse_radio_stations_with_client(&RadioBrowserClient::default(), &facet, 50)
                .map_err(|error| error.to_string())?
        };
        fill_station_results(controls.station_results, &stations);
        radio_state.borrow_mut().station_results = stations;
        Ok(format!("Loaded stations for {}.", facet.name))
    }

    fn selected_browse_kind(controls: RadioControls) -> BrowseFacetKind {
        match controls.browse_kind.get_selection().unwrap_or(0) {
            1 => BrowseFacetKind::Location,
            2 => BrowseFacetKind::Language,
            _ => BrowseFacetKind::Genre,
        }
    }

    fn add_selected_radio_favorite(
        runtime: SharedRuntime,
        controls: RadioControls,
        radio_state: SharedRadioListState,
    ) -> Result<String, String> {
        let station = selected_radio_station(controls, radio_state.clone())
            .ok_or_else(|| "Select a station first.".to_string())?;
        {
            let mut borrowed = runtime.borrow_mut();
            let runtime = borrowed
                .as_mut()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime.add_radio_favorite(station.clone())?;
            runtime.save_settings()?;
        }
        refresh_radio_lists(runtime, controls, radio_state)?;
        Ok(format!("Added favorite: {}.", station.name))
    }

    fn play_selected_radio_from_source(
        runtime: SharedRuntime,
        controls: RadioControls,
        radio_state: SharedRadioListState,
        player: SharedPlayer,
        playback_controls: PlaybackControls,
        source: RadioSelectionSource,
    ) -> Result<String, String> {
        let station = selected_radio_station_from_source(controls, radio_state.clone(), source)
            .ok_or_else(|| "Select a station first.".to_string())?;
        let message = play_station_stream(&mut player.borrow_mut(), &station)?;
        update_now_playing(
            playback_controls,
            &station.name,
            &format!("Station stream: {}", station.url_resolved),
        );
        {
            let mut borrowed = runtime.borrow_mut();
            let runtime = borrowed
                .as_mut()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime.record_radio_recent(station.clone())?;
            runtime.save_settings()?;
        }
        refresh_radio_lists(runtime, controls, radio_state)?;
        Ok(message)
    }

    fn add_selected_radio_timer(
        runtime: SharedRuntime,
        controls: RadioControls,
        radio_state: SharedRadioListState,
    ) -> Result<String, String> {
        let station = selected_radio_station(controls, radio_state.clone())
            .ok_or_else(|| "Select a station first.".to_string())?;
        let timer = TuneTimer::new(&station.stationuuid, &station.name, "09:00");
        {
            let mut borrowed = runtime.borrow_mut();
            let runtime = borrowed
                .as_mut()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime.add_tune_timer(timer)?;
            runtime.save_settings()?;
        }
        refresh_radio_lists(runtime, controls, radio_state)?;
        Ok(format!("Added 09:00 tune-in timer for {}.", station.name))
    }

    fn refresh_radio_lists(
        runtime: SharedRuntime,
        controls: RadioControls,
        radio_state: SharedRadioListState,
    ) -> Result<(), String> {
        let (favorites, recents, timers) = {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            (
                runtime.state.favorites.clone(),
                runtime.state.recents.clone(),
                runtime.state.timers.clone(),
            )
        };
        fill_list(
            controls.favorites,
            &favorites.iter().map(station_label).collect::<Vec<_>>(),
        );
        fill_list(
            controls.recents,
            &recents.iter().map(station_label).collect::<Vec<_>>(),
        );
        fill_list(
            controls.timers,
            &timers.iter().map(timer_label).collect::<Vec<_>>(),
        );
        let mut state = radio_state.borrow_mut();
        state.favorite_stations = favorites;
        state.recent_stations = recents;
        Ok(())
    }

    fn selected_radio_station(
        controls: RadioControls,
        radio_state: SharedRadioListState,
    ) -> Option<Station> {
        selected_radio_station_from_source(controls, radio_state, RadioSelectionSource::Any)
    }

    fn selected_radio_station_from_source(
        controls: RadioControls,
        radio_state: SharedRadioListState,
        source: RadioSelectionSource,
    ) -> Option<Station> {
        let state = radio_state.borrow();
        let selected_result = || {
            let index = controls.station_results.get_first_selected_item();
            (index >= 0)
                .then_some(index as usize)
                .and_then(|index| state.station_results.get(index).cloned())
        };
        let selected_favorite = || {
            controls
                .favorites
                .get_selection()
                .and_then(|index| state.favorite_stations.get(index as usize).cloned())
        };
        let selected_recent = || {
            controls
                .recents
                .get_selection()
                .and_then(|index| state.recent_stations.get(index as usize).cloned())
        };
        match source {
            RadioSelectionSource::Any => match state.active_source {
                RadioSelectionSource::SearchResults => selected_result()
                    .or_else(selected_favorite)
                    .or_else(selected_recent),
                RadioSelectionSource::Favorites => selected_favorite()
                    .or_else(selected_recent)
                    .or_else(selected_result),
                RadioSelectionSource::Recents => selected_recent()
                    .or_else(selected_favorite)
                    .or_else(selected_result),
                RadioSelectionSource::Any => selected_favorite()
                    .or_else(selected_recent)
                    .or_else(selected_result),
            },
            RadioSelectionSource::SearchResults => selected_result(),
            RadioSelectionSource::Favorites => selected_favorite(),
            RadioSelectionSource::Recents => selected_recent(),
        }
    }

    fn fill_station_results(list: ListCtrl, stations: &[Station]) {
        list.delete_all_items();
        for (index, station) in stations.iter().enumerate() {
            let index = index as i64;
            list.insert_item(index, &station.name, None);
            list.set_item_text_by_column(index, 1, &station.subtitle());
            list.set_item_text_by_column(index, 2, &station.discovery_quality_label());
        }
        if !stations.is_empty() {
            list.set_item_state(0, ListItemState::Selected, ListItemState::Selected);
        }
    }

    fn wire_podcast_events(
        controls: MainControls,
        runtime: SharedRuntime,
        podcast_controls: PodcastControls,
        list_state: SharedPodcastListState,
        player: SharedPlayer,
        playback_controls: PlaybackControls,
    ) {
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls.run_search.on_click(move |_| {
                report_action(
                    controls,
                    run_podcast_search(runtime.clone(), podcast_controls, list_state.clone()),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls.subscribe.on_click(move |_| {
                report_action(
                    controls,
                    subscribe_to_feed_from_ui(
                        runtime.clone(),
                        podcast_controls,
                        list_state.clone(),
                    ),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls.check_now.on_click(move |_| {
                report_action(
                    controls,
                    check_for_new_episodes(runtime.clone(), podcast_controls, list_state.clone()),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls
                .subscriptions
                .on_selection_changed(move |_| {
                    report_action(
                        controls,
                        refresh_podcast_lists(
                            runtime.clone(),
                            podcast_controls,
                            list_state.clone(),
                        )
                        .map(|_| "Episodes refreshed.".to_string()),
                    );
                });
        }
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls.unsubscribe.on_click(move |_| {
                report_action(
                    controls,
                    unsubscribe_selected_show(
                        runtime.clone(),
                        podcast_controls,
                        list_state.clone(),
                    ),
                );
            });
        }
        {
            let list_state = list_state.clone();
            let player = player.clone();
            podcast_controls.play_episode.on_click(move |_| {
                report_action(
                    controls,
                    play_selected_podcast_episode(
                        podcast_controls,
                        list_state.clone(),
                        player.clone(),
                        playback_controls,
                    ),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls.add_episode_to_queue.on_click(move |_| {
                report_action(
                    controls,
                    queue_selected_episode(runtime.clone(), podcast_controls, list_state.clone()),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls.mark_episode_played.on_click(move |_| {
                report_action(
                    controls,
                    mark_selected_episode_played(
                        runtime.clone(),
                        podcast_controls,
                        list_state.clone(),
                    ),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls.move_queue_up.on_click(move |_| {
                report_action(
                    controls,
                    move_selected_queue_item(
                        runtime.clone(),
                        podcast_controls,
                        list_state.clone(),
                        -1,
                    ),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls.move_queue_down.on_click(move |_| {
                report_action(
                    controls,
                    move_selected_queue_item(
                        runtime.clone(),
                        podcast_controls,
                        list_state.clone(),
                        1,
                    ),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls.remove_queue_item.on_click(move |_| {
                report_action(
                    controls,
                    remove_selected_queue_item(
                        runtime.clone(),
                        podcast_controls,
                        list_state.clone(),
                    ),
                );
            });
        }
        {
            let runtime = runtime.clone();
            let list_state = list_state.clone();
            podcast_controls.clear_queue.on_click(move |_| {
                report_action(
                    controls,
                    clear_queue(runtime.clone(), podcast_controls, list_state.clone()),
                );
            });
        }
        podcast_controls.clear_inbox.on_click(move |_| {
            report_action(
                controls,
                clear_inbox(runtime.clone(), podcast_controls, list_state.clone()),
            );
        });
    }

    fn wire_playback_events(
        controls: MainControls,
        player: SharedPlayer,
        playback_controls: PlaybackControls,
    ) {
        playback_controls.stop.on_click(move |_| {
            report_action(
                controls,
                stop_playback(controls, player.clone(), playback_controls),
            );
        });
    }

    fn toggle_playback_pause(
        _controls: MainControls,
        player: SharedPlayer,
    ) -> Result<String, String> {
        let mut player = player.borrow_mut();
        if player.toggle_pause() {
            Ok("Toggled playback pause.".to_string())
        } else if let Some(error) = player.last_error() {
            Err(error.to_string())
        } else {
            Ok("No active playback to pause or resume.".to_string())
        }
    }

    fn stop_playback(
        _controls: MainControls,
        player: SharedPlayer,
        playback_controls: PlaybackControls,
    ) -> Result<String, String> {
        player.borrow_mut().stop();
        update_now_playing(playback_controls, "Nothing playing", "");
        Ok("Playback stopped.".to_string())
    }

    fn change_playback_volume(
        controls: MainControls,
        runtime: SharedRuntime,
        player: SharedPlayer,
        delta: f64,
    ) -> Result<String, String> {
        let volume = player.borrow_mut().change_volume(delta);
        if let Some(error) = player.borrow().last_error() {
            return Err(error.to_string());
        }
        {
            let mut borrowed = runtime.borrow_mut();
            if let Some(runtime) = borrowed.as_mut() {
                runtime.state.volume = volume;
                runtime.save_settings()?;
            }
        }
        let message = format!("Volume {}%.", (volume * 100.0).round() as i32);
        controls.frame.set_status_text(&message, 1);
        Ok(message)
    }

    fn is_enter_key(event: &WindowEventData) -> bool {
        match event {
            WindowEventData::Keyboard(keyboard) => {
                matches!(keyboard.get_key_code(), Some(13) | Some(370))
            }
            _ => false,
        }
    }

    fn update_now_playing(controls: PlaybackControls, title: &str, status: &str) {
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        controls.title.set_label(if title.is_empty() {
            "Nothing playing"
        } else {
            &title
        });
        controls.status.set_label(status);
    }

    fn report_action(controls: MainControls, result: Result<String, String>) {
        match result {
            Ok(message) => set_status(controls, &message),
            Err(error) => set_status(controls, &error),
        }
    }

    fn show_settings_dialog(
        controls: MainControls,
        runtime: SharedRuntime,
    ) -> Result<String, String> {
        let message = {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            let (lastfm_status, lastfm_action, show_finish) =
                lastfm_settings_view_state(&runtime.state);
            let spotify_status = if runtime.state.spotify_enabled {
                "Connected."
            } else {
                "Not connected."
            };
            let finish_lastfm = if show_finish {
                "\nFinish Last.fm: available after browser approval"
            } else {
                ""
            };
            format!(
                "Playlist logs\nFolder: {}\n\nLast.fm\nStatus: {}\nScrobbling: {}\nAction: {}{}\n\nSpotify\nStatus: {}\nAction: Connect Spotify",
                runtime.track_log_folder.display(),
                lastfm_status,
                if runtime.state.lastfm_scrobble_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                lastfm_action,
                finish_lastfm,
                spotify_status
            )
        };
        let dialog = MessageDialog::builder(&controls.frame, &message, "Station Scout settings")
            .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconInformation)
            .build();
        dialog.show_modal();
        Ok("Settings opened.".to_string())
    }

    fn import_opml_from_dialog(
        controls: MainControls,
        runtime: SharedRuntime,
        podcast_controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<String, String> {
        let dialog = FileDialog::builder(&controls.frame)
            .with_message("Import OPML")
            .with_wildcard("OPML files (*.opml;*.xml)|*.opml;*.xml|All files (*.*)|*.*")
            .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
            .build();
        if dialog.show_modal() != ID_OK {
            return Ok("Import cancelled.".to_string());
        }
        let path = dialog
            .get_path()
            .ok_or_else(|| "No OPML file was selected.".to_string())?;
        let payload = fs::read(&path).map_err(|error| format!("Could not read OPML: {error}"))?;
        let summary = {
            let mut borrowed = runtime.borrow_mut();
            let runtime = borrowed
                .as_mut()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .import_podcast_opml(payload.as_slice())
                .map_err(|error| error.to_string())?
        };
        refresh_podcast_lists(runtime, podcast_controls, list_state)?;
        Ok(format!(
            "Imported OPML: {} added, {} duplicates, {} rejected.",
            summary.added, summary.skipped_duplicate, summary.rejected_scheme
        ))
    }

    fn run_podcast_search(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<String, String> {
        let query = controls.query.get_value();
        let auth_date = Utc::now().timestamp().to_string();
        let outcome = {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .search_podcasts_with_client(
                    &PodcastSearchClient::default(),
                    &query,
                    &auth_date,
                    40,
                )
                .map_err(|error| error.to_string())?
        };
        let labels = outcome
            .results
            .iter()
            .map(podcast_search_result_label)
            .collect::<Vec<_>>();
        list_state.borrow_mut().search_feed_urls = outcome
            .results
            .iter()
            .map(|result| result.feed_url.clone())
            .collect();
        fill_list(controls.search_results, &labels);
        Ok(format!(
            "Found {} podcasts from {}.",
            labels.len(),
            outcome.source
        ))
    }

    fn subscribe_to_feed_from_ui(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<String, String> {
        let explicit_url = controls.feed_url.get_value().trim().to_string();
        let selected_url = selected_search_feed_url(controls, list_state.clone());
        let feed_url = if explicit_url.is_empty() {
            selected_url.ok_or_else(|| "Enter a feed URL or select a search result.".to_string())?
        } else {
            explicit_url
        };
        let next_poll_at = next_poll_time(runtime.clone())?;
        let title = {
            let mut borrowed = runtime.borrow_mut();
            let runtime = borrowed
                .as_mut()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .subscribe_to_podcast_feed(&feed_url, &next_poll_at)
                .map_err(|error| error.to_string())?
                .title
                .unwrap_or(feed_url)
        };
        refresh_podcast_lists(runtime, controls, list_state)?;
        Ok(format!("Subscribed to {title}."))
    }

    fn check_for_new_episodes(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<String, String> {
        let now = Utc::now();
        let next_poll_at = next_poll_time(runtime.clone())?;
        let summary = {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .poll_due_podcast_feeds(&now.to_rfc3339(), &next_poll_at, 50)
                .map_err(|error| error.to_string())?
        };
        refresh_podcast_lists(runtime, controls, list_state)?;
        Ok(format!(
            "Checked {} feeds; {} new episodes, {} unchanged, {} failures.",
            summary.feeds_checked, summary.new_episodes, summary.not_modified, summary.failures
        ))
    }

    fn unsubscribe_selected_show(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<String, String> {
        let subscription_id = selected_subscription_id(controls, list_state.clone())
            .ok_or_else(|| "Select a subscribed show first.".to_string())?;
        {
            let mut borrowed = runtime.borrow_mut();
            let runtime = borrowed
                .as_mut()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .unsubscribe_from_podcast(subscription_id)
                .map_err(|error| error.to_string())?;
        }
        refresh_podcast_lists(runtime, controls, list_state)?;
        Ok("Unsubscribed selected show.".to_string())
    }

    fn queue_selected_episode(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<String, String> {
        let episode_id = selected_episode_id(controls, list_state.clone())
            .ok_or_else(|| "Select an episode first.".to_string())?;
        {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .enqueue_podcast_episode(episode_id)
                .map_err(|error| error.to_string())?;
        }
        refresh_podcast_lists(runtime, controls, list_state)?;
        Ok("Queued selected episode.".to_string())
    }

    fn play_selected_podcast_episode(
        controls: PodcastControls,
        list_state: SharedPodcastListState,
        player: SharedPlayer,
        playback_controls: PlaybackControls,
    ) -> Result<String, String> {
        let episode = selected_podcast_episode(controls, list_state)
            .ok_or_else(|| "Select an episode first.".to_string())?;
        let message = play_podcast_episode_stream(
            &mut player.borrow_mut(),
            episode.title.as_deref(),
            episode.enclosure_url.as_deref(),
        )?;
        update_now_playing(
            playback_controls,
            episode.title.as_deref().unwrap_or("selected episode"),
            episode.enclosure_url.as_deref().unwrap_or(""),
        );
        Ok(message)
    }

    fn mark_selected_episode_played(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<String, String> {
        let episode_id = selected_episode_id(controls, list_state.clone())
            .ok_or_else(|| "Select an episode first.".to_string())?;
        {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .mark_podcast_episode_played(episode_id)
                .map_err(|error| error.to_string())?;
        }
        refresh_podcast_lists(runtime, controls, list_state)?;
        Ok("Marked selected episode as played.".to_string())
    }

    fn move_selected_queue_item(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
        delta: i64,
    ) -> Result<String, String> {
        let selected = controls
            .queue
            .get_selection()
            .ok_or_else(|| "Select a queue item first.".to_string())? as i64;
        let episode_id = selected_queue_episode_id(controls, list_state.clone())
            .ok_or_else(|| "Select a queue item first.".to_string())?;
        let new_position = (selected + delta).max(0);
        {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .move_queued_podcast_episode(episode_id, new_position)
                .map_err(|error| error.to_string())?;
        }
        refresh_podcast_lists(runtime, controls, list_state)?;
        controls.queue.set_selection(new_position as u32, true);
        Ok("Moved selected queue item.".to_string())
    }

    fn remove_selected_queue_item(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<String, String> {
        let episode_id = selected_queue_episode_id(controls, list_state.clone())
            .ok_or_else(|| "Select a queue item first.".to_string())?;
        {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .remove_queued_podcast_episode(episode_id)
                .map_err(|error| error.to_string())?;
        }
        refresh_podcast_lists(runtime, controls, list_state)?;
        Ok("Removed selected queue item.".to_string())
    }

    fn clear_queue(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<String, String> {
        {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .clear_podcast_queue()
                .map_err(|error| error.to_string())?;
        }
        refresh_podcast_lists(runtime, controls, list_state)?;
        Ok("Cleared podcast queue.".to_string())
    }

    fn clear_inbox(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<String, String> {
        {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            runtime
                .clear_podcast_notification_inbox()
                .map_err(|error| error.to_string())?;
        }
        refresh_podcast_lists(runtime, controls, list_state)?;
        Ok("Cleared podcast inbox.".to_string())
    }

    fn refresh_podcast_lists(
        runtime: SharedRuntime,
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Result<(), String> {
        let previous_subscription = selected_subscription_id(controls, list_state.clone());
        let (subscriptions, episodes, queue, inbox) = {
            let borrowed = runtime.borrow();
            let runtime = borrowed
                .as_ref()
                .ok_or_else(|| "Runtime unavailable.".to_string())?;
            let subscriptions = runtime
                .list_podcast_subscriptions()
                .map_err(|error| error.to_string())?;
            let selected_subscription = previous_subscription
                .filter(|id| subscriptions.iter().any(|row| row.id == *id))
                .or_else(|| subscriptions.first().map(|row| row.id));
            let episodes = match selected_subscription {
                Some(subscription_id) => runtime
                    .list_podcast_episodes(subscription_id)
                    .map_err(|error| error.to_string())?,
                None => Vec::new(),
            };
            let queue = runtime
                .list_podcast_queue()
                .map_err(|error| error.to_string())?;
            let inbox = runtime
                .list_podcast_notification_inbox(100)
                .map_err(|error| error.to_string())?;
            (subscriptions, episodes, queue, inbox)
        };
        let selected_subscription_index = previous_subscription
            .and_then(|id| subscriptions.iter().position(|row| row.id == id))
            .or_else(|| (!subscriptions.is_empty()).then_some(0));
        {
            let mut state = list_state.borrow_mut();
            state.subscription_ids = subscriptions.iter().map(|row| row.id).collect();
            state.episode_ids = episodes.iter().map(|row| row.id).collect();
            state.episode_rows = episodes.clone();
            state.queue_episode_ids = queue.iter().map(|row| row.episode_id).collect();
        }
        let model = PodcastUiModel::from_rows(subscriptions, episodes, queue, inbox);
        fill_list(controls.subscriptions, &model.subscriptions);
        fill_list(controls.episodes, &model.episodes);
        fill_list(controls.queue, &model.queue);
        fill_list(controls.inbox, &model.inbox);
        if let Some(index) = selected_subscription_index {
            controls.subscriptions.set_selection(index as u32, true);
        }
        Ok(())
    }

    fn next_poll_time(runtime: SharedRuntime) -> Result<String, String> {
        let borrowed = runtime.borrow();
        let runtime = borrowed
            .as_ref()
            .ok_or_else(|| "Runtime unavailable.".to_string())?;
        let minutes = runtime.state.podcast.polling_interval_minutes.max(5);
        Ok((Utc::now() + Duration::minutes(i64::from(minutes))).to_rfc3339())
    }

    fn selected_subscription_id(
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Option<i64> {
        selected_id(
            controls.subscriptions.get_selection(),
            &list_state.borrow().subscription_ids,
        )
    }

    fn selected_episode_id(
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Option<i64> {
        selected_id(
            controls.episodes.get_selection(),
            &list_state.borrow().episode_ids,
        )
    }

    fn selected_podcast_episode(
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Option<EpisodeRow> {
        let index = controls.episodes.get_selection()? as usize;
        list_state.borrow().episode_rows.get(index).cloned()
    }

    fn selected_queue_episode_id(
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Option<i64> {
        selected_id(
            controls.queue.get_selection(),
            &list_state.borrow().queue_episode_ids,
        )
    }

    fn selected_search_feed_url(
        controls: PodcastControls,
        list_state: SharedPodcastListState,
    ) -> Option<String> {
        let index = controls.search_results.get_selection()? as usize;
        list_state.borrow().search_feed_urls.get(index).cloned()
    }

    fn selected_id(selection: Option<u32>, ids: &[i64]) -> Option<i64> {
        ids.get(selection? as usize).copied()
    }

    fn fill_list(list: ListBox, labels: &[String]) {
        list.clear();
        for label in labels {
            list.append(label);
        }
        if !labels.is_empty() {
            list.set_selection(0, true);
        }
    }

    fn set_status(controls: MainControls, message: &str) {
        controls.status.set_label(message);
        controls.frame.set_status_text(message, 0);
    }

    fn add_menu_bar(frame: &Frame) {
        let file_menu = Menu::builder()
            .append_item(MENU_ID_SETTINGS, "&Settings...\tCtrl+,", "Settings")
            .append_separator()
            .append_item(ID_EXIT, "&Quit\tAlt+F4", "Quit")
            .build();
        let playback_menu = Menu::builder()
            .append_item(MENU_ID_FOCUS_SEARCH, "Focus search\tCtrl+F", "Focus search")
            .append_item(
                MENU_ID_PLAY_SELECTED,
                "Play selected\tCtrl+P",
                "Play selected",
            )
            .append_item(MENU_ID_TOGGLE_PAUSE, "Play/Pause\tCtrl+Space", "Play/Pause")
            .append_item(MENU_ID_STOP_PLAYBACK, "Stop\tCtrl+S", "Stop")
            .append_item(MENU_ID_VOLUME_UP, "Volume up\tCtrl+Up", "Volume up")
            .append_item(MENU_ID_VOLUME_DOWN, "Volume down\tCtrl+Down", "Volume down")
            .append_separator()
            .append_item(
                MENU_ID_PLAY_ON_COMPUTER,
                "Play on this computer",
                "Play on this computer",
            )
            .append_item(MENU_ID_CAST_SONOS, "Play on Sonos...", "Play on Sonos")
            .append_item(
                MENU_ID_CAST_CHROMECAST,
                "Cast to Chromecast...",
                "Cast to Chromecast",
            )
            .append_separator()
            .append_item(
                MENU_ID_ADD_TIMER,
                "Add tune-in timer\tCtrl+T",
                "Add tune-in timer",
            )
            .append_item(
                MENU_ID_START_TRACKING,
                "Start tracking\tCtrl+Shift+T",
                "Start tracking",
            )
            .append_item(
                MENU_ID_STOP_TRACKING,
                "Stop tracking\tCtrl+Shift+S",
                "Stop tracking",
            )
            .build();
        let menu_bar = MenuBar::builder()
            .append(file_menu, "&File")
            .append(playback_menu, "&Playback")
            .build();
        frame.set_menu_bar(menu_bar);
    }

    fn add_first_run_panel(
        parent: &Panel,
        root: &BoxSizer,
        startup: UiStartupState,
    ) -> FirstRunControls {
        let panel = Panel::builder(parent).build();
        panel.set_name("Podcast first run");
        let column = BoxSizer::builder(Orientation::Vertical).build();
        let welcome = StaticText::builder(&panel)
            .with_label("No podcast subscriptions yet. Choose how to start:")
            .build();
        describe_control(&welcome, "First run - welcome text");
        column.add(&welcome, 0, SizerFlag::All | SizerFlag::Expand, 6);
        let actions = BoxSizer::builder(Orientation::Horizontal).build();
        let subscribe = add_named_button(
            &panel,
            &actions,
            "Subscribe by URL",
            "First run - Subscribe by URL",
        );
        let import_opml =
            add_named_button(&panel, &actions, "Import OPML", "First run - Import OPML");
        let search_podcasts = add_named_button(
            &panel,
            &actions,
            "Search podcasts",
            "First run - Search podcasts",
        );
        column.add_sizer(&actions, 0, SizerFlag::Expand, 0);
        panel.set_sizer(column, true);
        panel.show(startup.show_first_run_panel);
        root.add(&panel, 0, SizerFlag::All | SizerFlag::Expand, 6);
        FirstRunControls {
            subscribe,
            import_opml,
            search_podcasts,
        }
    }

    fn add_main_workspace(
        parent: &Panel,
        root: &BoxSizer,
        startup: UiStartupState,
    ) -> (RadioControls, PodcastControls, PlaybackControls) {
        let panel = Panel::builder(parent).build();
        panel.set_name("Station Scout workspace");
        let column = BoxSizer::builder(Orientation::Vertical).build();
        add_station_picker(&panel, &column);
        let station_search = add_station_search(&panel, &column);
        let station_browse = add_station_browse(&panel, &column);
        let station_actions = add_station_actions(&panel, &column);
        let search_controls = add_podcast_search(&panel, &column);
        add_podcast_settings(&panel, &column);
        add_lastfm_settings(&panel, &column);
        let station_results = add_station_results(&panel, &column);
        let saved_lists = add_radio_saved_lists(&panel, &column);
        let notebook_controls = add_podcast_notebook(&panel, &column);
        let playback_controls = add_now_playing_controls(&panel, &column);
        panel.set_sizer(column, true);
        panel.show(startup.show_main_workspace);
        root.add(&panel, 1, SizerFlag::All | SizerFlag::Expand, 6);
        (
            RadioControls {
                name: station_search.name,
                country: station_search.country,
                language: station_search.language,
                tag: station_search.tag,
                search: station_search.search,
                browse_kind: station_browse.kind,
                load_browse: station_browse.load,
                browse_choices: station_browse.choices,
                play_selected: station_actions.play_selected,
                add_favorite: station_actions.add_favorite,
                open_website: station_actions.open_website,
                add_timer: station_actions.add_timer,
                start_tracking: station_actions.start_tracking,
                stop_tracking: station_actions.stop_tracking,
                station_results,
                favorites: saved_lists.favorites,
                recents: saved_lists.recents,
                timers: saved_lists.timers,
            },
            PodcastControls {
                query: search_controls.query,
                feed_url: search_controls.feed_url,
                run_search: search_controls.run_search,
                subscribe: notebook_controls.subscribe,
                check_now: notebook_controls.check_now,
                unsubscribe: notebook_controls.unsubscribe,
                play_episode: notebook_controls.play_episode,
                add_episode_to_queue: notebook_controls.add_episode_to_queue,
                mark_episode_played: notebook_controls.mark_episode_played,
                move_queue_up: notebook_controls.move_queue_up,
                move_queue_down: notebook_controls.move_queue_down,
                remove_queue_item: notebook_controls.remove_queue_item,
                clear_queue: notebook_controls.clear_queue,
                clear_inbox: notebook_controls.clear_inbox,
                subscriptions: notebook_controls.subscriptions,
                search_results: notebook_controls.search_results,
                episodes: notebook_controls.episodes,
                queue: notebook_controls.queue,
                inbox: notebook_controls.inbox,
            },
            playback_controls,
        )
    }

    fn add_station_picker(parent: &Panel, root: &BoxSizer) {
        let row = BoxSizer::builder(Orientation::Horizontal).build();
        add_label(parent, &row, "Station");
        let station = Choice::builder(parent)
            .with_choices(Vec::<String>::new())
            .build();
        describe_control(&station, "Station selection");
        row.add(&station, 1, SizerFlag::All | SizerFlag::Expand, 4);
        root.add_sizer(&row, 0, SizerFlag::Expand, 0);
    }

    #[derive(Clone, Copy)]
    struct StationSearchControls {
        name: TextCtrl,
        country: TextCtrl,
        language: TextCtrl,
        tag: TextCtrl,
        search: Button,
    }

    fn add_station_search(parent: &Panel, root: &BoxSizer) -> StationSearchControls {
        let row = BoxSizer::builder(Orientation::Horizontal).build();
        add_label(parent, &row, "Name");
        let name = TextCtrl::builder(parent).build();
        describe_control(&name, "Station name or call letters");
        row.add(&name, 1, SizerFlag::All | SizerFlag::Expand, 4);
        add_label(parent, &row, "Country");
        let country = TextCtrl::builder(parent).build();
        describe_control(&country, "Country");
        row.add(&country, 1, SizerFlag::All | SizerFlag::Expand, 4);
        add_label(parent, &row, "Language");
        let language = TextCtrl::builder(parent).build();
        describe_control(&language, "Language");
        row.add(&language, 1, SizerFlag::All | SizerFlag::Expand, 4);
        add_label(parent, &row, "Tag");
        let tag = TextCtrl::builder(parent).build();
        describe_control(&tag, "Tag");
        row.add(&tag, 1, SizerFlag::All | SizerFlag::Expand, 4);
        let search = add_button(parent, &row, "Search");
        root.add_sizer(&row, 0, SizerFlag::Expand, 0);
        StationSearchControls {
            name,
            country,
            language,
            tag,
            search,
        }
    }

    #[derive(Clone, Copy)]
    struct StationBrowseControls {
        kind: Choice,
        load: Button,
        choices: ListBox,
    }

    fn add_station_browse(parent: &Panel, root: &BoxSizer) -> StationBrowseControls {
        let row = BoxSizer::builder(Orientation::Horizontal).build();
        add_label(parent, &row, "Browse by");
        let kind = Choice::builder(parent)
            .with_choices(vec![
                "Genre".to_string(),
                "Location".to_string(),
                "Language".to_string(),
            ])
            .build();
        kind.set_selection(0);
        describe_control(&kind, "Browse category");
        row.add(&kind, 0, SizerFlag::All | SizerFlag::Expand, 4);
        let load = add_button(parent, &row, "Load browse choices");
        root.add_sizer(&row, 0, SizerFlag::Expand, 0);

        let column = BoxSizer::builder(Orientation::Vertical).build();
        add_label(parent, &column, "Choice");
        let choices = ListBox::builder(parent).build();
        describe_control(&choices, "Browse choices");
        column.add(&choices, 1, SizerFlag::All | SizerFlag::Expand, 4);
        root.add_sizer(&column, 1, SizerFlag::Expand, 0);
        StationBrowseControls {
            kind,
            load,
            choices,
        }
    }

    #[derive(Clone, Copy)]
    struct StationActionControls {
        play_selected: Button,
        add_favorite: Button,
        open_website: Button,
        add_timer: Button,
        start_tracking: Button,
        stop_tracking: Button,
    }

    fn add_station_actions(parent: &Panel, root: &BoxSizer) -> StationActionControls {
        let row = BoxSizer::builder(Orientation::Horizontal).build();
        let play_selected = add_button(parent, &row, "Play selected");
        play_selected.set_default();
        let add_favorite = add_button(parent, &row, "Add favorite");
        let open_website = add_button(parent, &row, "Open website");
        let add_timer = add_button(parent, &row, "Add tune-in timer");
        let start_tracking = add_button(parent, &row, "Start tracking");
        let stop_tracking = add_button(parent, &row, "Stop tracking");
        root.add_sizer(&row, 0, SizerFlag::Expand, 0);
        StationActionControls {
            play_selected,
            add_favorite,
            open_website,
            add_timer,
            start_tracking,
            stop_tracking,
        }
    }

    fn add_podcast_search(parent: &Panel, root: &BoxSizer) -> PodcastSearchControls {
        let row = BoxSizer::builder(Orientation::Horizontal).build();
        add_label(parent, &row, "Search term");
        let query = TextCtrl::builder(parent).build();
        describe_control(&query, "Podcast search query");
        row.add(&query, 1, SizerFlag::All | SizerFlag::Expand, 4);
        let run_search = add_button(parent, &row, "Run podcast search");
        add_label(parent, &row, "Feed URL");
        let feed_url = TextCtrl::builder(parent).build();
        describe_control(&feed_url, "Podcast feed URL");
        row.add(&feed_url, 1, SizerFlag::All | SizerFlag::Expand, 4);
        add_label(parent, &row, "Podcast Index API key");
        let key = TextCtrl::builder(parent).build();
        describe_control(&key, "Podcast Index API key");
        row.add(&key, 1, SizerFlag::All | SizerFlag::Expand, 4);
        add_label(parent, &row, "Podcast Index API secret");
        let secret = TextCtrl::builder(parent).build();
        describe_control(&secret, "Podcast Index API secret");
        row.add(&secret, 1, SizerFlag::All | SizerFlag::Expand, 4);
        root.add_sizer(&row, 0, SizerFlag::Expand, 0);
        PodcastSearchControls {
            query,
            feed_url,
            run_search,
        }
    }

    fn add_podcast_settings(parent: &Panel, root: &BoxSizer) {
        let timing_row = BoxSizer::builder(Orientation::Horizontal).build();

        add_label(parent, &timing_row, "Polling interval (minutes)");
        let interval = SpinCtrl::builder(parent).with_range(5, 240).build();
        describe_control(&interval, "Polling interval in minutes");
        interval.set_value(30);
        timing_row.add(&interval, 0, SizerFlag::All | SizerFlag::Expand, 4);

        add_label(parent, &timing_row, "Quiet hours");
        let quiet_hours = CheckBox::builder(parent).build();
        describe_control(&quiet_hours, "Enable quiet hours");
        quiet_hours.set_value(false);
        timing_row.add(
            &quiet_hours,
            0,
            SizerFlag::All | SizerFlag::AlignCenterVertical,
            4,
        );

        add_label(parent, &timing_row, "Quiet hours start (HH:MM)");
        let quiet_start = TextCtrl::builder(parent).build();
        describe_control(&quiet_start, "Quiet hours start time");
        quiet_start.set_value("22:00");
        timing_row.add(&quiet_start, 0, SizerFlag::All | SizerFlag::Expand, 4);

        add_label(parent, &timing_row, "Quiet hours end (HH:MM)");
        let quiet_end = TextCtrl::builder(parent).build();
        describe_control(&quiet_end, "Quiet hours end time");
        quiet_end.set_value("07:00");
        timing_row.add(&quiet_end, 0, SizerFlag::All | SizerFlag::Expand, 4);

        root.add_sizer(&timing_row, 0, SizerFlag::Expand, 0);

        let playback_row = BoxSizer::builder(Orientation::Horizontal).build();

        add_label(parent, &playback_row, "Startup");
        let start_with_windows = CheckBox::builder(parent).build();
        describe_control(&start_with_windows, "Start with Windows");
        start_with_windows.set_value(false);
        playback_row.add(
            &start_with_windows,
            0,
            SizerFlag::All | SizerFlag::AlignCenterVertical,
            4,
        );

        add_label(parent, &playback_row, "Default playback speed");
        let default_speed = TextCtrl::builder(parent).build();
        describe_control(&default_speed, "Default playback speed");
        default_speed.set_value("1.00");
        playback_row.add(&default_speed, 0, SizerFlag::All | SizerFlag::Expand, 4);

        add_label(parent, &playback_row, "Default sleep timer (minutes)");
        let sleep_default = SpinCtrl::builder(parent).with_range(1, 240).build();
        describe_control(&sleep_default, "Default sleep timer minutes");
        sleep_default.set_value(15);
        playback_row.add(&sleep_default, 0, SizerFlag::All | SizerFlag::Expand, 4);

        add_label(parent, &playback_row, "OpenRouter model");
        let openrouter_model = TextCtrl::builder(parent).build();
        describe_control(&openrouter_model, "OpenRouter model");
        playback_row.add(&openrouter_model, 1, SizerFlag::All | SizerFlag::Expand, 4);

        root.add_sizer(&playback_row, 0, SizerFlag::Expand, 0);
    }

    fn add_lastfm_settings(parent: &Panel, root: &BoxSizer) {
        let row = BoxSizer::builder(Orientation::Horizontal).build();

        add_label(parent, &row, "Last.fm status");
        let status = StaticText::builder(parent)
            .with_label("Not connected.")
            .build();
        describe_control(&status, "Last.fm connection status");
        row.add(&status, 1, SizerFlag::All | SizerFlag::Expand, 4);

        add_label(parent, &row, "Scrobbling");
        let scrobbling = CheckBox::builder(parent).build();
        describe_control(&scrobbling, "Scrobble played songs to Last.fm");
        scrobbling.set_value(true);
        row.add(
            &scrobbling,
            0,
            SizerFlag::All | SizerFlag::AlignCenterVertical,
            4,
        );

        root.add_sizer(&row, 0, SizerFlag::Expand, 0);
    }

    fn add_station_results(parent: &Panel, root: &BoxSizer) -> ListCtrl {
        let station_col = BoxSizer::builder(Orientation::Vertical).build();
        add_label(parent, &station_col, "Station results");
        let station_results = ListCtrl::builder(parent)
            .with_style(ListCtrlStyle::Report | ListCtrlStyle::SingleSel)
            .build();
        station_results.insert_column(0, "Station", ListColumnFormat::Left, 220);
        station_results.insert_column(1, "Location", ListColumnFormat::Left, 160);
        station_results.insert_column(2, "Quality", ListColumnFormat::Left, 220);
        describe_control(&station_results, "Station search results");
        station_col.add(&station_results, 1, SizerFlag::All | SizerFlag::Expand, 4);
        root.add_sizer(&station_col, 1, SizerFlag::Expand, 0);
        station_results
    }

    #[derive(Clone, Copy)]
    struct RadioSavedLists {
        favorites: ListBox,
        recents: ListBox,
        timers: ListBox,
    }

    fn add_radio_saved_lists(parent: &Panel, root: &BoxSizer) -> RadioSavedLists {
        let row = BoxSizer::builder(Orientation::Horizontal).build();

        let favorites_col = BoxSizer::builder(Orientation::Vertical).build();
        add_label(parent, &favorites_col, "Favorite stations");
        let favorites = ListBox::builder(parent).build();
        describe_control(&favorites, "Favorite stations");
        favorites_col.add(&favorites, 1, SizerFlag::All | SizerFlag::Expand, 4);
        row.add_sizer(&favorites_col, 1, SizerFlag::Expand, 0);

        let recents_col = BoxSizer::builder(Orientation::Vertical).build();
        add_label(parent, &recents_col, "Recently played stations");
        let recents = ListBox::builder(parent).build();
        describe_control(&recents, "Recently played stations");
        recents_col.add(&recents, 1, SizerFlag::All | SizerFlag::Expand, 4);
        row.add_sizer(&recents_col, 1, SizerFlag::Expand, 0);

        let timers_col = BoxSizer::builder(Orientation::Vertical).build();
        add_label(parent, &timers_col, "Timers");
        let timers = ListBox::builder(parent).build();
        describe_control(&timers, "Tune-in timers");
        timers_col.add(&timers, 1, SizerFlag::All | SizerFlag::Expand, 4);
        row.add_sizer(&timers_col, 1, SizerFlag::Expand, 0);

        root.add_sizer(&row, 1, SizerFlag::Expand, 0);
        RadioSavedLists {
            favorites,
            recents,
            timers,
        }
    }

    fn add_podcast_notebook(parent: &Panel, root: &BoxSizer) -> PodcastNotebookControls {
        let notebook = Notebook::builder(parent).build();
        describe_control(&notebook, "Podcast navigation tabs");

        let subscriptions = Panel::builder(&notebook).build();
        let subscription_controls = add_subscriptions_page(&subscriptions);
        notebook.add_page(&subscriptions, "Subscriptions", true, None);

        let episodes = Panel::builder(&notebook).build();
        let episode_controls = add_episodes_page(&episodes);
        notebook.add_page(&episodes, "Episodes", false, None);

        let queue = Panel::builder(&notebook).build();
        let queue_controls = add_queue_page(&queue);
        notebook.add_page(&queue, "Queue", false, None);

        let inbox = Panel::builder(&notebook).build();
        let inbox_controls = add_inbox_page(&inbox);
        notebook.add_page(&inbox, "Inbox", false, None);

        root.add(&notebook, 2, SizerFlag::All | SizerFlag::Expand, 4);
        PodcastNotebookControls {
            subscribe: subscription_controls.subscribe,
            check_now: subscription_controls.check_now,
            unsubscribe: subscription_controls.unsubscribe,
            play_episode: episode_controls.play_episode,
            add_episode_to_queue: episode_controls.add_episode_to_queue,
            mark_episode_played: episode_controls.mark_episode_played,
            move_queue_up: queue_controls.move_queue_up,
            move_queue_down: queue_controls.move_queue_down,
            remove_queue_item: queue_controls.remove_queue_item,
            clear_queue: queue_controls.clear_queue,
            clear_inbox: inbox_controls.clear_inbox,
            subscriptions: subscription_controls.subscriptions,
            search_results: subscription_controls.search_results,
            episodes: episode_controls.episodes,
            queue: queue_controls.queue,
            inbox: inbox_controls.inbox,
        }
    }

    #[derive(Clone, Copy)]
    struct SubscriptionPageControls {
        subscribe: Button,
        check_now: Button,
        unsubscribe: Button,
        subscriptions: ListBox,
        search_results: ListBox,
    }

    fn add_subscriptions_page(parent: &Panel) -> SubscriptionPageControls {
        let column = BoxSizer::builder(Orientation::Vertical).build();
        add_label(parent, &column, "Subscriptions");
        let subscriptions = ListBox::builder(parent).build();
        describe_control(&subscriptions, "Subscribed shows");
        column.add(&subscriptions, 1, SizerFlag::All | SizerFlag::Expand, 4);

        let subscription_actions = BoxSizer::builder(Orientation::Horizontal).build();
        let subscribe = add_button(parent, &subscription_actions, "Subscribe by URL");
        let check_now = add_button(parent, &subscription_actions, "Check for new episodes now");
        let unsubscribe = add_button(parent, &subscription_actions, "Unsubscribe selected show");
        column.add_sizer(&subscription_actions, 0, SizerFlag::Expand, 0);

        add_label(parent, &column, "Results");
        let podcast_results = ListBox::builder(parent).build();
        describe_control(&podcast_results, "Podcast search results");
        column.add(&podcast_results, 1, SizerFlag::All | SizerFlag::Expand, 4);

        add_per_show_settings_controls(parent, &column);
        parent.set_sizer(column, true);
        SubscriptionPageControls {
            subscribe,
            check_now,
            unsubscribe,
            subscriptions,
            search_results: podcast_results,
        }
    }

    fn add_per_show_settings_controls(parent: &Panel, root: &BoxSizer) {
        let controls = station_scout_ui_blueprint();
        let row = BoxSizer::builder(Orientation::Horizontal).build();

        add_label(parent, &row, "Notifications");
        let rule = Choice::builder(parent)
            .with_choices(
                controls
                    .per_show_notification_choices
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>(),
            )
            .build();
        describe_control(&rule, "Per-show notification rule");
        rule.set_selection(0);
        row.add(&rule, 0, SizerFlag::All | SizerFlag::Expand, 4);

        add_label(parent, &row, "Skip silence");
        let skip_silence = CheckBox::builder(parent).build();
        describe_control(&skip_silence, "Skip silence for this show");
        skip_silence.set_value(false);
        row.add(
            &skip_silence,
            0,
            SizerFlag::All | SizerFlag::AlignCenterVertical,
            4,
        );

        add_label(parent, &row, "Skip intro (seconds)");
        let intro = SpinCtrl::builder(parent).with_range(0, 600).build();
        describe_control(&intro, "Intro skip seconds");
        intro.set_value(0);
        row.add(&intro, 0, SizerFlag::All | SizerFlag::Expand, 4);

        add_label(parent, &row, "Skip outro (seconds)");
        let outro = SpinCtrl::builder(parent).with_range(0, 600).build();
        describe_control(&outro, "Outro skip seconds");
        outro.set_value(0);
        row.add(&outro, 0, SizerFlag::All | SizerFlag::Expand, 4);

        root.add_sizer(&row, 0, SizerFlag::Expand, 0);
    }

    #[derive(Clone, Copy)]
    struct EpisodePageControls {
        play_episode: Button,
        add_episode_to_queue: Button,
        mark_episode_played: Button,
        episodes: ListBox,
    }

    fn add_episodes_page(parent: &Panel) -> EpisodePageControls {
        let column = BoxSizer::builder(Orientation::Vertical).build();
        add_label(parent, &column, "Episodes");
        let episodes = ListBox::builder(parent).build();
        describe_control(&episodes, "Episode list for selected show");
        column.add(&episodes, 1, SizerFlag::All | SizerFlag::Expand, 4);

        add_label(parent, &column, "Show notes");
        let show_notes = TextCtrl::builder(parent)
            .with_style(
                TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly | TextCtrlStyle::WordWrap,
            )
            .build();
        describe_control(&show_notes, "Show notes for selected episode");
        column.add(&show_notes, 1, SizerFlag::All | SizerFlag::Expand, 4);

        add_label(parent, &column, "Transcript");
        let transcript = TextCtrl::builder(parent)
            .with_style(
                TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly | TextCtrlStyle::WordWrap,
            )
            .build();
        describe_control(&transcript, "Transcript for selected episode");
        column.add(&transcript, 1, SizerFlag::All | SizerFlag::Expand, 4);

        let row = BoxSizer::builder(Orientation::Horizontal).build();
        let play_episode = add_button(parent, &row, "Play selected episode");
        let add_episode_to_queue = add_button(parent, &row, "Add selected episode to queue");
        let mark_episode_played = add_button(parent, &row, "Mark selected episode as played");
        column.add_sizer(&row, 0, SizerFlag::Expand, 0);
        parent.set_sizer(column, true);
        EpisodePageControls {
            play_episode,
            add_episode_to_queue,
            mark_episode_played,
            episodes,
        }
    }

    #[derive(Clone, Copy)]
    struct QueuePageControls {
        move_queue_up: Button,
        move_queue_down: Button,
        remove_queue_item: Button,
        clear_queue: Button,
        queue: ListBox,
    }

    fn add_queue_page(parent: &Panel) -> QueuePageControls {
        let column = BoxSizer::builder(Orientation::Vertical).build();
        add_label(parent, &column, "Queue (up next)");
        let queue = ListBox::builder(parent).build();
        describe_control(&queue, "Playback queue");
        column.add(&queue, 1, SizerFlag::All | SizerFlag::Expand, 4);

        let row = BoxSizer::builder(Orientation::Horizontal).build();
        let move_queue_up = add_button(parent, &row, "Move selected queue item up");
        let move_queue_down = add_button(parent, &row, "Move selected queue item down");
        let remove_queue_item = add_button(parent, &row, "Remove selected queue item");
        let clear_queue = add_button(parent, &row, "Clear queue");
        column.add_sizer(&row, 0, SizerFlag::Expand, 0);
        parent.set_sizer(column, true);
        QueuePageControls {
            move_queue_up,
            move_queue_down,
            remove_queue_item,
            clear_queue,
            queue,
        }
    }

    #[derive(Clone, Copy)]
    struct InboxPageControls {
        clear_inbox: Button,
        inbox: ListBox,
    }

    fn add_inbox_page(parent: &Panel) -> InboxPageControls {
        let column = BoxSizer::builder(Orientation::Vertical).build();
        add_label(parent, &column, "Inbox");
        let inbox = ListBox::builder(parent).build();
        describe_control(&inbox, "Notification inbox");
        column.add(&inbox, 1, SizerFlag::All | SizerFlag::Expand, 4);

        let row = BoxSizer::builder(Orientation::Horizontal).build();
        let clear_inbox = add_button(parent, &row, "Clear inbox");
        column.add_sizer(&row, 0, SizerFlag::Expand, 0);
        parent.set_sizer(column, true);
        InboxPageControls { clear_inbox, inbox }
    }

    fn add_now_playing_controls(parent: &Panel, root: &BoxSizer) -> PlaybackControls {
        let body = BoxSizer::builder(Orientation::Horizontal).build();
        let status_col = BoxSizer::builder(Orientation::Vertical).build();
        add_label(parent, &status_col, "Now Playing");
        let title = StaticText::builder(parent)
            .with_label("Nothing playing")
            .build();
        describe_control(&title, "Now-playing episode title");
        status_col.add(&title, 0, SizerFlag::All | SizerFlag::Expand, 4);
        let status = StaticText::builder(parent).with_label("").build();
        describe_control(&status, "Now-playing status");
        status_col.add(&status, 0, SizerFlag::All | SizerFlag::Expand, 4);
        let now_playing = TextCtrl::builder(parent)
            .with_style(
                TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly | TextCtrlStyle::WordWrap,
            )
            .build();
        describe_control(&now_playing, "Now playing");
        status_col.add(&now_playing, 1, SizerFlag::All | SizerFlag::Expand, 4);
        body.add_sizer(&status_col, 1, SizerFlag::Expand, 0);

        let controls = station_scout_ui_blueprint();
        let action_col = BoxSizer::builder(Orientation::Vertical).build();
        let transport_row = BoxSizer::builder(Orientation::Horizontal).build();
        let stop = add_button(parent, &transport_row, "Stop current playback");
        action_col.add_sizer(&transport_row, 0, SizerFlag::Expand, 0);

        let row = BoxSizer::builder(Orientation::Horizontal).build();

        add_label(parent, &row, "Speed");
        let speed = Choice::builder(parent)
            .with_choices(
                controls
                    .playback_speed_choices
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>(),
            )
            .build();
        describe_control(&speed, "Playback speed");
        speed.set_selection(1);
        row.add(&speed, 0, SizerFlag::All | SizerFlag::Expand, 4);

        add_label(parent, &row, "Sleep timer");
        let sleep = Choice::builder(parent)
            .with_choices(
                controls
                    .sleep_timer_choices
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>(),
            )
            .build();
        describe_control(&sleep, "Sleep timer");
        sleep.set_selection(0);
        row.add(&sleep, 0, SizerFlag::All | SizerFlag::Expand, 4);

        action_col.add_sizer(&row, 0, SizerFlag::Expand, 0);
        body.add_sizer(&action_col, 2, SizerFlag::Expand, 0);
        root.add_sizer(&body, 0, SizerFlag::Expand, 0);
        PlaybackControls {
            title,
            status,
            stop,
        }
    }

    fn add_button(parent: &Panel, row: &BoxSizer, label: &str) -> Button {
        add_named_button(parent, row, label, label)
    }

    fn add_named_button(parent: &Panel, row: &BoxSizer, label: &str, name: &str) -> Button {
        let button = Button::builder(parent).with_label(label).build();
        describe_control(&button, name);
        row.add(&button, 0, SizerFlag::All, 4);
        button
    }

    fn add_label(parent: &Panel, row: &BoxSizer, label: &str) {
        let text = StaticText::builder(parent).with_label(label).build();
        text.set_name(label);
        row.add(&text, 0, SizerFlag::All, 4);
    }

    fn describe_control(control: &impl WxWidget, description: &str) {
        control.set_name(description);
        control.set_tooltip(description);
    }
}
