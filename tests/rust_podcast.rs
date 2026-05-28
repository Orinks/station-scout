use station_scout::podcasts::discovery::{
    PodcastIndexCredentials, PodcastSearchClient, build_podcast_index_auth_headers,
    podcast_index_credentials_from_state,
};
use station_scout::podcasts::feed::{
    compute_title_pubdate_hash, normalize_enclosure_url, parse_feed,
};
use station_scout::podcasts::feed_client::{
    FeedFetchError, feed_fetch_headers, fetch_feed_sync, validate_feed_url,
};
use station_scout::podcasts::opml::{import_opml, parse_opml, render_opml, validate_opml_rows};
use station_scout::podcasts::repository::{
    list_active_subscriptions, list_due_subscriptions, mark_poll_success, open_database,
    open_memory_database, subscribe, unsubscribe,
};
use station_scout::podcasts::service::{
    export_opml_subscriptions, import_opml_subscriptions, poll_due_feeds, subscribe_to_feed,
};
use station_scout::storage::AppState;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

#[test]
fn opml_import_classifies_added_duplicate_bad_scheme_and_malformed_rows() {
    let payload = br#"<?xml version="1.0"?>
<opml version="2.0">
  <body>
    <outline text="New Show" type="rss" xmlUrl="https://feeds.example/new.xml" />
    <outline text="Old Show" type="rss" xmlUrl="https://feeds.example/old.xml" />
    <outline text="Bad Scheme" type="rss" xmlUrl="file:///secret.xml" />
    <outline text="Missing URL" type="rss" />
  </body>
</opml>"#;

    let result = import_opml(
        payload.as_slice(),
        ["https://feeds.example/old.xml".to_string()]
            .into_iter()
            .collect(),
    )
    .expect("OPML should parse");

    assert_eq!(result.added, 1);
    assert_eq!(result.skipped_duplicate, 1);
    assert_eq!(result.rejected_scheme, 1);
    assert_eq!(result.malformed, 1);
    assert_eq!(result.rows[0].status, "added");
    assert_eq!(result.rows[2].status, "rejected_scheme");
}

#[test]
fn opml_parser_rejects_external_entity_payloads() {
    let payload = br#"<?xml version="1.0"?>
<!DOCTYPE opml [
  <!ENTITY xxe SYSTEM "file:///c:/windows/win.ini">
]>
<opml version="2.0"><body><outline text="Unsafe" type="rss" xmlUrl="&xxe;" /></body></opml>"#;

    let error = parse_opml(payload.as_slice()).expect_err("XXE payload should be rejected");

    assert!(error.to_string().contains("external entity"));
}

