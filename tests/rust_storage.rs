use station_scout::models::{Station, TuneTimer};
use station_scout::notifications::NotificationRule;
use station_scout::podcasts::feed::{Episode, ParsedFeed};
use station_scout::podcasts::repository::{
    SubscriptionSettings, clear_notification_inbox, clear_queue, enqueue_episode, get_cache_entry,
    get_feed_urls, get_played_state, get_subscription_settings, lease_cache_entry,
    list_active_subscriptions, list_due_subscriptions, list_episodes, list_notification_inbox,
    list_queue, log_notification_event, mark_cache_was_playing, mark_episode_played,
    mark_notification_toast_fired, mark_poll_failure, mark_poll_success, move_queued_episode,
    open_memory_database, release_cache_entry, remove_queued_episode, set_playback_position,
    subscribe, unsubscribe, update_cache_entry, update_subscription_settings, upsert_episodes,
};
use station_scout::storage::{
    AppState, CredentialStore, KeyringCredentialStore, MemoryCredentialStore, PodcastSettings,
    SettingsStore, add_unique_station, station_scout_settings_path,
};
use std::collections::HashMap;
use std::path::Path;

#[test]
fn settings_store_round_trips_non_secret_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("station_scout.json");
    let store = SettingsStore::new(path.clone());
    let mut state = AppState {
        favorites: vec![Station::new("abc", "Example", "https://stream.test")],
        recents: vec![Station::new("def", "Recent", "https://recent.test")],
        timers: vec![TuneTimer::new("abc", "Example", "07:30")],
        volume: 1.4,
        track_log_folder: "C:/Shows".to_string(),
        lastfm_enabled: true,
        lastfm_scrobble_enabled: false,
        lastfm_username: "listener".to_string(),
        spotify_enabled: true,
        spotify_token_expires_at: 123,
        spotify_auth_state: "state".to_string(),
        ..AppState::default()
    };
    state.spotify_access_token = "secret-access".to_string();
    state.spotify_refresh_token = "secret-refresh".to_string();

    store.save(&state).expect("state should save");
    let loaded = store.load().expect("state should load");
    let saved_text = std::fs::read_to_string(path).expect("settings file should exist");

    assert_eq!(loaded.favorites[0].name, "Example");
    assert_eq!(loaded.recents[0].name, "Recent");
    assert_eq!(loaded.timers[0].time, "07:30");
    assert_eq!(loaded.volume, 1.0);
    assert_eq!(loaded.track_log_folder, "C:/Shows");
    assert!(loaded.lastfm_enabled);
    assert!(loaded.spotify_enabled);
    assert_eq!(loaded.spotify_token_expires_at, 123);
    assert_eq!(loaded.spotify_access_token, "");
    assert!(!saved_text.contains("secret-access"));
    assert!(!saved_text.contains("secret-refresh"));
}

#[test]
fn settings_store_round_trips_secrets_through_credential_store() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("station_scout.json");
    let credentials = MemoryCredentialStore::default();
    let store = SettingsStore::new(path.clone()).with_credentials(Box::new(credentials.clone()));
    let state = AppState {
        lastfm_session_key: "session".to_string(),
        lastfm_pending_token: "pending-token".to_string(),
        spotify_access_token: "access".to_string(),
        spotify_refresh_token: "refresh".to_string(),
        spotify_code_verifier: "verifier".to_string(),
        podcast_index_api_key: "podcast-index-key".to_string(),
        podcast_index_api_secret: "podcast-index-secret".to_string(),
        openrouter_api_key: "openrouter-key".to_string(),
        ..AppState::default()
    };

    store.save(&state).expect("state should save");
    let loaded = store.load().expect("state should load");
    let saved_text = std::fs::read_to_string(path).expect("settings file should exist");

    assert_eq!(loaded.lastfm_session_key, "session");
    assert_eq!(loaded.lastfm_pending_token, "pending-token");
    assert_eq!(loaded.spotify_access_token, "access");
    assert_eq!(loaded.spotify_refresh_token, "refresh");
    assert_eq!(loaded.spotify_code_verifier, "verifier");
    assert_eq!(loaded.podcast_index_api_key, "podcast-index-key");
    assert_eq!(loaded.podcast_index_api_secret, "podcast-index-secret");
    assert_eq!(loaded.openrouter_api_key, "openrouter-key");
    assert_eq!(credentials.get("spotify_refresh_token"), "refresh");
    assert_eq!(credentials.get("podcast_index_key"), "podcast-index-key");
    assert_eq!(
        credentials.get("podcast_index_secret"),
        "podcast-index-secret"
    );
    assert_eq!(credentials.get("openrouter"), "openrouter-key");
    assert!(!saved_text.contains("session"));
    assert!(!saved_text.contains("access"));
    assert!(!saved_text.contains("refresh"));
    assert!(!saved_text.contains("verifier"));
    assert!(!saved_text.contains("podcast-index-key"));
    assert!(!saved_text.contains("podcast-index-secret"));
    assert!(!saved_text.contains("openrouter-key"));
}

