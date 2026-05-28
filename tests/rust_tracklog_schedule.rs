use chrono::{TimeZone, Utc};
use station_scout::models::{Station, TuneTimer};
use station_scout::schedule::due_timers;
use station_scout::tracklog::{
    TrackEntry, TrackSessionLog, extract_stream_title, parse_stream_title,
};
use std::collections::HashSet;

#[test]
fn parse_artist_title_stream_title() {
    let entry = parse_stream_title(
        "Kate Bush - Running Up That Hill",
        Some(Utc.with_ymd_and_hms(2026, 5, 9, 17, 0, 0).unwrap()),
    )
    .expect("metadata should parse");

    assert_eq!(entry.artist, "Kate Bush");
    assert_eq!(entry.title, "Running Up That Hill");
    assert_eq!(entry.display_line(), "Kate Bush - Running Up That Hill");
}

#[test]
fn parse_provider_metadata_blob() {
    let entry = parse_stream_title(
        r#"Justin Bieber - text="Yukon" song_spot="M" MediaBaseId="3142175""#,
        None,
    )
    .expect("metadata should parse");

    assert_eq!(entry.artist, "Justin Bieber");
    assert_eq!(entry.title, "Yukon");
    assert_eq!(entry.display_line(), "Justin Bieber - Yukon");
}

#[test]
fn parse_marks_non_music_provider_metadata_uncertain() {
    let entry = parse_stream_title(
        r#"Philly's #1 Hit Music Station - text="Q102" song_spot="T" MediaBaseId="0""#,
        None,
    )
    .expect("metadata should be preserved");

    assert!(entry.uncertain);
    assert_eq!(
        entry.display_line(),
        "Philly's #1 Hit Music Station - Q102 ?"
    );
}

#[test]
fn bare_station_identifier_is_ignored() {
    assert!(parse_stream_title("PST", None).is_none());
    assert!(parse_stream_title("T", None).is_none());
}

#[test]
fn extract_stream_title_keeps_apostrophes_inside_metadata() {
    let metadata = r#"StreamTitle='Philly's #1 Hit Music Station - text="Q102" song_spot="T" MediaBaseId="0"';"#;

    assert_eq!(
        extract_stream_title(metadata),
        "Philly's #1 Hit Music Station - text=\"Q102\" song_spot=\"T\" MediaBaseId=\"0\""
    );
}

#[test]
fn show_session_file_omits_station_name() {
    let dir = tempfile::tempdir().unwrap();
    let station = Station::new("abc", "Transmission FM", "https://example.test");
    let mut session = TrackSessionLog::new(
        dir.path(),
        station,
        "Gina's Organized Chaos",
        Utc.with_ymd_and_hms(2026, 5, 13, 17, 0, 0).unwrap(),
    );

    session
        .add(
            TrackEntry::new("Kate Bush", "Running Up That Hill", "raw"),
            None,
        )
        .unwrap();

    assert_eq!(
        session.path.file_name().unwrap().to_string_lossy(),
        "gina-s-organized-chaos-2026-05-13-1700.txt"
    );
    assert_eq!(
        std::fs::read_to_string(&session.path).unwrap(),
        "Kate Bush - Running Up That Hill\n"
    );
}

#[test]
fn session_records_station_change_without_losing_tracks() {
    let dir = tempfile::tempdir().unwrap();
    let first = Station::new("abc", "Transmission FM", "https://one.test");
    let second = Station::new("def", "Evening Radio", "https://two.test");
    let mut session = TrackSessionLog::new(dir.path(), first, "", Utc::now());

    session
        .add(
            TrackEntry::new("Kate Bush", "Running Up That Hill", "raw"),
            None,
        )
        .unwrap();
    session
        .add(
            TrackEntry::new("Zara Larsson", "Midnight Sun", "raw"),
            Some(second),
        )
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(&session.path).unwrap(),
        "Kate Bush - Running Up That Hill\n\n== Station: Evening Radio ==\nZara Larsson - Midnight Sun\n"
    );
}

#[test]
fn due_timers_returns_enabled_timers_once_per_day() {
    let timers = vec![
        TuneTimer::new("abc", "Example", "07:30"),
        TuneTimer {
            enabled: false,
            ..TuneTimer::new("def", "Disabled", "07:30")
        },
        TuneTimer::new("ghi", "Later", "08:00"),
    ];
    let mut fired = HashSet::from([(
        "2026-05-09".to_string(),
        "already".to_string(),
        "07:30".to_string(),
    )]);

    let due = due_timers(
        &timers,
        Utc.with_ymd_and_hms(2026, 5, 9, 7, 30, 0).unwrap(),
        &mut fired,
    );

    assert_eq!(due, vec![timers[0].clone()]);
    assert!(fired.contains(&(
        "2026-05-09".to_string(),
        "abc".to_string(),
        "07:30".to_string()
    )));
}
