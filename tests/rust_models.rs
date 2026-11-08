use station_scout::direct_streams::{station_from_direct_url, streamurl_search_url};
use station_scout::models::{BrowseFacet, BrowseFacetKind, Station, TuneTimer};
use station_scout::radio_browser::{
    BrowseEndpoint, DEFAULT_STATION_RESULT_LIMIT_INDEX, MAX_STATION_RESULTS, RadioBrowserClient,
    STATION_RESULT_LIMITS, browse_facet_request, clean_browse_facet, discovery_sorted,
    filter_choice_label, filter_dropdown_choices, resolve_filter_value, search_request,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

#[test]
fn station_normalizes_api_payload() {
    let station = Station::from_api(serde_json::json!({
        "stationuuid": "abc",
        "name": "Example",
        "url": "https://stream.example/live",
        "bitrate": "128",
        "lastcheckok": "1"
    }));

    assert_eq!(station.stationuuid, "abc");
    assert_eq!(station.url_resolved, "https://stream.example/live");
    assert_eq!(station.bitrate, 128);
    assert!(station.quality_label().contains("working"));
}

#[test]
fn timer_round_trips_json() {
    let timer = TuneTimer {
        stationuuid: "abc".to_string(),
        station_name: "Example".to_string(),
        time: "07:30".to_string(),
        enabled: true,
        auto_play: true,
        end_time: "10:30".to_string(),
        show_name: "Morning Show".to_string(),
        track_playlist: true,
    };

    assert_eq!(TuneTimer::from_json(timer.to_json()), timer);
}

#[test]
fn direct_stream_station_has_stable_id_and_source() {
    let station =
        Station::direct_stream("Example", "https://stream.test/live", "", "StreamURL.link");

    assert!(station.stationuuid.starts_with("direct-"));
    assert_eq!(station.source, "StreamURL.link");
    assert!(station.quality_label().contains("StreamURL.link"));
}

#[test]
fn browse_facet_normalizes_api_payload_and_query_value() {
    let facet = BrowseFacet::from_api(
        BrowseFacetKind::Location,
        serde_json::json!({"name": "United States", "stationcount": "123", "iso_3166_1": "US"}),
    );

    assert_eq!(facet.name, "United States");
    assert_eq!(facet.station_count, 123);
    assert_eq!(facet.query_value(), "US");
}

#[test]
fn station_discovery_quality_scores_reliable_popular_streams_higher() {
    let strong = Station::from_api(serde_json::json!({
        "stationuuid": "good",
        "name": "Good",
        "url": "https://good.test/live",
        "lastcheckok": 1,
        "bitrate": 320,
        "codec": "AAC",
        "clicktrend": 3,
        "votes": 250
    }));
    let weak = Station::from_api(serde_json::json!({
        "stationuuid": "weak",
        "name": "Weak",
        "url": "https://weak.test/live",
        "lastcheckok": 1,
        "bitrate": 64,
        "codec": "MP3",
        "clicktrend": 0
    }));

    assert!(strong.discovery_quality_score() > weak.discovery_quality_score());
    assert!(
        strong
            .discovery_quality_label()
            .starts_with("excellent discovery match")
    );
}

#[test]
fn streamurl_search_url_uses_station_query() {
    assert_eq!(
        streamurl_search_url("BBC Radio 1"),
        "https://streamurl.link/s/BBC%20Radio%201/"
    );
}

#[test]
fn station_from_direct_url_builds_playable_station() {
    let station =
        station_from_direct_url("Example FM", "https://streams.example.org/live.m3u8", "")
            .expect("direct stream should be accepted");

    assert_eq!(station.name, "Example FM");
    assert_eq!(
        station.url_resolved,
        "https://streams.example.org/live.m3u8"
    );
    assert_eq!(station.source, "Direct stream URL");
}

#[test]
fn station_from_direct_url_rejects_plain_search_text() {
    assert!(station_from_direct_url("Example FM", "Example FM", "").is_err());
}

#[test]
fn discovery_sorted_prefers_quality_and_popularity_signals() {
    let weak = Station::from_api(serde_json::json!({
        "stationuuid": "weak",
        "name": "Weak",
        "url": "https://weak.test",
        "lastcheckok": 1,
        "bitrate": 64,
        "codec": "MP3",
        "clicktrend": 0
    }));
    let strong = Station::from_api(serde_json::json!({
        "stationuuid": "strong",
        "name": "Strong",
        "url": "https://strong.test",
        "lastcheckok": 1,
        "bitrate": 192,
        "codec": "AAC",
        "clicktrend": 5
    }));

    assert!(discovery_sorted(Vec::new()).is_empty());
    let sorted_names: Vec<String> = discovery_sorted(vec![weak, strong])
        .into_iter()
        .map(|station| station.name)
        .collect();
    assert_eq!(sorted_names, vec!["Strong", "Weak"]);
}

#[test]
fn search_request_sends_filters_and_hides_broken_stations() {
    let request = search_request("jazz", "US", "english", "news", 25);

    assert_eq!(request.path, "/json/stations/search");
    assert_eq!(
        request.params.get("hidebroken").map(String::as_str),
        Some("true")
    );
    assert_eq!(request.params.get("limit").map(String::as_str), Some("25"));
    assert_eq!(
        request.params.get("order").map(String::as_str),
        Some("clicktrend")
    );
    assert_eq!(
        request.params.get("reverse").map(String::as_str),
        Some("true")
    );
    assert_eq!(request.params.get("name").map(String::as_str), Some("jazz"));
    assert_eq!(
        request.params.get("countrycode").map(String::as_str),
        Some("US")
    );
    assert_eq!(
        request.params.get("language").map(String::as_str),
        Some("english")
    );
    assert_eq!(request.params.get("tag").map(String::as_str), Some("news"));
    assert!(!request.params.contains_key("country"));
}

#[test]
fn search_request_treats_non_code_country_as_country_name() {
    let request = search_request("", "United States", "", "", 250);

    assert_eq!(
        request.params.get("country").map(String::as_str),
        Some("United States")
    );
    assert_eq!(request.params.get("limit").map(String::as_str), Some("250"));
}

#[test]
fn search_request_caps_limit_at_max_station_results() {
    let request = search_request("", "", "", "", 5000);

    assert_eq!(request.params.get("limit").map(String::as_str), Some("500"));
    assert_eq!(STATION_RESULT_LIMITS.last(), Some(&MAX_STATION_RESULTS));
    assert_eq!(
        STATION_RESULT_LIMITS[DEFAULT_STATION_RESULT_LIMIT_INDEX],
        100
    );
}

fn facet(kind: BrowseFacetKind, name: &str, count: i32, code: &str) -> BrowseFacet {
    BrowseFacet {
        kind,
        name: name.to_string(),
        station_count: count,
        code: code.to_string(),
    }
}

#[test]
fn filter_dropdown_choices_sorts_by_name_and_drops_duplicates() {
    let choices = filter_dropdown_choices(vec![
        facet(BrowseFacetKind::Location, "Germany", 3000, "de"),
        facet(BrowseFacetKind::Location, "austria", 400, "AT"),
        facet(BrowseFacetKind::Location, "Germany", 5, ""),
        facet(BrowseFacetKind::Location, "#", 1, ""),
    ]);

    let names: Vec<_> = choices.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["austria", "Germany"]);
    assert_eq!(choices[1].code, "DE");
}

