use station_scout::models::{BrowseFacetKind, TuneTimer};
use station_scout::notifications::NotificationRule;
use station_scout::podcasts::discovery::PodcastSearchClient;
use station_scout::podcasts::repository::{SubscriptionSettings, open_database, subscribe};
use station_scout::radio_browser::RadioBrowserClient;
use station_scout::runtime::{
    RuntimeStoragePaths, StationScoutRuntime, is_forced_portable_mode, resolve_runtime_storage,
};
use station_scout::storage::{AppState, CredentialStore, MemoryCredentialStore, SettingsStore};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

#[test]
fn runtime_bootstrap_loads_settings_and_opens_podcast_database() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let credentials = MemoryCredentialStore::default();
    let store =
        SettingsStore::new(settings_path.clone()).with_credentials(Box::new(credentials.clone()));
    store
        .save(&AppState {
            track_log_folder: dir.path().join("logs").to_string_lossy().to_string(),
            ..AppState::default()
        })
        .expect("settings should save");

    let runtime = StationScoutRuntime::open(settings_path.clone(), Box::new(credentials))
        .expect("runtime should open");
    let schema_version: i64 = runtime
        .podcast_database
        .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
        .expect("schema version should exist");

    assert_eq!(runtime.settings_path, settings_path);
    assert_eq!(
        runtime.state.track_log_folder,
        dir.path().join("logs").to_string_lossy()
    );
    assert_eq!(runtime.track_log_folder, dir.path().join("logs"));
    assert_eq!(
        runtime.podcast_database_path,
        dir.path().join("Station Scout").join("podcatcher.sqlite3")
    );
    assert_eq!(schema_version, 1);
    assert!(runtime.podcast_database_path.exists());
    assert!(runtime.track_log_folder.exists());
}

#[test]
fn runtime_reports_whether_active_podcast_subscriptions_exist() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");

    let runtime = StationScoutRuntime::open(
        settings_path.clone(),
        Box::new(MemoryCredentialStore::default()),
    )
    .expect("runtime should open without subscriptions");
    let podcast_database_path = runtime.podcast_database_path.clone();

    assert!(!runtime.has_active_podcast_subscriptions);
    drop(runtime);

    let conn = open_database(&podcast_database_path).expect("database should reopen");
    subscribe(
        &conn,
        "https://feeds.example.org/station-scout.xml",
        None,
        Some("Station Scout Daily"),
    )
    .expect("subscription should save");
    drop(conn);

    let runtime =
        StationScoutRuntime::open(settings_path, Box::new(MemoryCredentialStore::default()))
            .expect("runtime should reopen with subscriptions");

    assert!(runtime.has_active_podcast_subscriptions);
}

#[test]
fn runtime_imports_and_exports_podcast_opml_through_app_actions() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let mut runtime =
        StationScoutRuntime::open(settings_path, Box::new(MemoryCredentialStore::default()))
            .expect("runtime should open");
    let payload = br#"<?xml version="1.0"?>
<opml version="2.0">
  <body>
    <outline text="Station Scout Daily" type="rss" xmlUrl="https://feeds.example/daily.xml" />
  </body>
