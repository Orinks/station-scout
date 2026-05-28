use station_scout::ui::{
    APP_TITLE, UiStartupState, playback_window_title, startup_state_for_podcast_subscriptions,
    station_scout_ui_blueprint,
};

#[test]
fn playback_window_title_defaults_to_app_title_and_prefixes_now_playing() {
    assert_eq!(playback_window_title(""), APP_TITLE);
    assert_eq!(
        playback_window_title("Transmission FM\nGina's Organized Chaos"),
        "Transmission FM Gina's Organized Chaos - Station Scout"
    );
}

#[test]
fn ui_blueprint_preserves_station_scout_surfaces_and_accessible_initial_focus() {
    let blueprint = station_scout_ui_blueprint();

    assert_eq!(blueprint.title, APP_TITLE);
    assert_eq!(blueprint.initial_focus, "Favorite stations");
    assert!(blueprint.sections.contains(&"Search Radio Browser"));
    assert!(blueprint.sections.contains(&"Browse Stations"));
    assert!(blueprint.sections.contains(&"Tune-in Timers"));
    assert!(blueprint.primary_actions.contains(&"Play selected"));
    assert!(!blueprint.primary_actions.contains(&"Play selected station"));
    assert!(blueprint.list_names.contains(&"Station search results"));
}

#[test]
fn ui_blueprint_matches_reference_python_menubar() {
    let blueprint = station_scout_ui_blueprint();

    assert_eq!(blueprint.menu_titles, &["File", "Playback"]);
    assert_eq!(
        blueprint.menu_items,
        &[
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
        ]
    );
}

#[test]
fn ui_blueprint_includes_podcatcher_surfaces_from_reference_app() {
    let blueprint = station_scout_ui_blueprint();

    assert!(blueprint.sections.contains(&"Podcast Subscriptions"));
    assert!(blueprint.sections.contains(&"Episode List"));
    assert!(blueprint.sections.contains(&"Now Playing"));
    assert!(blueprint.sections.contains(&"Queue / Up Next"));
    assert!(blueprint.sections.contains(&"Podcast Inbox"));
    assert!(blueprint.primary_actions.contains(&"Subscribe by feed URL"));
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Feed URL", "Podcast feed URL"))
    );
    assert!(blueprint.primary_actions.contains(&"Run podcast search"));
    assert!(blueprint.primary_actions.contains(&"Import OPML"));
    assert!(blueprint.primary_actions.contains(&"Export OPML"));
    assert!(blueprint.primary_actions.contains(&"Subscribe by URL"));
    assert!(
        blueprint
            .primary_actions
            .contains(&"Check for new episodes now")
    );
    assert!(
        blueprint
            .primary_actions
            .contains(&"Per-show settings for selected show")
    );
    assert!(
        blueprint
            .primary_actions
            .contains(&"Unsubscribe selected show")
    );
    assert!(blueprint.primary_actions.contains(&"Per-show settings"));
    assert!(blueprint.primary_actions.contains(&"Play selected episode"));
    assert!(
        blueprint
            .primary_actions
            .contains(&"Add selected episode to queue")
    );
    assert!(
        blueprint
            .primary_actions
            .contains(&"Mark selected episode as played")
    );
    assert!(
        blueprint
            .primary_actions
            .contains(&"Move selected queue item up")
    );
    assert!(
        blueprint
            .primary_actions
            .contains(&"Move selected queue item down")
    );
    assert!(
        blueprint
            .primary_actions
            .contains(&"Remove selected queue item")
    );
    assert!(blueprint.primary_actions.contains(&"Clear queue"));
    assert!(
        blueprint
            .primary_actions
            .contains(&"Open selected inbox entry")
    );
    assert!(blueprint.primary_actions.contains(&"Clear inbox"));
    assert!(blueprint.list_names.contains(&"Subscribed shows"));
    assert!(
        blueprint
            .list_names
            .contains(&"Episode list for selected show")
    );
    assert!(blueprint.list_names.contains(&"Playback queue"));
    assert!(blueprint.list_names.contains(&"Notification inbox"));
    assert!(blueprint.list_names.contains(&"Podcast search results"));
    assert!(blueprint.sections.contains(&"Podcast Search"));
    assert!(blueprint.sections.contains(&"Podcast Settings"));
}