#[test]
fn resolve_filter_value_maps_country_names_to_codes() {
    let countries = vec![facet(BrowseFacetKind::Location, "Germany", 3000, "DE")];

    assert_eq!(resolve_filter_value("germany", &countries), "DE");
    assert_eq!(resolve_filter_value(" Czechia ", &countries), "Czechia");
    assert_eq!(resolve_filter_value("", &countries), "");
}

#[test]
fn language_choices_display_capitalized_but_search_original_name() {
    let languages = vec![facet(BrowseFacetKind::Language, "czech", 300, "cs")];

    assert_eq!(filter_choice_label(&languages[0]), "Czech");
    assert_eq!(resolve_filter_value("Czech", &languages), "czech");
    assert_eq!(
        filter_choice_label(&facet(BrowseFacetKind::Location, "Czechia", 1, "CZ")),
        "Czechia"
    );
}

#[test]
fn browse_facet_request_maps_kind_to_endpoint_and_limit() {
    let request = browse_facet_request(BrowseEndpoint::Genre, 700);

    assert_eq!(request.path, "/json/tags");
    assert_eq!(
        request.params.get("hidebroken").map(String::as_str),
        Some("true")
    );
    assert_eq!(request.params.get("limit").map(String::as_str), Some("500"));
    assert_eq!(
        request.params.get("order").map(String::as_str),
        Some("stationcount")
    );
}

#[test]
fn clean_browse_facet_rejects_noise_and_preserves_country_codes() {
    assert_eq!(
        clean_browse_facet(BrowseFacet::from_api(
            BrowseFacetKind::Genre,
            serde_json::json!({"name": "\"classic rock\"", "stationcount": "80"})
        )),
        Some(BrowseFacet {
            kind: BrowseFacetKind::Genre,
            name: "classic rock".to_string(),
            station_count: 80,
            code: String::new(),
        })
    );
    assert_eq!(
        clean_browse_facet(BrowseFacet::from_api(
            BrowseFacetKind::Location,
            serde_json::json!({"name": "United States", "iso_3166_1": "us", "stationcount": "200"})
        ))
        .unwrap()
        .code,
        "US"
    );
    assert!(
        clean_browse_facet(BrowseFacet::from_api(
            BrowseFacetKind::Genre,
            serde_json::json!({"name": "#english", "stationcount": 15})
        ))
        .is_none()
    );
}

#[test]
fn radio_browser_client_searches_http_api_and_maps_stations() {
    let server = one_request_server(
        r#"[{"stationuuid":"abc","name":"Example FM","url_resolved":"https://stream.example/live","bitrate":128,"lastcheckok":1}]"#,
    );
    let client = RadioBrowserClient::new([server.url.clone()]);

    let stations = client
        .search("example", "US", "", "jazz", 5)
        .expect("search should fetch stations");

    let request = server.request.join().expect("server thread should finish");
    assert!(request.starts_with("GET /json/stations/search?"));
    assert!(request.contains("name=example"));
    assert!(request.contains("countrycode=US"));
    assert!(request.contains("tag=jazz"));
    assert_eq!(stations.len(), 1);
    assert_eq!(stations[0].stationuuid, "abc");
    assert_eq!(stations[0].name, "Example FM");
}

#[test]
fn radio_browser_client_click_url_requires_ok_payload() {
    let server = one_request_server(r#"{"ok":false,"url":"https://stream.example/live"}"#);
    let client = RadioBrowserClient::new([server.url.clone()]);

    let error = client
        .click_url("abc")
        .expect_err("not-ok click response should fail");

    let request = server.request.join().expect("server thread should finish");
    assert!(request.starts_with("GET /json/url/abc "));
    assert!(error.to_string().contains("playable URL"));
}

struct TestServer {
    url: String,
    request: thread::JoinHandle<String>,
}

fn one_request_server(json: &'static str) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let request = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test server should accept");
        let request = read_request(&mut stream);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            json.len(),
            json
        );
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
