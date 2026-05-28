use station_scout::models::{Station, TuneTimer};
use station_scout::podcasts::discovery::SearchResult;
use station_scout::podcasts::repository::{
    EpisodeRow, NotificationEntry, QueueEntry, SubscriptionRow,
};
use station_scout::ui_actions::{
    PodcastUiModel, episode_label, inbox_label, podcast_search_result_label, queue_label,
    station_label, subscription_label, timer_label,
};

#[test]
fn podcast_ui_labels_include_state_for_screen_reader_lists() {
    let subscription = SubscriptionRow {
        id: 7,
        feed_url: "https://feeds.example/show.xml".to_string(),
        title: Some("Accessible Show".to_string()),
        author: Some("A. Host".to_string()),
        image_url: None,
        description: None,
        subscribed_at: "2026-05-28T10:00:00Z".to_string(),
        unsubscribed_at: None,
    };
    let episode = EpisodeRow {
        id: 42,
        subscription_id: 7,
        guid: Some("episode-42".to_string()),
        title: Some("Keyboard News".to_string()),
        enclosure_url: Some("https://cdn.example/keyboard.mp3".to_string()),
        pub_date: Some("2026-05-27".to_string()),
        duration_seconds: Some(3723),
        description_html: Some("<p>Shownotes</p>".to_string()),
        transcript_url: Some("https://cdn.example/keyboard.vtt".to_string()),
        transcript_format: Some("text/vtt".to_string()),
        chapters_url: None,
        position_ms: 90_000,
        is_played: true,
        played_at: Some("2026-05-28T11:00:00Z".to_string()),
        is_queued: true,
        queue_position: Some(2),
    };
    let queue = QueueEntry {
        episode_id: 42,
        position: 2,
        title: Some("Keyboard News".to_string()),
        added_at: "2026-05-28T11:30:00Z".to_string(),
    };
    let inbox = NotificationEntry {
        id: 3,
        episode_id: 42,
        episode_title: Some("Keyboard News".to_string()),
        show_title: Some("Accessible Show".to_string()),
        classified_at: "2026-05-28T11:45:00Z".to_string(),
        action: "digest".to_string(),
        toast_fired: true,
        fired_at: Some("2026-05-28T12:00:00Z".to_string()),
        suppression_reason: Some("quiet hours".to_string()),
    };

    assert_eq!(
        subscription_label(&subscription),
        "Accessible Show by A. Host; feed https://feeds.example/show.xml"
    );
    assert_eq!(
        episode_label(&episode),
        "Keyboard News; played; queued position 2; listened 1 minute 30 seconds; duration 1 hour 2 minutes 3 seconds; published 2026-05-27; transcript available"
    );
    assert_eq!(
        queue_label(&queue),
        "Queue position 2: Keyboard News; added 2026-05-28T11:30:00Z"
    );
    assert_eq!(
        inbox_label(&inbox),
        "Keyboard News from Accessible Show; action digest; toast fired; suppressed: quiet hours; classified 2026-05-28T11:45:00Z"
    );

    let model =
        PodcastUiModel::from_rows(vec![subscription], vec![episode], vec![queue], vec![inbox]);
    assert_eq!(
        model.subscriptions[0],
        "Accessible Show by A. Host; feed https://feeds.example/show.xml"
    );
    assert_eq!(
        model.episodes[0],
        "Keyboard News; played; queued position 2; listened 1 minute 30 seconds; duration 1 hour 2 minutes 3 seconds; published 2026-05-27; transcript available"
    );
    assert_eq!(
        model.queue[0],
        "Queue position 2: Keyboard News; added 2026-05-28T11:30:00Z"
    );
    assert_eq!(
        model.inbox[0],
        "Keyboard News from Accessible Show; action digest; toast fired; suppressed: quiet hours; classified 2026-05-28T11:45:00Z"
    );
}

#[test]
fn radio_ui_labels_include_quality_and_timer_state() {
    let mut station = Station::new("abc", "Example FM", "https://stream.example/live");
    station.country = "United States".to_string();
    station.language = "english".to_string();
    station.tags = "news".to_string();
    station.codec = "MP3".to_string();
    station.bitrate = 128;
    station.lastcheckok = 1;
    let mut timer = TuneTimer::new("abc", "Example FM", "07:30");
    timer.show_name = "Morning news".to_string();
    timer.end_time = "08:00".to_string();
    timer.track_playlist = true;

    assert_eq!(
        station_label(&station),
        "Example FM; United States | english | news; working, MP3, 128 kbps"
    );
    assert_eq!(
        timer_label(&timer),
        "Example FM at 07:30; enabled; auto play; stop at 08:00; show Morning news; track playlist"
    );
}

#[test]
fn podcast_search_result_label_names_source_author_and_feed() {
    let result = SearchResult {
        feed_url: "https://feeds.example/rust.xml".to_string(),
        title: "Rust Radio".to_string(),
        author: Some("Station Scout Labs".to_string()),
        description: Some("Systems programming news".to_string()),
        image_url: None,
        source: "itunes".to_string(),
    };

    assert_eq!(
        podcast_search_result_label(&result),
        "Rust Radio by Station Scout Labs; source itunes; feed https://feeds.example/rust.xml"
    );
}