#[test]
fn keyring_credential_store_round_trips_and_deletes_secrets() {
    keyring::use_sample_store(&HashMap::new()).expect("sample keyring should configure");
    let store = KeyringCredentialStore::new("Station Scout Test");
    let key = format!("secret-{}", std::process::id());

    store.set(&key, "saved-secret");
    assert_eq!(store.get(&key), "saved-secret");

    store.set(&key, "");
    assert_eq!(store.get(&key), "");
}

#[test]
fn settings_store_migrates_plaintext_secrets_to_credential_store() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("station_scout.json");
    std::fs::write(
        &path,
        r#"{
            "lastfm_enabled": true,
            "lastfm_session_key": "old-session",
            "lastfm_pending_token": "old-token",
            "spotify_enabled": true,
            "spotify_access_token": "old-access",
            "spotify_refresh_token": "old-refresh",
            "spotify_code_verifier": "old-verifier",
            "podcast_index_key": "old-pi-key",
            "podcast_index_secret": "old-pi-secret",
            "openrouter": "old-openrouter-key"
        }"#,
    )
    .unwrap();
    let credentials = MemoryCredentialStore::default();
    let store = SettingsStore::new(path.clone()).with_credentials(Box::new(credentials.clone()));

    let loaded = store.load().expect("state should load");
    let settings_text = std::fs::read_to_string(path).unwrap();

    assert_eq!(loaded.lastfm_session_key, "old-session");
    assert_eq!(loaded.lastfm_pending_token, "old-token");
    assert_eq!(loaded.spotify_access_token, "old-access");
    assert_eq!(loaded.spotify_refresh_token, "old-refresh");
    assert_eq!(loaded.spotify_code_verifier, "old-verifier");
    assert_eq!(loaded.podcast_index_api_key, "old-pi-key");
    assert_eq!(loaded.podcast_index_api_secret, "old-pi-secret");
    assert_eq!(loaded.openrouter_api_key, "old-openrouter-key");
    assert_eq!(credentials.get("spotify_code_verifier"), "old-verifier");
    assert_eq!(credentials.get("podcast_index_key"), "old-pi-key");
    assert_eq!(credentials.get("podcast_index_secret"), "old-pi-secret");
    assert_eq!(credentials.get("openrouter"), "old-openrouter-key");
    assert!(!settings_text.contains("old-session"));
    assert!(!settings_text.contains("old-access"));
    assert!(!settings_text.contains("old-refresh"));
    assert!(!settings_text.contains("old-verifier"));
    assert!(!settings_text.contains("old-pi-key"));
    assert!(!settings_text.contains("old-pi-secret"));
    assert!(!settings_text.contains("old-openrouter-key"));
}

#[test]
fn settings_store_round_trips_podcast_preferences() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("station_scout.json");
    let store = SettingsStore::new(path.clone());
    let state = AppState {
        podcast: PodcastSettings {
            polling_interval_minutes: 60,
            quiet_hours: station_scout::notifications::QuietHours {
                enabled: true,
                start: "21:30".to_string(),
                end: "06:15".to_string(),
            },
            start_with_windows: true,
            playback_speed: 1.5,
            sleep_timer_default_minutes: 45,
            openrouter_model: "anthropic/claude-haiku-4.5".to_string(),
            first_run_completed: true,
            schema_version: 1,
        },
        ..AppState::default()
    };

    store.save(&state).expect("state should save");
    let loaded = store.load().expect("state should load");
    let saved_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

    assert_eq!(loaded.podcast.polling_interval_minutes, 60);
    assert!(loaded.podcast.quiet_hours.enabled);
    assert_eq!(loaded.podcast.quiet_hours.start, "21:30");
    assert_eq!(loaded.podcast.quiet_hours.end, "06:15");
    assert!(loaded.podcast.start_with_windows);
    assert_eq!(loaded.podcast.playback_speed, 1.5);
    assert_eq!(loaded.podcast.sleep_timer_default_minutes, 45);
    assert_eq!(
        loaded.podcast.openrouter_model,
        "anthropic/claude-haiku-4.5"
    );
    assert!(loaded.podcast.first_run_completed);
    assert_eq!(loaded.podcast.schema_version, 1);
    assert_eq!(saved_json["podcast"]["polling_interval_minutes"], 60);
    assert_eq!(saved_json["podcast"]["quiet_hours"]["start"], "21:30");
}