</opml>"#;

    let import = runtime
        .import_podcast_opml(payload.as_slice())
        .expect("OPML import should succeed");
    let export = runtime
        .export_podcast_opml()
        .expect("OPML export should succeed");
    let text = String::from_utf8(export).expect("OPML should be UTF-8");

    assert_eq!(import.added, 1);
    assert!(runtime.has_active_podcast_subscriptions);
    assert!(text.contains(r#"text="Station Scout Daily""#));
    assert!(text.contains(r#"xmlUrl="https://feeds.example/daily.xml""#));
}

#[test]
fn runtime_subscribes_to_podcast_feed_url_through_app_action() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let mut runtime =
        StationScoutRuntime::open(settings_path, Box::new(MemoryCredentialStore::default()))
            .expect("runtime should open");
    let server = one_request_server(&http_response(
        200,
        &[("ETag", "\"feed-v1\"")],
        sample_feed("episode-1", "First Episode").as_bytes(),
    ));

    let result = runtime
        .subscribe_to_podcast_feed(&server.url, "2026-05-28T13:00:00Z")
        .expect("feed URL subscription should succeed");

    let request = server.request.join().expect("server thread should finish");
    assert!(request.starts_with("GET / "));
    assert_eq!(result.title.as_deref(), Some("Example Podcast"));
    assert_eq!(result.episodes_added, 1);
    assert!(runtime.has_active_podcast_subscriptions);
}

#[test]
fn runtime_polls_due_podcast_feeds_through_refresh_action() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let mut runtime =
        StationScoutRuntime::open(settings_path, Box::new(MemoryCredentialStore::default()))
            .expect("runtime should open");
    let server = request_sequence_server(vec![
        http_response(
            200,
            &[("ETag", "\"feed-v1\"")],
            sample_feed("episode-1", "First Episode").as_bytes(),
        ),
        http_response(304, &[], b""),
    ]);

    runtime
        .subscribe_to_podcast_feed(&server.url, "2026-05-28T13:00:00Z")
        .expect("feed URL subscription should succeed");
    let summary = runtime
        .poll_due_podcast_feeds("2026-05-28T13:00:00Z", "2026-05-28T14:00:00Z", 10)
        .expect("refresh should succeed");

    let requests = server.requests.join().expect("server thread should finish");
    assert_eq!(requests.len(), 2);
    assert!(requests[1].to_ascii_lowercase().contains("if-none-match"));
    assert_eq!(summary.feeds_checked, 1);
    assert_eq!(summary.not_modified, 1);
    assert_eq!(summary.new_episodes, 0);
    assert_eq!(summary.failures, 0);
}

#[test]
fn runtime_searches_podcasts_with_saved_podcast_index_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let credentials = MemoryCredentialStore::default();
    let store =
        SettingsStore::new(settings_path.clone()).with_credentials(Box::new(credentials.clone()));
    store
        .save(&AppState {
            podcast_index_api_key: "KEY".to_string(),
            podcast_index_api_secret: "SECRET".to_string(),
            ..AppState::default()
        })
        .expect("settings should save");
    let runtime = StationScoutRuntime::open(settings_path.clone(), Box::new(credentials))
        .expect("runtime should open");
    assert_eq!(runtime.state.podcast_index_api_key, "KEY");
    assert_eq!(runtime.state.podcast_index_api_secret, "SECRET");
    let server = one_request_server(&http_response(
        200,
        &[],
        br#"{"feeds":[{"url":"https://feeds.example/show.xml","title":"Show","author":"Host"}]}"#,
    ));
    let client = PodcastSearchClient::new(server.url.clone(), "http://127.0.0.1:9/search");

    let outcome = runtime
        .search_podcasts_with_client(&client, "rust radio", "1716900000", 40)
        .expect("runtime podcast search should succeed");

    let request = server.request.join().expect("server thread should finish");
    let normalized = request.to_ascii_lowercase();
    assert!(request.starts_with("GET /?"));
    assert!(normalized.contains("q=rust"));
    assert!(normalized.contains("max=40"));
    assert!(request.contains("X-Auth-Key: KEY") || normalized.contains("x-auth-key: key"));
    assert_eq!(outcome.source, "podcast_index");
    assert!(!outcome.fallback_used);
    assert_eq!(outcome.results.len(), 1);
    assert_eq!(outcome.results[0].title, "Show");
}

