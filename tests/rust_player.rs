use station_scout::models::Station;
use station_scout::player::{
    GStreamerRadioBackend, PlaybackState, RadioPlaybackBackend, RadioPlayer, RadioPlayerCallbacks,
    UriPlaybackControl, play_podcast_episode_stream, play_station_stream,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn player_reports_missing_backend_when_play_requested_without_audio_driver() {
    let mut player = RadioPlayer::without_backend();

    assert!(!player.play("https://example.test/live"));
    assert_eq!(player.state(), PlaybackState::Stopped);
    assert!(
        player
            .last_error()
            .unwrap()
            .contains("audio playback backend")
    );
}

#[test]
fn player_with_available_backend_tracks_play_pause_resume_and_stop() {
    let mut player = RadioPlayer::with_backend_available();

    assert!(player.play("https://example.test/live"));
    assert_eq!(player.state(), PlaybackState::Playing);
    assert!(player.pause());
    assert_eq!(player.state(), PlaybackState::Paused);
    assert!(player.resume());
    assert_eq!(player.state(), PlaybackState::Playing);
    player.stop();
    assert_eq!(player.state(), PlaybackState::Stopped);
}

#[test]
fn player_volume_clamps_to_range() {
    let mut player = RadioPlayer::with_backend_available();

    assert_eq!(player.change_volume(-2.0), 0.0);
    assert_eq!(player.change_volume(1.5), 1.0);
    assert_eq!(player.get_volume(), 1.0);
}

#[test]
fn player_volume_changes_reach_active_backend() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut player = RadioPlayer::with_backend(Box::new(VolumeRecordingBackend {
        events: events.clone(),
    }));

    assert!(player.play("https://example.test/live"));
    assert_eq!(player.change_volume(-0.2), 0.8);
    assert!((player.change_volume(0.05) - 0.85).abs() < 0.000_001);

    assert_eq!(
        events.borrow().as_slice(),
        [
            "play:https://example.test/live:1.00",
            "set_volume:0.80",
            "set_volume:0.85",
        ]
    );
}

#[test]
fn player_delegates_stream_lifecycle_to_backend_and_callbacks() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let playing_count = Rc::new(Cell::new(0));
    let stopped_count = Rc::new(Cell::new(0));
    let errors = Rc::new(RefCell::new(Vec::new()));
    let mut player = RadioPlayer::with_backend_and_callbacks(
        Box::new(RecordingBackend {
            events: events.clone(),
            fail_play: false,
        }),
        RadioPlayerCallbacks {
            on_playing: Some(Box::new({
                let playing_count = playing_count.clone();
                move || playing_count.set(playing_count.get() + 1)
            })),
            on_stopped: Some(Box::new({
                let stopped_count = stopped_count.clone();
                move || stopped_count.set(stopped_count.get() + 1)
            })),
            on_error: Some(Box::new({
                let errors = errors.clone();
                move |message| errors.borrow_mut().push(message.to_string())
            })),
        },
    );

    player.set_volume(0.35);

    assert!(player.play("https://example.test/one"));
    assert!(player.play("https://example.test/two"));
    assert!(player.pause());
    assert!(player.resume());
    player.stop();

    assert_eq!(
        events.borrow().as_slice(),
        [
            "play:https://example.test/one:0.35",
            "stop",
            "play:https://example.test/two:0.35",
            "pause",
            "resume",
            "stop",
        ]
    );
    assert_eq!(playing_count.get(), 2);
    assert_eq!(stopped_count.get(), 1);
    assert!(errors.borrow().is_empty());
}

#[test]
fn gstreamer_backend_routes_any_uri_to_one_shared_player_control() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut backend = GStreamerRadioBackend::new(RecordingUriControl {
        events: events.clone(),
        fail_play: false,
    });

    backend
        .play("https://stream.example.test/live", 0.6)
        .expect("stream uri should be accepted");
    assert!(backend.pause().unwrap());
    assert!(backend.resume().unwrap());
    backend.stop();

    assert_eq!(
        events.borrow().as_slice(),
        [
            "set_uri:https://stream.example.test/live",
            "set_volume:0.60",
            "play",
            "pause",
            "play",
            "stop",
        ]
    );
}

#[test]
fn station_stream_playback_uses_radio_player_backend() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut player = RadioPlayer::with_backend(Box::new(RecordingBackend {
        events: events.clone(),
        fail_play: false,
    }));
    player.set_volume(0.75);
    let station = Station::new(
        "station-1",
        "Example FM",
        "https://stream.example.test/live",
    );

    let message = play_station_stream(&mut player, &station).expect("station should play");

    assert_eq!(message, "Playing Example FM.");
    assert_eq!(player.state(), PlaybackState::Playing);
    assert_eq!(
        events.borrow().as_slice(),
        ["play:https://stream.example.test/live:0.75"]
    );
}