#[test]
fn missing_podcast_preferences_fall_back_to_reference_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("station_scout.json");
    std::fs::write(&path, "{}").unwrap();
    let store = SettingsStore::new(path);

    let loaded = store.load().expect("state should load");

    assert_eq!(loaded.podcast.polling_interval_minutes, 30);
    assert!(!loaded.podcast.quiet_hours.enabled);
    assert_eq!(loaded.podcast.quiet_hours.start, "22:00");
    assert_eq!(loaded.podcast.quiet_hours.end, "07:00");
    assert!(!loaded.podcast.start_with_windows);
    assert_eq!(loaded.podcast.playback_speed, 1.0);
    assert_eq!(loaded.podcast.sleep_timer_default_minutes, 15);
    assert_eq!(loaded.podcast.openrouter_model, "");
    assert!(!loaded.podcast.first_run_completed);
    assert_eq!(loaded.podcast.schema_version, 1);
}

#[test]
fn corrupt_settings_json_falls_back_to_defaults_so_app_can_boot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("station_scout.json");
    std::fs::write(&path, "{not valid json").unwrap();
    let store = SettingsStore::new(path);

    let loaded = store
        .load()
        .expect("corrupt settings should not block startup");

    assert_eq!(loaded, AppState::default());
}

#[test]
fn settings_path_uses_appdata_station_scout_folder() {
    assert_eq!(
        station_scout_settings_path(Some(Path::new("C:/Users/Example/AppData/Roaming")), None),
        Path::new("C:/Users/Example/AppData/Roaming")
            .join("Station Scout")
            .join("station_scout.json")
    );
}

#[test]
fn settings_path_falls_back_to_home_appdata_roaming() {
    assert_eq!(
        station_scout_settings_path(None, Some(Path::new("C:/Users/Example"))),
        Path::new("C:/Users/Example")
            .join("AppData")
            .join("Roaming")
            .join("Station Scout")
            .join("station_scout.json")
    );
}

#[test]
fn track_log_folder_defaults_next_to_settings_file_and_allows_override() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::new(dir.path().join("station_scout.json"));
    let default_state = AppState::default();
    let custom_state = AppState {
        track_log_folder: dir.path().join("custom logs").to_string_lossy().to_string(),
        ..AppState::default()
    };

    assert_eq!(
        store.default_track_log_folder(),
        dir.path().join("track sessions")
    );
    assert_eq!(
        store.track_log_folder(&default_state),
        dir.path().join("track sessions")
    );
    assert_eq!(
        store.track_log_folder(&custom_state),
        dir.path().join("custom logs")
    );
}

#[test]
fn add_unique_station_moves_existing_station_to_front_and_applies_limit() {
    let first = Station::new("abc", "First", "https://one.test");
    let second = Station::new("def", "Second", "https://two.test");
    let updated = add_unique_station(vec![first.clone(), second.clone()], first.clone(), Some(1));

    assert_eq!(updated, vec![first]);
}