#[test]
fn runtime_exposes_python_radio_actions_for_search_favorites_recents_and_timers() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let mut runtime = StationScoutRuntime::open(
        settings_path.clone(),
        Box::new(MemoryCredentialStore::default()),
    )
    .expect("runtime should open");
    let server = one_request_server(&http_response(
        200,
        &[],
        br#"[{"stationuuid":"abc","name":"Example FM","url_resolved":"https://stream.example/live","homepage":"https://example.test","country":"United States","language":"english","tags":"news","bitrate":128,"lastcheckok":1}]"#,
    ));
    let client = RadioBrowserClient::new([server.url.clone()]);

    let stations = runtime
        .search_radio_with_client(&client, "Example", "US", "english", "news", 25)
        .expect("radio search should use Radio Browser");
    runtime
        .add_radio_favorite(stations[0].clone())
        .expect("favorite should save");
    runtime
        .record_radio_recent(stations[0].clone())
        .expect("recent should save");
    runtime
        .add_tune_timer(TuneTimer::new("abc", "Example FM", "07:30"))
        .expect("timer should save");
    runtime.save_settings().expect("settings should save");
    drop(runtime);

    let reopened =
        StationScoutRuntime::open(settings_path, Box::new(MemoryCredentialStore::default()))
            .expect("runtime should reopen");
    assert_eq!(stations.len(), 1);
    assert_eq!(reopened.state.favorites[0].name, "Example FM");
    assert_eq!(reopened.state.recents[0].stationuuid, "abc");
    assert_eq!(reopened.state.timers[0].time, "07:30");
    let request = server.request.join().expect("server thread should finish");
    assert!(request.starts_with("GET /json/stations/search?"));
    assert!(request.contains("name=Example"));
}

#[test]
fn runtime_exposes_python_radio_browse_actions() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let runtime =
        StationScoutRuntime::open(settings_path, Box::new(MemoryCredentialStore::default()))
            .expect("runtime should open");
    let server = request_sequence_server(vec![
        http_response(
            200,
            &[],
            br#"[{"name":"jazz","stationcount":120},{"name":"!!!","stationcount":1}]"#,
        ),
        http_response(
            200,
            &[],
            br#"[{"stationuuid":"jazz-1","name":"Jazz FM","url_resolved":"https://stream.example/jazz","bitrate":128,"lastcheckok":1}]"#,
        ),
    ]);
    let client = RadioBrowserClient::new([server.url.clone()]);

    let facets = runtime
        .browse_radio_facets_with_client(&client, BrowseFacetKind::Genre, 25)
        .expect("browse facets should load");
    let stations = runtime
        .browse_radio_stations_with_client(&client, &facets[0], 10)
        .expect("browse stations should load");

    assert_eq!(facets.len(), 1);
    assert_eq!(facets[0].name, "jazz");
    assert_eq!(stations[0].name, "Jazz FM");
    let requests = server.requests.join().expect("server thread should finish");
    assert!(requests[0].starts_with("GET /json/tags?"));
    assert!(requests[1].starts_with("GET /json/stations/search?"));
    assert!(requests[1].contains("tag=jazz"));
}

#[test]
fn runtime_exposes_episode_queue_played_state_and_unsubscribe_actions() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let mut runtime =
        StationScoutRuntime::open(settings_path, Box::new(MemoryCredentialStore::default()))
            .expect("runtime should open");
    let servers = request_sequence_server(vec![
        http_response(
            200,
            &[("ETag", "\"feed-v1\"")],
            sample_feed("episode-1", "First Episode").as_bytes(),
        ),
        http_response(
            200,
            &[("ETag", "\"feed-v2\"")],
            sample_feed("episode-2", "Second Episode").as_bytes(),
        ),
    ]);

    runtime
        .subscribe_to_podcast_feed(&format!("{}?feed=one", servers.url), "2026-05-28T13:00:00Z")
        .expect("first feed should subscribe");
    runtime
        .subscribe_to_podcast_feed(&format!("{}?feed=two", servers.url), "2026-05-28T13:00:00Z")
        .expect("second feed should subscribe");
    let subscriptions = runtime
        .list_podcast_subscriptions()
        .expect("subscriptions should list");
    let first_episodes = runtime
        .list_podcast_episodes(subscriptions[0].id)
        .expect("episodes should list");
    let second_episodes = runtime
        .list_podcast_episodes(subscriptions[1].id)
        .expect("episodes should list");
    let first_id = first_episodes[0].id;
    let second_id = second_episodes[0].id;

    runtime
        .enqueue_podcast_episode(first_id)
        .expect("first episode should queue");
    runtime
        .enqueue_podcast_episode(second_id)
        .expect("second episode should queue");
    runtime
        .move_queued_podcast_episode(second_id, 0)
        .expect("queued episode should move");
    let queue = runtime
        .list_podcast_queue()
        .expect("queue should list after move");
    assert_eq!(queue[0].episode_id, second_id);
    assert_eq!(queue[1].episode_id, first_id);

    runtime
        .set_podcast_playback_position(second_id, 42_000)
        .expect("playback position should persist");
    runtime
        .mark_podcast_episode_played(second_id)
        .expect("played state should persist");
    let second_episodes = runtime
        .list_podcast_episodes(subscriptions[1].id)
        .expect("episodes should list after played state update");
    assert!(second_episodes[0].is_played);
    assert_eq!(second_episodes[0].position_ms, 42_000);

    runtime
        .remove_queued_podcast_episode(second_id)
        .expect("queued episode should be removed");
    let queue = runtime
        .list_podcast_queue()
        .expect("queue should list after remove");
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].position, 0);
    runtime
        .clear_podcast_queue()
        .expect("queue should clear through runtime action");
    assert!(
        runtime
            .list_podcast_queue()
            .expect("queue should list after clear")
            .is_empty()
    );

    runtime
        .unsubscribe_from_podcast(subscriptions[0].id)
        .expect("subscription should unsubscribe");
    assert_eq!(
        runtime
            .list_podcast_subscriptions()
            .expect("subscriptions should list after unsubscribe")
            .len(),
        1
    );
    assert!(runtime.has_active_podcast_subscriptions);
    let _requests = servers
        .requests
        .join()
        .expect("server thread should finish");
}

