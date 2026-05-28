use station_scout::player::{
    MediaControl, MediaControlRadioBackend, PlaybackState, RadioPlaybackBackend, RadioPlayer,
    RadioPlayerCallbacks,
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
fn media_control_backend_loads_internet_radio_uri_before_playing() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut backend = MediaControlRadioBackend::new(RecordingMediaControl {
        events: events.clone(),
        fail_load: false,
        fail_play: false,
    });

    assert!(
        backend
            .play("https://stream.example.test/live", 0.6)
            .is_ok()
    );
    assert_eq!(
        events.borrow().as_slice(),
        [
            "load_uri:https://stream.example.test/live",
            "set_volume:0.60",
            "play",
        ]
    );

    assert!(backend.pause().unwrap());
    assert!(backend.resume().unwrap());
    backend.stop();
    assert_eq!(events.borrow().as_slice()[3..], ["pause", "play", "stop"]);
}

struct RecordingBackend {
    events: Rc<RefCell<Vec<String>>>,
    fail_play: bool,
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

struct RecordingMediaControl {
    events: Rc<RefCell<Vec<String>>>,
    fail_load: bool,
    fail_play: bool,
}

impl MediaControl for RecordingMediaControl {
    fn load_uri(&mut self, uri: &str) -> bool {
        self.events.borrow_mut().push(format!("load_uri:{uri}"));
        !self.fail_load
    }

    fn set_volume(&mut self, volume: f64) -> bool {
        self.events
            .borrow_mut()
            .push(format!("set_volume:{volume:.2}"));
        true
    }

    fn play(&mut self) -> bool {
        self.events.borrow_mut().push("play".to_string());
        !self.fail_play
    }

    fn pause(&mut self) -> bool {
        self.events.borrow_mut().push("pause".to_string());
        true
    }

    fn stop(&mut self) -> bool {
        self.events.borrow_mut().push("stop".to_string());
        true
    }
}