#[test]
fn podcast_repository_subscribes_reactivates_and_preserves_history() {
    let conn = open_memory_database().expect("database should open");
    let parsed = ParsedFeed {
        feed_title: Some("Example Podcast".to_string()),
        feed_author: Some("Host".to_string()),
        feed_image_url: None,
        feed_description: Some("Description".to_string()),
        feed_link: None,
        episodes: vec![Episode {
            guid: Some("episode-1".to_string()),
            title: Some("First Episode".to_string()),
            enclosure_url: Some("https://cdn.example.test/one.mp3".to_string()),
            enclosure_url_norm: Some("https://cdn.example.test/one.mp3".to_string()),
            title_pubdate_hash: Some("hash".to_string()),
            pub_date: Some("Tue, 09 May 2026 17:00:00 GMT".to_string()),
            duration_seconds: Some(1800),
            description_html: Some("<p>Hello</p>".to_string()),
            transcript: None,
            chapters_url: None,
        }],
    };

    let subscription_id =
        subscribe(&conn, "https://feeds.example/show.xml", Some(&parsed), None).unwrap();
    assert_eq!(
        upsert_episodes(&conn, subscription_id, &parsed.episodes).unwrap(),
        1
    );
    assert_eq!(
        upsert_episodes(&conn, subscription_id, &parsed.episodes).unwrap(),
        0
    );
    unsubscribe(&conn, subscription_id).unwrap();
    assert!(list_active_subscriptions(&conn).unwrap().is_empty());
    let reactivated = subscribe(
        &conn,
        "https://feeds.example/show.xml",
        None,
        Some("Fallback"),
    )
    .unwrap();

    assert_eq!(reactivated, subscription_id);
    let active = list_active_subscriptions(&conn).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].title.as_deref(), Some("Fallback"));
    let episode_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episodes WHERE subscription_id = ?1",
            [subscription_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(episode_count, 1);
    assert!(
        get_feed_urls(&conn)
            .unwrap()
            .contains("https://feeds.example/show.xml")
    );
}

#[test]
fn podcast_repository_persists_per_show_settings_with_dialog_bounds() {
    let conn = open_memory_database().expect("database should open");
    let subscription_id = subscribe(
        &conn,
        "https://feeds.example/per-show.xml",
        None,
        Some("Per Show"),
    )
    .unwrap();

    assert_eq!(
        get_subscription_settings(&conn, subscription_id)
            .unwrap()
            .unwrap(),
        SubscriptionSettings {
            notification_rule: NotificationRule::Always,
            skip_silence: false,
            intro_skip_seconds: 0,
            outro_skip_seconds: 0,
        }
    );

    update_subscription_settings(
        &conn,
        subscription_id,
        &SubscriptionSettings {
            notification_rule: NotificationRule::Digest,
            skip_silence: true,
            intro_skip_seconds: 45,
            outro_skip_seconds: 75,
        },
    )
    .unwrap();

    let settings = get_subscription_settings(&conn, subscription_id)
        .unwrap()
        .unwrap();
    assert_eq!(settings.notification_rule, NotificationRule::Digest);
    assert!(settings.skip_silence);
    assert_eq!(settings.intro_skip_seconds, 45);
    assert_eq!(settings.outro_skip_seconds, 75);

    update_subscription_settings(
        &conn,
        subscription_id,
        &SubscriptionSettings {
            notification_rule: NotificationRule::Never,
            skip_silence: true,
            intro_skip_seconds: -5,
            outro_skip_seconds: 700,
        },
    )
    .unwrap();

    let clamped = get_subscription_settings(&conn, subscription_id)
        .unwrap()
        .unwrap();
    assert_eq!(clamped.notification_rule, NotificationRule::Never);
    assert_eq!(clamped.intro_skip_seconds, 0);
    assert_eq!(clamped.outro_skip_seconds, 600);
}

#[test]
fn podcast_repository_persists_playback_position_and_played_state() {
    let conn = open_memory_database().expect("database should open");
    let episode_id = seed_episode(&conn);

    set_playback_position(&conn, episode_id, 95_000).unwrap();
    let state = get_played_state(&conn, episode_id).unwrap().unwrap();
    assert_eq!(state.position_ms, 95_000);
    assert!(!state.is_played);

    mark_episode_played(&conn, episode_id).unwrap();
    let state = get_played_state(&conn, episode_id).unwrap().unwrap();
    assert!(state.is_played);
    assert!(state.played_at.is_some());
}