#[test]
fn runtime_exposes_per_show_podcast_settings_actions() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let mut runtime =
        StationScoutRuntime::open(settings_path, Box::new(MemoryCredentialStore::default()))
            .expect("runtime should open");
    let server = one_request_server(&http_response(
        200,
        &[],
        sample_feed("episode-1", "First Episode").as_bytes(),
    ));

    runtime
        .subscribe_to_podcast_feed(&server.url, "2026-05-28T13:00:00Z")
        .expect("feed should subscribe");
    let subscription_id = runtime
        .list_podcast_subscriptions()
        .expect("subscriptions should list")[0]
        .id;
    let defaults = runtime
        .get_podcast_subscription_settings(subscription_id)
        .expect("settings should load")
        .expect("subscription settings should exist");
    assert_eq!(defaults.notification_rule, NotificationRule::Always);
    assert!(!defaults.skip_silence);

    runtime
        .update_podcast_subscription_settings(
            subscription_id,
            &SubscriptionSettings {
                notification_rule: NotificationRule::Digest,
                skip_silence: true,
                intro_skip_seconds: 30,
                outro_skip_seconds: 45,
            },
        )
        .expect("settings should update");
    let updated = runtime
        .get_podcast_subscription_settings(subscription_id)
        .expect("settings should reload")
        .expect("subscription settings should exist");

    assert_eq!(updated.notification_rule, NotificationRule::Digest);
    assert!(updated.skip_silence);
    assert_eq!(updated.intro_skip_seconds, 30);
    assert_eq!(updated.outro_skip_seconds, 45);
    let _request = server.request.join().expect("server thread should finish");
}

#[test]
fn runtime_saves_global_podcast_preferences_and_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let credentials = MemoryCredentialStore::default();
    let mut runtime =
        StationScoutRuntime::open(settings_path.clone(), Box::new(credentials.clone()))
            .expect("runtime should open");

    runtime.state.podcast.polling_interval_minutes = 45;
    runtime.state.podcast.playback_speed = 1.75;
    runtime.state.podcast.sleep_timer_default_minutes = 60;
    runtime.state.podcast_index_api_key = "PI-KEY".to_string();
    runtime.state.podcast_index_api_secret = "PI-SECRET".to_string();
    runtime.state.openrouter_api_key = "OPENROUTER".to_string();
    runtime
        .save_settings()
        .expect("runtime settings should save");
    drop(runtime);

    let reopened = StationScoutRuntime::open(settings_path.clone(), Box::new(credentials.clone()))
        .expect("runtime should reopen");
    let settings_text = std::fs::read_to_string(settings_path).expect("settings file should exist");

    assert_eq!(reopened.state.podcast.polling_interval_minutes, 45);
    assert_eq!(reopened.state.podcast.playback_speed, 1.75);
    assert_eq!(reopened.state.podcast.sleep_timer_default_minutes, 60);
    assert_eq!(reopened.state.podcast_index_api_key, "PI-KEY");
    assert_eq!(reopened.state.podcast_index_api_secret, "PI-SECRET");
    assert_eq!(reopened.state.openrouter_api_key, "OPENROUTER");
    assert_eq!(credentials.get("podcast_index_key"), "PI-KEY");
    assert_eq!(credentials.get("podcast_index_secret"), "PI-SECRET");
    assert_eq!(credentials.get("openrouter"), "OPENROUTER");
    assert!(!settings_text.contains("PI-KEY"));
    assert!(!settings_text.contains("PI-SECRET"));
    assert!(!settings_text.contains("OPENROUTER"));
}

