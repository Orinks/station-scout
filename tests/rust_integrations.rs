use station_scout::cast::station_content_type;
use station_scout::integrations_config::{
    DEFAULT_LASTFM_PROXY_URL, LastFmAppConfig, lastfm_app_config_from_values, local_env_value,
    spotify_app_config_from_values, spotify_loopback_redirect_base, spotify_proxy_config,
};
use station_scout::lastfm::{
    LastFmClient, LastFmProxyClient, LastFmScrobbleCache, lastfm_settings_view_state,
    should_scrobble_lastfm, should_show_stream_title, sign_lastfm_request,
};
use station_scout::models::Station;
use station_scout::spotify::{
    DEFAULT_SCOPES, SpotifyClient, build_spotify_auth_request, spotify_playlist_tracks,
};
use station_scout::storage::AppState;
use station_scout::tracklog::TrackEntry;
use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn spotify_auth_request_uses_pkce_and_playlist_scopes() {
    let request = build_spotify_auth_request(
        "client",
        "http://127.0.0.1:49152/spotify/callback",
        DEFAULT_SCOPES,
    );

    assert!(
        request
            .url
            .starts_with("https://accounts.spotify.com/authorize?")
    );
    assert!(request.url.contains("client_id=client"));
    assert!(
        request
            .url
            .contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A49152%2Fspotify%2Fcallback")
    );
    assert!(request.url.contains("code_challenge_method=S256"));
    assert!(request.url.contains("playlist-modify-private"));
    assert!(!request.state.is_empty());
    assert!(!request.code_verifier.is_empty());
}

#[test]
fn spotify_client_exchanges_code_with_pkce_verifier() {
    let server = one_request_server(&json_response(
        r#"{"access_token":"access","refresh_token":"refresh","expires_in":3600}"#,
    ));
    let client = SpotifyClient::new_with_token_url("client", "http://callback", &server.url);
    let before = unix_seconds();

    let token_set = client
        .exchange_code("code", "verifier")
        .expect("token exchange should succeed");
    let after = unix_seconds();

    let request = server.request.join().expect("server thread should finish");
    assert_eq!(token_set.access_token, "access");
    assert_eq!(token_set.refresh_token, "refresh");
    assert_eq!(token_set.expires_in, 3600);
    assert!(token_set.expires_at >= before + 3600);
    assert!(token_set.expires_at <= after + 3600);
    assert!(request.starts_with("POST /"));
    assert!(request.contains("grant_type=authorization_code"));
    assert!(request.contains("code=code"));
    assert!(request.contains("code_verifier=verifier"));
    assert!(request.contains("redirect_uri=http%3A%2F%2Fcallback"));
}

#[test]
fn spotify_client_refresh_preserves_existing_refresh_token_when_omitted() {
    let server = one_request_server(&json_response(
        r#"{"access_token":"new-access","expires_in":3600}"#,
    ));
    let client = SpotifyClient::new_with_token_url("client", "http://callback", &server.url);

    let token_set = client
        .refresh_access_token("old-refresh")
        .expect("token refresh should succeed");

    let request = server.request.join().expect("server thread should finish");
    assert_eq!(token_set.access_token, "new-access");
    assert_eq!(token_set.refresh_token, "old-refresh");
    assert_eq!(token_set.expires_in, 3600);
    assert!(request.contains("grant_type=refresh_token"));
    assert!(request.contains("refresh_token=old-refresh"));
}