#[test]
fn podcast_repository_persists_reorders_and_removes_queue() {
    let conn = open_memory_database().expect("database should open");
    let first = seed_episode(&conn);
    let second = seed_episode_with_guid(&conn, "episode-2", "Second Episode");

    enqueue_episode(&conn, first).unwrap();
    enqueue_episode(&conn, second).unwrap();
    enqueue_episode(&conn, first).unwrap();
    assert_eq!(
        list_queue(&conn)
            .unwrap()
            .into_iter()
            .map(|entry| entry.episode_id)
            .collect::<Vec<_>>(),
        vec![first, second]
    );

    move_queued_episode(&conn, second, 0).unwrap();
    assert_eq!(list_queue(&conn).unwrap()[0].episode_id, second);

    remove_queued_episode(&conn, second).unwrap();
    assert_eq!(
        list_queue(&conn)
            .unwrap()
            .into_iter()
            .map(|entry| entry.episode_id)
            .collect::<Vec<_>>(),
        vec![first]
    );
}

#[test]
fn podcast_repository_clears_queue_without_deleting_episodes() {
    let conn = open_memory_database().expect("database should open");
    let first = seed_episode(&conn);
    let second = seed_episode_with_guid(&conn, "episode-2", "Second Episode");

    enqueue_episode(&conn, first).unwrap();
    enqueue_episode(&conn, second).unwrap();
    clear_queue(&conn).unwrap();

    assert!(list_queue(&conn).unwrap().is_empty());
    let episode_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(episode_count, 2);
}

#[test]
fn podcast_repository_lists_episodes_with_played_and_queue_state() {
    let conn = open_memory_database().expect("database should open");
    let first = seed_episode_with_guid(&conn, "episode-1", "First Episode");
    let second = seed_episode_with_guid(&conn, "episode-2", "Second Episode");
    let subscription_id: i64 = conn
        .query_row(
            "SELECT subscription_id FROM episodes WHERE id = ?1",
            [first],
            |row| row.get(0),
        )
        .unwrap();

    set_playback_position(&conn, first, 42_000).unwrap();
    mark_episode_played(&conn, second).unwrap();
    enqueue_episode(&conn, second).unwrap();

    let episodes = list_episodes(&conn, subscription_id).unwrap();

    assert_eq!(episodes.len(), 2);
    assert_eq!(episodes[0].title.as_deref(), Some("First Episode"));
    assert_eq!(episodes[0].position_ms, 42_000);
    assert!(!episodes[0].is_played);
    assert!(!episodes[0].is_queued);
    assert_eq!(episodes[1].title.as_deref(), Some("Second Episode"));
    assert!(episodes[1].is_played);
    assert!(episodes[1].is_queued);
    assert_eq!(episodes[1].queue_position, Some(0));
}

#[test]
fn podcast_repository_tracks_poll_success_and_due_subscriptions() {
    let conn = open_memory_database().expect("database should open");
    let subscription_id = subscribe(
        &conn,
        "https://feeds.example/show.xml",
        None,
        Some("Example Show"),
    )
    .unwrap();

    let due = list_due_subscriptions(&conn, "2026-05-28T12:00:00Z", 10).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id, subscription_id);
    assert_eq!(due[0].etag, None);

    mark_poll_success(
        &conn,
        subscription_id,
        Some("\"etag-1\""),
        Some("Tue, 09 May 2026 17:00:00 GMT"),
        "2026-05-28T13:00:00Z",
    )
    .unwrap();

    assert!(
        list_due_subscriptions(&conn, "2026-05-28T12:30:00Z", 10)
            .unwrap()
            .is_empty()
    );
    let due = list_due_subscriptions(&conn, "2026-05-28T13:00:00Z", 10).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].etag.as_deref(), Some("\"etag-1\""));
    assert_eq!(
        due[0].last_modified.as_deref(),
        Some("Tue, 09 May 2026 17:00:00 GMT")
    );
}

#[test]
fn podcast_repository_records_poll_failures_without_polling_tombstones() {
    let conn = open_memory_database().expect("database should open");
    let active_id = subscribe(
        &conn,
        "https://feeds.example/active.xml",
        None,
        Some("Active Show"),
    )
    .unwrap();
    let tombstone_id = subscribe(
        &conn,
        "https://feeds.example/old.xml",
        None,
        Some("Old Show"),
    )
    .unwrap();
    unsubscribe(&conn, tombstone_id).unwrap();

    mark_poll_failure(&conn, active_id, "2026-05-28T13:15:00Z").unwrap();
    mark_poll_failure(&conn, active_id, "2026-05-28T13:30:00Z").unwrap();
    mark_poll_failure(&conn, active_id, "2026-05-28T14:00:00Z").unwrap();

    let due = list_due_subscriptions(&conn, "2026-05-28T14:00:00Z", 10).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id, active_id);
    assert_eq!(due[0].consecutive_fails, 3);
    assert_eq!(due[0].health_state, "unhealthy");
}