#[test]
fn runtime_exposes_notification_inbox_and_cache_metadata_actions() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("Station Scout").join("station_scout.json");
    let mut runtime =
        StationScoutRuntime::open(settings_path, Box::new(MemoryCredentialStore::default()))
            .expect("runtime should open");
    let server = one_request_server(&http_response(
        200,
        &[],
        sample_feed("episode-1", "First Episode").as_bytes(),
    ));

    runtime
        .subscribe_to_podcast_feed(&server.url, "2026-05-28T13:00:00Z")
        .expect("feed should subscribe");
    let subscription_id = runtime
        .list_podcast_subscriptions()
        .expect("subscriptions should list")[0]
        .id;
    let episode_id = runtime
        .list_podcast_episodes(subscription_id)
        .expect("episodes should list")[0]
        .id;

    let notification_id = runtime
        .log_podcast_notification_event(
            episode_id,
            "2026-05-28T12:00:00Z",
            "InboxOnly",
            Some("quiet_hours"),
        )
        .expect("notification event should log");
    runtime
        .mark_podcast_notification_toast_fired(notification_id, "2026-05-28T12:01:00Z")
        .expect("toast fired state should persist");
    let inbox = runtime
        .list_podcast_notification_inbox(10)
        .expect("notification inbox should list");
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].episode_id, episode_id);
    assert!(inbox[0].toast_fired);
    assert_eq!(inbox[0].suppression_reason.as_deref(), Some("quiet_hours"));

    runtime
        .update_podcast_cache_entry(episode_id, "episodes/episode-1/audio.mp3", 2048, Some(4096))
        .expect("cache metadata should save");
    runtime
        .lease_podcast_cache_entry(episode_id)
        .expect("cache lease should increment");
    runtime
        .mark_podcast_cache_was_playing(episode_id, true)
        .expect("cache resume state should save");
    let cache = runtime
        .get_podcast_cache_entry(episode_id)
        .expect("cache metadata should load")
        .expect("cache metadata should exist");
    assert_eq!(cache.cache_path, "episodes/episode-1/audio.mp3");
    assert_eq!(cache.bytes_cached, 2048);
    assert_eq!(cache.total_bytes, Some(4096));
    assert_eq!(cache.lease_count, 1);
    assert!(cache.was_playing);

    runtime
        .release_podcast_cache_entry(episode_id)
        .expect("cache lease should decrement");
    assert_eq!(
        runtime
            .get_podcast_cache_entry(episode_id)
            .expect("cache metadata should reload")
            .expect("cache metadata should exist")
            .lease_count,
        0
    );
    runtime
        .clear_podcast_notification_inbox()
        .expect("notification inbox should clear");
    assert!(
        runtime
            .list_podcast_notification_inbox(10)
            .expect("notification inbox should list after clear")
            .is_empty()
    );
    let _request = server.request.join().expect("server thread should finish");
}