#[test]
fn spotify_client_creates_playlist_from_matched_tracks() {
    let server = request_sequence_server(vec![
        json_response(r#"{"id":"user"}"#),
        json_response(
            r#"{"id":"playlist","external_urls":{"spotify":"https://open.spotify.com/playlist/playlist"}}"#,
        ),
        json_response(r#"{"tracks":{"items":[{"uri":"spotify:track:one"}]}}"#),
        json_response(r#"{"tracks":{"items":[]}}"#),
        json_response("{}"),
    ]);
    let client = SpotifyClient::new_with_urls(
        "client",
        "http://callback",
        "http://127.0.0.1:9/token",
        &server.url,
    );

    let result = client
        .create_playlist_from_tracks(
            "access",
            "Show",
            &[
                ("Artist".to_string(), "Song".to_string()),
                ("Missing".to_string(), "Song".to_string()),
            ],
        )
        .expect("playlist should be created");

    let requests = server.requests.join().expect("server thread should finish");
    assert_eq!(result.name, "Show");
    assert_eq!(result.url, "https://open.spotify.com/playlist/playlist");
    assert_eq!(result.matched_tracks, 1);
    assert_eq!(result.skipped_tracks, 1);
    assert!(requests[0].starts_with("GET /me "));
    assert!(requests[1].starts_with("POST /users/user/playlists "));
    assert!(requests[1].contains(r#""name":"Show""#));
    assert!(requests[2].starts_with("GET /search?"));
    assert!(requests[4].starts_with("POST /playlists/playlist/tracks "));
    assert!(requests[4].contains(r#""uris":["spotify:track:one"]"#));
}

#[test]
fn spotify_playlist_tracks_skips_uncertain_and_deduplicates() {
    let entries = vec![
        TrackEntry::new("Artist", "Song", "Artist - Song"),
        TrackEntry::new("Artist", "Song", "Artist - Song"),
        TrackEntry {
            uncertain: true,
            ..TrackEntry::new("DJ", "Sweeper", "raw")
        },
    ];

    assert_eq!(
        spotify_playlist_tracks(&entries),
        vec![("Artist".to_string(), "Song".to_string())]
    );
}

#[test]
fn lastfm_signature_sorts_params_and_appends_secret() {
    let params = BTreeMap::from([
        ("track".to_string(), "Song".to_string()),
        ("artist".to_string(), "Artist".to_string()),
        ("api_key".to_string(), "key".to_string()),
    ]);

    assert_eq!(
        sign_lastfm_request(&params, "secret"),
        "76b2da4c153c8e14e28bae1c9c7d90d7"
    );
}

#[test]
fn lastfm_client_scrobbles_radio_tracks_as_not_chosen_by_user() {
    let server = one_request_server(&xml_response(r#"<lfm status="ok"></lfm>"#));
    let client = LastFmClient::new_with_api_root("key", "secret", "session", &server.url);

    client
        .scrobble(&TrackEntry::new("Artist", "Song", "Artist - Song"))
        .expect("scrobble should succeed");

    let request = server.request.join().expect("server thread should finish");
    assert!(request.starts_with("POST /"));
    assert!(request.contains("method=track.scrobble"));
    assert!(request.contains("artist=Artist"));
    assert!(request.contains("track=Song"));
    assert!(request.contains("chosenByUser=0"));
    assert!(request.contains("api_key=key"));
    assert!(request.contains("sk=session"));
    assert!(request.contains("api_sig="));
}

#[test]
fn lastfm_browser_auth_flow_gets_token_and_session() {
    let server = request_sequence_server(vec![
        xml_response(r#"<lfm status="ok"><token>abc</token></lfm>"#),
        xml_response(
            r#"<lfm status="ok"><session><name>listener</name><key>session</key></session></lfm>"#,
        ),
    ]);
    let client = LastFmClient::new_with_api_root("key", "secret", "", &server.url);

    let token = client.request_token().expect("token should be returned");
    let (session_key, username) = client
        .create_session(&token)
        .expect("session should be returned");

    let requests = server.requests.join().expect("server thread should finish");
    assert_eq!(token, "abc");
    assert_eq!(
        client.auth_url(&token),
        "https://www.last.fm/api/auth/?api_key=key&token=abc"
    );
    assert_eq!(session_key, "session");
    assert_eq!(username, "listener");
    assert!(requests[0].contains("method=auth.getToken"));
    assert!(requests[1].contains("method=auth.getSession"));
    assert!(requests[1].contains("token=abc"));
}

#[test]
fn lastfm_proxy_flow_gets_token_and_session() {
    let server = request_sequence_server(vec![
        json_response(r#"{"token":"abc","authUrl":"https://www.last.fm/api/auth/?token=abc"}"#),
        json_response(r#"{"sessionKey":"session","username":"listener"}"#),
    ]);
    let client = LastFmProxyClient::new(&server.url, "");

    let (token, auth_url) = client.request_token().expect("proxy token should succeed");
    let (session_key, username) = client
        .create_session(&token)
        .expect("proxy session should succeed");

    let requests = server.requests.join().expect("server thread should finish");
    assert_eq!(token, "abc");
    assert_eq!(auth_url, "https://www.last.fm/api/auth/?token=abc");
    assert_eq!(session_key, "session");
    assert_eq!(username, "listener");
    assert!(requests[0].starts_with("POST /token "));
    assert!(requests[1].starts_with("POST /session "));
    assert!(requests[1].contains(r#""token":"abc""#));
}

#[test]
fn lastfm_proxy_scrobbles_radio_tracks() {
    let server = request_sequence_server(vec![json_response("{}"), json_response("{}")]);
    let client = LastFmProxyClient::new(&server.url, "session");

    client
        .update_now_playing(&TrackEntry::new("Artist", "Song", "Artist - Song"))
        .expect("now playing should succeed");
    client
        .scrobble(&TrackEntry::new("Artist", "Song", "Artist - Song"))
        .expect("scrobble should succeed");

    let requests = server.requests.join().expect("server thread should finish");
    assert!(requests[0].starts_with("POST /now-playing "));
    assert!(requests[0].contains(r#""sessionKey":"session""#));
    assert!(requests[0].contains(r#""artist":"Artist""#));
    assert!(requests[0].contains(r#""track":"Song""#));
    assert!(requests[1].starts_with("POST /scrobble "));
    assert!(requests[1].contains(r#""timestamp":"#));
}

#[test]
fn lastfm_scrobble_cache_skips_uncertain_entries() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let path = temp.path().join("lastfm-scrobble-cache.jsonl");
    let cache = LastFmScrobbleCache::new(&path);

    cache
        .append(&TrackEntry {
            uncertain: true,
            ..TrackEntry::new("", "", "Station ID")
        })
        .expect("cache append should succeed");

    assert!(!path.exists());
}

#[test]
fn lastfm_failure_reports_retryability() {
    let server = one_request_server(&xml_response(
        r#"<lfm status="failed"><error code="16">Try later</error></lfm>"#,
    ));
    let client = LastFmClient::new_with_api_root("key", "secret", "session", &server.url);

    let error = client
        .scrobble(&TrackEntry::new("Artist", "Song", "Artist - Song"))
        .expect_err("Last.fm failure should surface");

    let _request = server.request.join().expect("server thread should finish");
    assert!(error.to_string().contains("Try later"));
    assert!(error.to_string().contains("retryable"));
    assert!(error.to_string().contains("code 16"));
}

#[test]
fn lastfm_scrobble_cache_appends_valid_entries_as_jsonl() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let path = temp.path().join("lastfm-scrobble-cache.jsonl");
    let cache = LastFmScrobbleCache::new(&path);

    cache
        .append(&TrackEntry::new("Artist", "Song", "Artist - Song"))
        .expect("cache append should succeed");

    let text = std::fs::read_to_string(path).expect("cache should be readable");
    let line: serde_json::Value =
        serde_json::from_str(text.trim()).expect("cache line should be JSON");
    assert_eq!(line["artist"], "Artist");
    assert_eq!(line["title"], "Song");
    assert_eq!(line["raw"], "Artist - Song");
    assert!(line["timestamp"].is_number());
}

#[test]
fn lastfm_scrobble_skips_uncertain_or_duplicate_tracks() {
    let mut state = AppState {
        lastfm_enabled: true,
        lastfm_scrobble_enabled: true,
        ..AppState::default()
    };
    let entry = TrackEntry::new("Artist", "Song", "raw");
    let sent = HashSet::from(["artist\0song".to_string()]);

    assert!(!should_scrobble_lastfm(&state, &entry, &sent));
    state.lastfm_scrobble_enabled = false;
    assert!(!should_scrobble_lastfm(&state, &entry, &HashSet::new()));
}

#[test]
fn lastfm_settings_view_state_matches_connection_state() {
    let connected = AppState {
        lastfm_enabled: true,
        lastfm_username: "Orinks".to_string(),
        lastfm_pending_token: "old".to_string(),
        ..AppState::default()
    };
    assert_eq!(
        lastfm_settings_view_state(&connected),
        (
            "Connected as Orinks.".to_string(),
            "Reconnect Last.fm".to_string(),
            false
        )
    );

    let pending = AppState {
        lastfm_pending_token: "pending".to_string(),
        ..AppState::default()
    };
    assert_eq!(
        lastfm_settings_view_state(&pending),
        (
            "Waiting for browser approval. Choose Finish Last.fm afterward.".to_string(),
            "Restart Last.fm".to_string(),
            true
        )
    );
}

#[test]
fn uncertain_station_id_metadata_does_not_replace_now_playing() {
    let entry = TrackEntry {
        uncertain: true,
        ..TrackEntry::new("Philly's #1 Hit Music Station", "Q102", "raw")
    };

    assert!(!should_show_stream_title(&entry));
}

#[test]
fn station_content_type_uses_hls_codec_and_default() {
    let hls = Station {
        hls: 1,
        ..Station::new("abc", "Example", "https://example.test/live")
    };
    let aac = Station {
        codec: "AAC".to_string(),
        ..Station::new("def", "Example", "https://example.test/live")
    };
    let unknown = Station::new("ghi", "Example", "https://example.test/live");

    assert_eq!(
        station_content_type(&hls, &hls.url_resolved),
        "application/x-mpegURL"
    );
    assert_eq!(station_content_type(&aac, &aac.url_resolved), "audio/aac");
    assert_eq!(
        station_content_type(&unknown, &unknown.url_resolved),
        "audio/mpeg"
    );
}

#[test]
fn spotify_loopback_redirect_normalizes_localhost_and_drops_port() {
    assert_eq!(
        spotify_loopback_redirect_base("http://localhost:8765/spotify/callback"),
        "http://127.0.0.1/spotify/callback"
    );
    assert_eq!(
        spotify_loopback_redirect_base("https://example.test/callback"),
        "https://example.test/callback"
    );
}

#[test]
fn lastfm_config_trims_proxy_url() {
    let config = LastFmAppConfig::proxy("https://example.test/api/lastfm/");

    assert_eq!(config.proxy_url, "https://example.test/api/lastfm");
}

#[test]
fn lastfm_config_defaults_to_proxy_and_supports_direct_credentials_when_proxy_disabled() {
    let proxy = lastfm_app_config_from_values(None, "", "").expect("default proxy should resolve");
    assert_eq!(proxy.proxy_url, DEFAULT_LASTFM_PROXY_URL);

    let direct = lastfm_app_config_from_values(Some(""), " key ", " secret ")
        .expect("direct Last.fm credentials should resolve when proxy is disabled");
    assert_eq!(direct.api_key, "key");
    assert_eq!(direct.api_secret, "secret");
    assert!(direct.proxy_url.is_empty());

    assert!(lastfm_app_config_from_values(Some(""), "key", "").is_none());
}

#[test]
fn spotify_config_normalizes_loopback_redirect_for_client_id_sources() {
    let config =
        spotify_app_config_from_values(" client ", Some("http://localhost:8765/spotify/callback"))
            .expect("client id should resolve");

    assert_eq!(config.client_id, "client");
    assert_eq!(config.redirect_uri, "http://127.0.0.1/spotify/callback");
    assert!(spotify_app_config_from_values("", None).is_none());
}

#[test]
fn spotify_proxy_config_fetches_public_client_id_without_network_in_tests() {
    let server = one_request_server(&json_response(
        r#"{"clientId":"proxy-client","redirectUri":"http://localhost:8765/spotify/callback"}"#,
    ));

    let config = spotify_proxy_config(&server.url).expect("proxy config should resolve");

    let request = server.request.join().expect("server thread should finish");
    assert!(request.starts_with("GET / "));
    assert_eq!(config.client_id, "proxy-client");
    assert_eq!(config.redirect_uri, "http://127.0.0.1/spotify/callback");
}

#[test]
fn local_env_value_reads_equals_colon_and_following_line_formats() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let path = temp.path().join(".env.local");
    std::fs::write(
        &path,
        "STATION_SCOUT_SPOTIFY_CLIENT_ID='eq-client'\nSpotify client ID: colon-client\nOpenRouter\nnext-line\n",
    )
    .expect("env file should write");

    assert_eq!(
        local_env_value(&path, "STATION_SCOUT_SPOTIFY_CLIENT_ID"),
        "eq-client"
    );
    assert_eq!(local_env_value(&path, "Spotify client ID"), "colon-client");
    assert_eq!(local_env_value(&path, "OpenRouter"), "next-line");
    assert_eq!(local_env_value(&path, "missing"), "");
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

fn json_response(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

fn xml_response(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0; 512];
    let mut header_end = None;
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("request should be readable");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if let Some(position) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            header_end = Some(position + 4);
            break;
        }
    }
    if let Some(header_end) = header_end {
        let headers = String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
        let content_length = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length: "))
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let read = stream
                .read(&mut buffer)
                .expect("request body should be readable");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
    }
    String::from_utf8(request).expect("request should be UTF-8")
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_secs() as i64
}