#[test]
fn podcast_repository_persists_notification_inbox_events() {
    let conn = open_memory_database().expect("database should open");
    let episode_id = seed_episode(&conn);

    let first_id =
        log_notification_event(&conn, episode_id, "2026-05-28T12:00:00Z", "FireToast", None)
            .unwrap();
    log_notification_event(
        &conn,
        episode_id,
        "2026-05-28T12:05:00Z",
        "InboxOnly",
        Some("quiet_hours"),
    )
    .unwrap();
    mark_notification_toast_fired(&conn, first_id, "2026-05-28T12:01:00Z").unwrap();

    let inbox = list_notification_inbox(&conn, 10).unwrap();

    assert_eq!(inbox.len(), 2);
    assert_eq!(inbox[0].action, "InboxOnly");
    assert_eq!(inbox[0].suppression_reason.as_deref(), Some("quiet_hours"));
    assert_eq!(inbox[0].episode_title.as_deref(), Some("First Episode"));
    assert_eq!(inbox[0].show_title.as_deref(), Some("Example Podcast"));
    assert!(!inbox[0].toast_fired);
    assert_eq!(inbox[1].id, first_id);
    assert!(inbox[1].toast_fired);
    assert_eq!(inbox[1].fired_at.as_deref(), Some("2026-05-28T12:01:00Z"));

    clear_notification_inbox(&conn).unwrap();
    assert!(list_notification_inbox(&conn, 10).unwrap().is_empty());
}

#[test]
fn podcast_repository_tracks_cache_index_leases_and_resume_state() {
    let conn = open_memory_database().expect("database should open");
    let episode_id = seed_episode(&conn);

    update_cache_entry(
        &conn,
        episode_id,
        "episodes/episode-1/audio.mp3",
        2048,
        Some(4096),
    )
    .unwrap();
    lease_cache_entry(&conn, episode_id).unwrap();
    lease_cache_entry(&conn, episode_id).unwrap();
    release_cache_entry(&conn, episode_id).unwrap();
    mark_cache_was_playing(&conn, episode_id, true).unwrap();

    let entry = get_cache_entry(&conn, episode_id)
        .unwrap()
        .expect("cache entry should exist");

    assert_eq!(entry.episode_id, episode_id);
    assert_eq!(entry.cache_path, "episodes/episode-1/audio.mp3");
    assert_eq!(entry.bytes_cached, 2048);
    assert_eq!(entry.total_bytes, Some(4096));
    assert_eq!(entry.lease_count, 1);
    assert!(entry.was_playing);

    release_cache_entry(&conn, episode_id).unwrap();
    release_cache_entry(&conn, episode_id).unwrap();
    let entry = get_cache_entry(&conn, episode_id).unwrap().unwrap();
    assert_eq!(entry.lease_count, 0);
}

fn seed_episode(conn: &rusqlite::Connection) -> i64 {
    seed_episode_with_guid(conn, "episode-1", "First Episode")
}

fn seed_episode_with_guid(conn: &rusqlite::Connection, guid: &str, title: &str) -> i64 {
    let parsed = ParsedFeed {
        feed_title: Some("Example Podcast".to_string()),
        feed_author: Some("Host".to_string()),
        feed_image_url: None,
        feed_description: None,
        feed_link: None,
        episodes: vec![Episode {
            guid: Some(guid.to_string()),
            title: Some(title.to_string()),
            enclosure_url: Some(format!("https://cdn.example.test/{guid}.mp3")),
            enclosure_url_norm: Some(format!("https://cdn.example.test/{guid}.mp3")),
            title_pubdate_hash: Some(format!("hash-{guid}")),
            pub_date: Some("Tue, 09 May 2026 17:00:00 GMT".to_string()),
            duration_seconds: Some(1800),
            description_html: None,
            transcript: None,
            chapters_url: None,
        }],
    };
    let subscription_id =
        subscribe(conn, "https://feeds.example/show.xml", Some(&parsed), None).unwrap();
    upsert_episodes(conn, subscription_id, &parsed.episodes).unwrap();
    conn.query_row("SELECT id FROM episodes WHERE guid = ?1", [guid], |row| {
        row.get(0)
    })
    .unwrap()
}