#[test]
fn ui_blueprint_includes_podcatcher_now_playing_controls() {
    let blueprint = station_scout_ui_blueprint();

    assert_eq!(
        blueprint.notebook_pages,
        &["Subscriptions", "Episodes", "Queue", "Inbox"]
    );
    assert_eq!(blueprint.now_playing_controls, &["Stop current playback",]);
    assert_eq!(
        blueprint.playback_shortcuts,
        &[
            "Ctrl+F",
            "Ctrl+P",
            "Ctrl+Space",
            "Ctrl+S",
            "Ctrl+Up",
            "Ctrl+Down",
            "Ctrl+T",
            "Ctrl+Shift+T",
            "Ctrl+Shift+S",
        ]
    );
    assert_eq!(
        blueprint.now_playing_status_fields,
        &["Now-playing episode title", "Now-playing status"]
    );
    assert_eq!(
        blueprint.playback_speed_choices,
        &["0.8x", "1.0x", "1.25x", "1.5x", "1.75x", "2.0x"]
    );
    assert_eq!(
        blueprint.sleep_timer_choices,
        &[
            "Off",
            "5 min",
            "10 min",
            "15 min",
            "30 min",
            "45 min",
            "60 min",
            "End of episode",
        ]
    );
}

#[test]
fn ui_blueprint_marks_station_lists_as_enter_activatable() {
    let blueprint = station_scout_ui_blueprint();

    assert_eq!(
        blueprint.station_activation_lists,
        &[
            "Station search results",
            "Favorite stations",
            "Recently played stations",
        ]
    );
}

#[test]
fn ui_blueprint_includes_first_run_control_order_from_reference_app() {
    let blueprint = station_scout_ui_blueprint();

    assert_eq!(
        blueprint.first_run_controls,
        &[
            "First run - Subscribe by URL",
            "First run - Import OPML",
            "First run - Search podcasts",
        ]
    );
}

#[test]
fn ui_blueprint_includes_per_show_settings_controls() {
    let blueprint = station_scout_ui_blueprint();

    assert!(blueprint.sections.contains(&"Per-show Settings"));
    assert_eq!(
        blueprint.per_show_notification_choices,
        &["Always", "Digest", "Never"]
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Notifications", "Per-show notification rule"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Skip silence", "Skip silence for this show"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Skip intro (seconds)", "Intro skip seconds"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Skip outro (seconds)", "Outro skip seconds"))
    );
}

#[test]
fn ui_blueprint_includes_global_podcast_settings_controls() {
    let blueprint = station_scout_ui_blueprint();

    assert!(blueprint.sections.contains(&"Podcast Settings"));
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Polling interval (minutes)", "Polling interval in minutes"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Quiet hours", "Enable quiet hours"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Quiet hours start (HH:MM)", "Quiet hours start time"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Quiet hours end (HH:MM)", "Quiet hours end time"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Startup", "Start with Windows"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Default playback speed", "Default playback speed"))
    );
    assert!(blueprint.label_order_pairs.contains(&(
        "Default sleep timer (minutes)",
        "Default sleep timer minutes"
    )));
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("OpenRouter model", "OpenRouter model"))
    );
}

#[test]
fn ui_blueprint_includes_lastfm_settings_controls() {
    let blueprint = station_scout_ui_blueprint();

    assert!(blueprint.sections.contains(&"Last.fm"));
    assert!(blueprint.primary_actions.contains(&"Connect Last.fm"));
    assert!(blueprint.primary_actions.contains(&"Finish Last.fm"));
    assert!(blueprint.primary_actions.contains(&"Connect Spotify"));
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Last.fm status", "Last.fm connection status"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Scrobbling", "Scrobble played songs to Last.fm"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Spotify status", "Spotify connection status"))
    );
}

#[test]
fn ui_blueprint_keeps_label_before_control_pairs_for_wx_screen_readers() {
    let blueprint = station_scout_ui_blueprint();

    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Station", "Station selection"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Name", "Station name or call letters"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Subscriptions", "Subscribed shows"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Episodes", "Episode list for selected show"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Show notes", "Show notes for selected episode"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Transcript", "Transcript for selected episode"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Search term", "Podcast search query"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Results", "Podcast search results"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Queue (up next)", "Playback queue"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Inbox", "Notification inbox"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Podcast Index API key", "Podcast Index API key"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Podcast Index API secret", "Podcast Index API secret"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Speed", "Playback speed"))
    );
    assert!(
        blueprint
            .label_order_pairs
            .contains(&("Sleep timer", "Sleep timer"))
    );
}

#[test]
fn ui_startup_state_matches_podcatcher_first_run_behavior() {
    assert_eq!(
        startup_state_for_podcast_subscriptions(false),
        UiStartupState {
            show_first_run_panel: true,
            show_main_workspace: true,
            initial_focus: "Favorite stations",
            status: "Ready. No podcast subscriptions yet.",
        }
    );
    assert_eq!(
        startup_state_for_podcast_subscriptions(true),
        UiStartupState {
            show_first_run_panel: false,
            show_main_workspace: true,
            initial_focus: "Podcast subscriptions",
            status: "Ready.",
        }
    );
}