#[test]
fn opml_export_renders_subscription_feed_urls() {
    let xml = render_opml(vec![(
        "Station Scout Daily".to_string(),
        "https://feeds.example/daily.xml".to_string(),
        Some("https://example.test/daily".to_string()),
    )]);
    let text = String::from_utf8(xml).expect("OPML should be UTF-8");

    assert!(text.contains(r#"xmlUrl="https://feeds.example/daily.xml""#));
    assert!(text.contains(r#"htmlUrl="https://example.test/daily""#));
    assert!(text.contains("Station Scout subscriptions"));
}

#[test]
fn opml_validate_rows_keeps_parse_pure() {
    let rows = parse_opml(
        br#"<opml version="2.0"><body><outline text="Show" type="rss" xmlUrl="https://feed.test/rss" /></body></opml>"#
            .as_slice(),
    )
    .expect("OPML should parse");

    let result = validate_opml_rows(rows, Default::default());

    assert_eq!(result.added, 1);
    assert_eq!(
        result.rows[0].feed_url.as_deref(),
        Some("https://feed.test/rss")
    );
}

#[test]
fn podcast_service_imports_opml_rows_into_database_with_reference_counts() {
    let conn = open_memory_database().expect("database should open");
    subscribe(
        &conn,
        "https://feeds.example/old.xml",
        None,
        Some("Old Show"),
    )
    .expect("existing subscription should persist");
    let payload = br#"<?xml version="1.0"?>
<opml version="2.0">
  <body>
    <outline text="New Show" type="rss" xmlUrl="https://feeds.example/new.xml" />
    <outline text="Old Show" type="rss" xmlUrl="https://feeds.example/old.xml" />
    <outline text="Bad Scheme" type="rss" xmlUrl="file:///secret.xml" />
    <outline text="Missing URL" type="rss" />
  </body>
</opml>"#;

    let summary = import_opml_subscriptions(&conn, payload.as_slice()).expect("OPML imports");

    assert_eq!(summary.added, 1);
    assert_eq!(summary.skipped_duplicate, 1);
    assert_eq!(summary.rejected_scheme, 1);
    assert_eq!(summary.malformed, 1);
    let active = list_active_subscriptions(&conn).unwrap();
    assert_eq!(active.len(), 2);
    assert!(active.iter().any(|row| {
        row.feed_url == "https://feeds.example/new.xml" && row.title.as_deref() == Some("New Show")
    }));
}

#[test]
fn podcast_service_exports_active_database_subscriptions_as_opml() {
    let conn = open_memory_database().expect("database should open");
    subscribe(
        &conn,
        "https://feeds.example/active.xml",
        None,
        Some("Active Show"),
    )
    .expect("active subscription should persist");
    let inactive_id = subscribe(
        &conn,
        "https://feeds.example/inactive.xml",
        None,
        Some("Inactive Show"),
    )
    .expect("inactive subscription should persist first");
    unsubscribe(&conn, inactive_id).expect("inactive subscription should be hidden");

    let payload = export_opml_subscriptions(&conn).expect("OPML exports");
    let text = String::from_utf8(payload).expect("OPML should be UTF-8");

    assert!(text.contains("Station Scout subscriptions"));
    assert!(text.contains(r#"text="Active Show""#));
    assert!(text.contains(r#"xmlUrl="https://feeds.example/active.xml""#));
    assert!(!text.contains("Inactive Show"));
    assert!(!text.contains("https://feeds.example/inactive.xml"));
}

#[test]
fn podcast_database_enables_wal_foreign_keys_and_cache_index() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("podcatcher.sqlite3");
    let conn = open_database(&path).expect("podcast database should open");

    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("journal mode should be readable");
    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("foreign key pragma should be readable");
    let cache_index_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'cache_index'",
            [],
            |row| row.get(0),
        )
        .expect("cache_index table should be queryable");

    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    assert_eq!(foreign_keys, 1);
    assert_eq!(cache_index_count, 1);
    assert!(
        conn.execute(
            "INSERT INTO episodes (subscription_id, first_seen_at) VALUES (?1, ?2)",
            (123_i64, "2026-05-28T12:00:00Z"),
        )
        .is_err()
    );
}

#[test]
fn feed_parser_extracts_podcast_episode_metadata() {
    let feed = br#"<?xml version="1.0"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" xmlns:podcast="https://podcastindex.org/namespace/1.0">
  <channel>
    <title>Example Podcast</title>
    <itunes:author>Example Host</itunes:author>
    <description>Show description</description>
    <image><url>https://example.test/art.png</url></image>
    <item>
      <guid>episode-1</guid>
      <title>First Episode</title>
      <pubDate>Tue, 09 May 2026 17:00:00 GMT</pubDate>
      <itunes:duration>01:02:03</itunes:duration>
      <description><![CDATA[<p>Hello <strong>world</strong></p>]]></description>
      <enclosure url="https://pdst.fm/e/https://cdn.example.test/audio.mp3" type="audio/mpeg" />
      <podcast:transcript url="https://example.test/transcript.vtt" type="text/vtt" language="en" />
      <podcast:chapters url="https://example.test/chapters.json" type="application/json+chapters" />
    </item>
  </channel>
</rss>"#;

    let parsed = parse_feed(feed.as_slice()).expect("feed should parse");

    assert_eq!(parsed.feed_title.as_deref(), Some("Example Podcast"));
    assert_eq!(parsed.feed_author.as_deref(), Some("Example Host"));
    assert_eq!(
        parsed.feed_image_url.as_deref(),
        Some("https://example.test/art.png")
    );
    assert_eq!(parsed.episodes.len(), 1);
    let episode = &parsed.episodes[0];
    assert_eq!(episode.guid.as_deref(), Some("episode-1"));
    assert_eq!(episode.title.as_deref(), Some("First Episode"));
    assert_eq!(episode.duration_seconds, Some(3723));
    assert_eq!(
        episode.enclosure_url_norm.as_deref(),
        Some("https://cdn.example.test/audio.mp3")
    );
    assert_eq!(
        episode.transcript.as_ref().unwrap().mime_type.as_deref(),
        Some("text/vtt")
    );
    assert_eq!(
        episode.chapters_url.as_deref(),
        Some("https://example.test/chapters.json")
    );
}

#[test]
fn feed_parser_treats_malformed_xml_as_empty_feed_like_reference_app() {
    let parsed = parse_feed(br#"<not-rss>hello</not-rss>"#.as_slice())
        .expect("malformed feed payloads should not abort parsing");

    assert_eq!(parsed.feed_title, None);
    assert!(parsed.episodes.is_empty());
}

#[test]
fn feed_identity_helpers_match_python_reference_behavior() {
    assert_eq!(
        normalize_enclosure_url("https://op3.dev/e/https://media.example.test/show.mp3"),
        Some("https://media.example.test/show.mp3".to_string())
    );
    assert_eq!(
        compute_title_pubdate_hash("Episode", "Tue, 09 May 2026 17:00:00 GMT"),
        Some("0eec3e950f23d0382a0021df8a2682ea6bc55130".to_string())
    );
}

#[test]
fn podcast_discovery_builds_podcast_index_auth_headers() {
    let headers = build_podcast_index_auth_headers(
        &PodcastIndexCredentials {
            key: "KEY".to_string(),
            secret: "SECRET".to_string(),
        },
        "1716900000",
    );

    assert_eq!(headers.get("X-Auth-Key").map(String::as_str), Some("KEY"));
    assert_eq!(
        headers.get("X-Auth-Date").map(String::as_str),
        Some("1716900000")
    );
    assert_eq!(
        headers.get("Authorization").map(String::as_str),
        Some("1b34fa7abff099b1e9bebef1642f23a356371f5c")
    );
}

#[test]
fn podcast_discovery_credentials_are_built_from_loaded_app_state() {
    let mut state = AppState {
        podcast_index_api_key: " KEY ".to_string(),
        podcast_index_api_secret: " SECRET ".to_string(),
        ..AppState::default()
    };

    assert_eq!(
        podcast_index_credentials_from_state(&state),
        Some(PodcastIndexCredentials {
            key: "KEY".to_string(),
            secret: "SECRET".to_string(),
        })
    );

    state.podcast_index_api_secret.clear();

    assert_eq!(podcast_index_credentials_from_state(&state), None);
}

#[test]
fn podcast_discovery_uses_podcast_index_when_credentials_are_present() {
    let server = one_request_server(&http_response(
        200,
        &[],
        br#"{"feeds":[{"url":"https://feeds.example/show.xml","title":"Show","author":"Host","description":"Desc","image":"https://example.test/art.png"},{"url":"file:///secret.xml","title":"Bad"}]}"#,
    ));
    let client = PodcastSearchClient::new(server.url.clone(), "http://127.0.0.1:9/search");

    let outcome = client
        .search(
            "rust radio",
            Some(&PodcastIndexCredentials {
                key: "KEY".to_string(),
                secret: "SECRET".to_string(),
            }),
            "1716900000",
            40,
        )
        .expect("podcast index search should succeed");

    let request = server.request.join().expect("server thread should finish");
    let normalized = request.to_ascii_lowercase();
    assert!(request.starts_with("GET /?"));
    assert!(normalized.contains("q=rust"));
    assert!(normalized.contains("max=40"));
    assert!(request.contains("X-Auth-Key: KEY") || normalized.contains("x-auth-key: key"));
    assert_eq!(outcome.source, "podcast_index");
    assert!(!outcome.fallback_used);
    assert_eq!(outcome.results.len(), 1);
    assert_eq!(
        outcome.results[0].feed_url,
        "https://feeds.example/show.xml"
    );
    assert_eq!(outcome.results[0].title, "Show");
    assert_eq!(outcome.results[0].source, "podcast_index");
}

#[test]
fn podcast_discovery_reports_podcast_index_auth_failure() {
    let server = one_request_server(&http_response(401, &[], b""));
    let client = PodcastSearchClient::new(server.url.clone(), "http://127.0.0.1:9/search");

    let error = client
        .search(
            "rust radio",
            Some(&PodcastIndexCredentials {
                key: "BAD".to_string(),
                secret: "SECRET".to_string(),
            }),
            "1716900000",
            40,
        )
        .expect_err("invalid Podcast Index credentials should fail distinctly");

    let _request = server.request.join().expect("server thread should finish");
    assert!(error.is_auth_error());
    assert!(error.to_string().contains("Podcast Index rejected"));
}

#[test]
fn podcast_discovery_falls_back_to_itunes_without_credentials() {
    let server = one_request_server(&http_response(
        200,
        &[],
        br#"{"results":[{"feedUrl":"https://feeds.example/itunes.xml","collectionName":"iTunes Show","artistName":"iTunes Author","artworkUrl600":"https://example.test/art.jpg"}]}"#,
    ));
    let client = PodcastSearchClient::new("http://127.0.0.1:9/search", server.url.clone());

    let outcome = client
        .search("station scout", None, "1716900000", 500)
        .expect("itunes fallback should succeed");

    let request = server.request.join().expect("server thread should finish");
    let normalized = request.to_ascii_lowercase();
    assert!(normalized.contains("term=station"));
    assert!(normalized.contains("entity=podcast"));
    assert!(normalized.contains("limit=200"));
    assert_eq!(outcome.source, "itunes");
    assert!(outcome.fallback_used);
    assert_eq!(outcome.results.len(), 1);
    assert_eq!(outcome.results[0].title, "iTunes Show");
    assert_eq!(
        outcome.results[0].feed_url,
        "https://feeds.example/itunes.xml"
    );
}

#[test]
fn feed_client_rejects_non_http_urls_before_fetch() {
    assert!(validate_feed_url("https://feeds.example/show.xml").is_ok());
    assert!(validate_feed_url("http://feeds.example/show.xml").is_ok());
    assert!(validate_feed_url("file:///c:/secret.xml").is_err());
    assert!(validate_feed_url("javascript:alert(1)").is_err());
    assert!(validate_feed_url("").is_err());
}

#[test]
fn feed_client_builds_conditional_get_headers() {
    let headers = feed_fetch_headers(
        Some(r#""etag-value""#),
        Some("Tue, 09 May 2026 17:00:00 GMT"),
    );

    assert_eq!(
        headers.get("User-Agent").map(String::as_str),
        Some("StationScout/1.0")
    );
    assert_eq!(
        headers.get("If-None-Match").map(String::as_str),
        Some(r#""etag-value""#)
    );
    assert_eq!(
        headers.get("If-Modified-Since").map(String::as_str),
        Some("Tue, 09 May 2026 17:00:00 GMT")
    );
}

#[test]
fn feed_client_fetches_http_body_and_response_metadata() {
    let server = one_request_server(
        "HTTP/1.1 200 OK\r\nETag: \"abc\"\r\nLast-Modified: Tue, 09 May 2026 17:00:00 GMT\r\nContent-Length: 6\r\n\r\n<rss/>",
    );

    let response = fetch_feed_sync(&server.url, None, None).expect("feed should fetch");

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"<rss/>");
    assert_eq!(response.etag.as_deref(), Some("\"abc\""));
    assert_eq!(
        response.last_modified.as_deref(),
        Some("Tue, 09 May 2026 17:00:00 GMT")
    );
    assert!(!response.not_modified);
}

#[test]
fn feed_client_sends_conditional_get_headers() {
    let server = one_request_server("HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\n\r\n");

    let response = fetch_feed_sync(
        &server.url,
        Some("\"old-etag\""),
        Some("Wed, 21 Oct 2024 07:28:00 GMT"),
    )
    .expect("304 is a successful conditional outcome");

    let request = server.request.join().expect("server thread should finish");
    let normalized_request = request.to_ascii_lowercase();
    assert!(normalized_request.contains("if-none-match: \"old-etag\""));
    assert!(normalized_request.contains("if-modified-since: wed, 21 oct 2024 07:28:00 gmt"));
    assert_eq!(response.status, 304);
    assert!(response.body.is_empty());
    assert!(response.not_modified);
}

#[test]
fn feed_client_raises_on_non_success_response() {
    let server =
        one_request_server("HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n");

    let error = fetch_feed_sync(&server.url, None, None).expect_err("5xx should fail");

    assert_eq!(error.status, Some(503));
    assert!(matches!(error, FeedFetchError { .. }));
}

#[test]
fn podcast_service_subscribes_by_url_fetches_feed_and_persists_poll_metadata() {
    let conn = open_memory_database().expect("database should open");
    let server = one_request_server(&http_response(
        200,
        &[
            ("ETag", "\"feed-v1\""),
            ("Last-Modified", "Tue, 09 May 2026 17:00:00 GMT"),
        ],
        sample_feed("episode-1", "First Episode").as_bytes(),
    ));

    let result =
        subscribe_to_feed(&conn, &server.url, "2026-05-28T13:00:00Z").expect("feed subscribes");

    assert_eq!(result.episodes_added, 1);
    assert_eq!(result.title.as_deref(), Some("Example Podcast"));
    let active = list_active_subscriptions(&conn).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].title.as_deref(), Some("Example Podcast"));
    assert_eq!(active[0].feed_url, server.url);
    let episode_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(episode_count, 1);
    assert!(
        list_due_subscriptions(&conn, "2026-05-28T12:59:59Z", 10)
            .unwrap()
            .is_empty()
    );
    let due = list_due_subscriptions(&conn, "2026-05-28T13:00:00Z", 10).unwrap();
    assert_eq!(due[0].etag.as_deref(), Some("\"feed-v1\""));
    assert_eq!(
        due[0].last_modified.as_deref(),
        Some("Tue, 09 May 2026 17:00:00 GMT")
    );
}

#[test]
fn podcast_service_polls_due_feeds_with_conditionals_and_records_304() {
    let conn = open_memory_database().expect("database should open");
    let server = one_request_server(&http_response(304, &[], b""));
    let subscription_id = subscribe(&conn, &server.url, None, Some("Example Podcast")).unwrap();
    mark_poll_success(
        &conn,
        subscription_id,
        Some("\"feed-v1\""),
        Some("Tue, 09 May 2026 17:00:00 GMT"),
        "2026-05-28T12:00:00Z",
    )
    .unwrap();

    let summary =
        poll_due_feeds(&conn, "2026-05-28T12:00:00Z", "2026-05-28T13:00:00Z", 10).unwrap();

    let request = server.request.join().expect("server thread should finish");
    let normalized_request = request.to_ascii_lowercase();
    assert!(normalized_request.contains("if-none-match: \"feed-v1\""));
    assert!(normalized_request.contains("if-modified-since: tue, 09 may 2026 17:00:00 gmt"));
    assert_eq!(summary.feeds_checked, 1);
    assert_eq!(summary.not_modified, 1);
    assert_eq!(summary.new_episodes, 0);
    assert_eq!(summary.failures, 0);
    assert!(
        list_due_subscriptions(&conn, "2026-05-28T12:59:59Z", 10)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn podcast_repository_opens_file_database_and_creates_schema() {
    let dir = tempfile::tempdir().unwrap();
    let database_path = dir.path().join("data").join("podcatcher.sqlite3");

    let conn = open_database(&database_path).expect("database should open");
    let version: i64 = conn
        .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
        .expect("schema version should exist");
    drop(conn);

    assert_eq!(version, 1);
    assert!(database_path.exists());
}

fn sample_feed(guid: &str, title: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Example Podcast</title>
    <description>Show description</description>
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
        503 => "Service Unavailable",
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