#[test]
fn runtime_storage_paths_name_all_station_and_podcast_files() {
    let dir = tempfile::tempdir().unwrap();
    let paths = RuntimeStoragePaths::new(
        dir.path().join("Config"),
        dir.path().join("Data"),
        dir.path().join("Cache"),
        dir.path().join("Logs"),
        false,
        false,
    );

    assert_eq!(
        paths.settings_file(),
        dir.path().join("Config").join("station_scout.json")
    );
    assert_eq!(
        paths.podcast_database_file(),
        dir.path().join("Data").join("podcatcher.sqlite3")
    );
    assert_eq!(
        paths.episode_cache_root(),
        dir.path().join("Cache").join("episodes")
    );
    assert_eq!(
        paths.log_file(),
        dir.path().join("Logs").join("station_scout.log")
    );

    paths
        .ensure_dirs()
        .expect("runtime directories should be created");
    assert!(paths.config_root.is_dir());
    assert!(paths.data_root.is_dir());
    assert!(paths.episode_cache_root().is_dir());
}

#[test]
fn resolve_runtime_storage_supports_custom_and_portable_layouts() {
    let dir = tempfile::tempdir().unwrap();
    let custom = dir.path().join("custom");
    let paths = resolve_runtime_storage(
        Some(custom.clone()),
        false,
        Some(Path::new("C:/Users/Example/AppData/Roaming")),
        Some(Path::new("C:/Users/Example")),
        Path::new("C:/Program Files/Station Scout"),
    );

    assert!(paths.custom_config_dir);
    assert!(!paths.portable_mode);
    assert_eq!(paths.config_root, custom);
    assert_eq!(paths.data_root, custom);
    assert_eq!(
        paths.podcast_database_file(),
        dir.path().join("custom").join("podcatcher.sqlite3")
    );

    let portable =
        resolve_runtime_storage(None, true, None, None, Path::new("D:/Tools/Station Scout"));

    assert!(portable.portable_mode);
    assert_eq!(
        portable.settings_file(),
        Path::new("D:/Tools/Station Scout")
            .join("config")
            .join("station_scout.json")
    );
    assert_eq!(
        portable.episode_cache_root(),
        Path::new("D:/Tools/Station Scout")
            .join("config")
            .join("cache")
            .join("episodes")
    );
}

#[test]
fn forced_portable_mode_uses_station_scout_env_flag() {
    unsafe {
        std::env::remove_var("STATION_SCOUT_FORCE_PORTABLE");
        assert!(!is_forced_portable_mode());
        std::env::set_var("STATION_SCOUT_FORCE_PORTABLE", "YES");
        assert!(is_forced_portable_mode());
        std::env::set_var("STATION_SCOUT_FORCE_PORTABLE", "0");
        assert!(!is_forced_portable_mode());
        std::env::remove_var("STATION_SCOUT_FORCE_PORTABLE");
    }
}

fn sample_feed(guid: &str, title: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Example Podcast</title>
    <item>
      <guid>{guid}</guid>
      <title>{title}</title>
      <pubDate>Tue, 09 May 2026 17:00:00 GMT</pubDate>
      <enclosure url="https://cdn.example.test/{guid}.mp3" type="audio/mpeg" />
    </item>
  </channel>
</rss>"#
    )
}

fn http_response(status: u16, headers: &[(&str, &str)], body: &[u8]) -> String {
    let reason = match status {
        200 => "OK",
        304 => "Not Modified",
        _ => "Status",
    };
    let mut response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    response.push_str(&String::from_utf8_lossy(body));
    response
}

struct TestServer {
    url: String,
    request: thread::JoinHandle<String>,
}

struct SequenceServer {
    url: String,
    requests: thread::JoinHandle<Vec<String>>,
}

fn one_request_server(response: &str) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let response = response.to_string();
    let request = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test server should accept");
        let request = read_request(&mut stream);
        stream
            .write_all(response.as_bytes())
            .expect("test server should respond");
        request
    });
    TestServer { url, request }
}

fn request_sequence_server(responses: Vec<String>) -> SequenceServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let requests = thread::spawn(move || {
        let mut requests = Vec::new();
        for response in responses {
            let (mut stream, _) = listener.accept().expect("test server should accept");
            requests.push(read_request(&mut stream));
            stream
                .write_all(response.as_bytes())
                .expect("test server should respond");
        }
        requests
    });
    SequenceServer { url, requests }
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0; 512];
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("request should be readable");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(request).expect("request should be UTF-8")
}