#[test]
fn station_stream_playback_rejects_stations_without_stream_urls() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut player = RadioPlayer::with_backend(Box::new(RecordingBackend {
        events: events.clone(),
        fail_play: false,
    }));
    let station = Station::new("station-1", "Example FM", "");

    let error = play_station_stream(&mut player, &station).unwrap_err();

    assert_eq!(error, "Selected station has no playable stream URL.");
    assert_eq!(player.state(), PlaybackState::Stopped);
    assert!(events.borrow().is_empty());
}

#[test]
fn podcast_episode_playback_uses_the_same_radio_player_backend() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut player = RadioPlayer::with_backend(Box::new(RecordingBackend {
        events: events.clone(),
        fail_play: false,
    }));

    let message = play_podcast_episode_stream(
        &mut player,
        Some("Episode One"),
        Some("https://podcast.example.test/episode.mp3"),
    )
    .expect("episode should play");

    assert_eq!(message, "Playing Episode One.");
    assert_eq!(player.state(), PlaybackState::Playing);
    assert_eq!(
        events.borrow().as_slice(),
        ["play:https://podcast.example.test/episode.mp3:1.00"]
    );
}

#[test]
fn podcast_episode_playback_rejects_episodes_without_enclosures() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut player = RadioPlayer::with_backend(Box::new(RecordingBackend {
        events: events.clone(),
        fail_play: false,
    }));

    let error = play_podcast_episode_stream(&mut player, Some("Episode One"), None).unwrap_err();

    assert_eq!(error, "Selected episode has no playable enclosure URL.");
    assert_eq!(player.state(), PlaybackState::Stopped);
    assert!(events.borrow().is_empty());
}

struct RecordingBackend {
    events: Rc<RefCell<Vec<String>>>,
    fail_play: bool,
}

struct VolumeRecordingBackend {
    events: Rc<RefCell<Vec<String>>>,
}

struct RecordingUriControl {
    events: Rc<RefCell<Vec<String>>>,
    fail_play: bool,
}

impl UriPlaybackControl for RecordingUriControl {
    fn set_uri(&mut self, uri: &str) -> Result<(), String> {
        self.events.borrow_mut().push(format!("set_uri:{uri}"));
        Ok(())
    }

    fn set_volume(&mut self, volume: f64) -> Result<(), String> {
        self.events
            .borrow_mut()
            .push(format!("set_volume:{volume:.2}"));
        Ok(())
    }

    fn play(&mut self) -> Result<(), String> {
        self.events.borrow_mut().push("play".to_string());
        if self.fail_play {
            Err("play failed".to_string())
        } else {
            Ok(())
        }
    }

    fn pause(&mut self) -> Result<(), String> {
        self.events.borrow_mut().push("pause".to_string());
        Ok(())
    }

    fn stop(&mut self) {
        self.events.borrow_mut().push("stop".to_string());
    }
}

impl RadioPlaybackBackend for RecordingBackend {
    fn play(&mut self, url: &str, volume: f64) -> Result<(), String> {
        if self.fail_play {
            return Err("backend refused stream".to_string());
        }
        self.events
            .borrow_mut()
            .push(format!("play:{url}:{volume:.2}"));
        Ok(())
    }

    fn pause(&mut self) -> Result<bool, String> {
        self.events.borrow_mut().push("pause".to_string());
        Ok(true)
    }

    fn resume(&mut self) -> Result<bool, String> {
        self.events.borrow_mut().push("resume".to_string());
        Ok(true)
    }

    fn stop(&mut self) {
        self.events.borrow_mut().push("stop".to_string());
    }
}

impl RadioPlaybackBackend for VolumeRecordingBackend {
    fn play(&mut self, url: &str, volume: f64) -> Result<(), String> {
        self.events
            .borrow_mut()
            .push(format!("play:{url}:{volume:.2}"));
        Ok(())
    }

    fn set_volume(&mut self, volume: f64) -> Result<(), String> {
        self.events
            .borrow_mut()
            .push(format!("set_volume:{volume:.2}"));
        Ok(())
    }

    fn pause(&mut self) -> Result<bool, String> {
        Ok(true)
    }

    fn resume(&mut self) -> Result<bool, String> {
        Ok(true)
    }

    fn stop(&mut self) {}
}
