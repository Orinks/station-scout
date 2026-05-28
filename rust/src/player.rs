use gstreamer::prelude::{ElementExt, ObjectExt};

use crate::models::Station;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

pub trait RadioPlaybackBackend {
    fn play(&mut self, url: &str, volume: f64) -> Result<(), String>;
    fn set_volume(&mut self, _volume: f64) -> Result<(), String> {
        Ok(())
    }
    fn pause(&mut self) -> Result<bool, String>;
    fn resume(&mut self) -> Result<bool, String>;
    fn stop(&mut self);
}

pub trait UriPlaybackControl {
    fn set_uri(&mut self, uri: &str) -> Result<(), String>;
    fn set_volume(&mut self, volume: f64) -> Result<(), String>;
    fn play(&mut self) -> Result<(), String>;
    fn pause(&mut self) -> Result<(), String>;
    fn stop(&mut self);
}

pub fn play_station_stream(player: &mut RadioPlayer, station: &Station) -> Result<String, String> {
    play_named_stream(
        player,
        &station.name,
        Some(&station.url_resolved),
        "Selected station has no playable stream URL.",
    )
}

pub fn play_podcast_episode_stream(
    player: &mut RadioPlayer,
    title: Option<&str>,
    enclosure_url: Option<&str>,
) -> Result<String, String> {
    play_named_stream(
        player,
        title.unwrap_or("selected episode"),
        enclosure_url,
        "Selected episode has no playable enclosure URL.",
    )
}

fn play_named_stream(
    player: &mut RadioPlayer,
    name: &str,
    url: Option<&str>,
    empty_url_message: &str,
) -> Result<String, String> {
    let url = url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| empty_url_message.to_string())?;
    if player.play(url) {
        Ok(format!("Playing {}.", display_stream_name(name)))
    } else {
        Err(player
            .last_error()
            .unwrap_or("Failed to start playback.")
            .to_string())
    }
}

fn display_stream_name(name: &str) -> String {
    let normalized = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        "selected episode".to_string()
    } else {
        normalized
    }
}

pub struct GStreamerRadioBackend<T> {
    control: T,
}

impl<T> GStreamerRadioBackend<T>
where
    T: UriPlaybackControl,
{
    pub fn new(control: T) -> Self {
        Self { control }
    }
}

impl<T> RadioPlaybackBackend for GStreamerRadioBackend<T>
where
    T: UriPlaybackControl,
{
    fn play(&mut self, url: &str, volume: f64) -> Result<(), String> {
        self.control.set_uri(url)?;
        self.control.set_volume(volume)?;
        self.control.play()
    }

    fn set_volume(&mut self, volume: f64) -> Result<(), String> {
        self.control.set_volume(volume)
    }

    fn pause(&mut self) -> Result<bool, String> {
        self.control.pause().map(|()| true)
    }

    fn resume(&mut self) -> Result<bool, String> {
        self.control.play().map(|()| true)
    }

    fn stop(&mut self) {
        self.control.stop();
    }
}

pub fn gstreamer_radio_player() -> Result<RadioPlayer, String> {
    Ok(RadioPlayer::with_backend(Box::new(
        GStreamerRadioBackend::new(GStreamerUriPlaybackControl::new()?),
    )))
}

struct GStreamerUriPlaybackControl {
    player: gstreamer::Element,
}

impl GStreamerUriPlaybackControl {
    fn new() -> Result<Self, String> {
        gstreamer::init().map_err(|error| format!("Failed to initialize GStreamer: {error}"))?;
        let player = gstreamer::ElementFactory::make("playbin")
            .build()
            .map_err(|error| format!("Failed to create GStreamer playbin: {error}"))?;
        Ok(Self { player })
    }
}

impl UriPlaybackControl for GStreamerUriPlaybackControl {
    fn set_uri(&mut self, uri: &str) -> Result<(), String> {
        self.player.set_property("uri", uri);
        Ok(())
    }

    fn set_volume(&mut self, volume: f64) -> Result<(), String> {
        self.player.set_property("volume", volume.clamp(0.0, 1.0));
        Ok(())
    }

    fn play(&mut self) -> Result<(), String> {
        self.player
            .set_state(gstreamer::State::Playing)
            .map(|_| ())
            .map_err(|error| format!("Failed to start GStreamer playback: {error}"))
    }

    fn pause(&mut self) -> Result<(), String> {
        self.player
            .set_state(gstreamer::State::Paused)
            .map(|_| ())
            .map_err(|error| format!("Failed to pause GStreamer playback: {error}"))
    }

    fn stop(&mut self) {
        let _ = self.player.set_state(gstreamer::State::Null);
    }
}

#[derive(Default)]
pub struct RadioPlayerCallbacks {
    pub on_playing: Option<Box<dyn FnMut()>>,
    pub on_stopped: Option<Box<dyn FnMut()>>,
    pub on_error: Option<Box<dyn FnMut(&str)>>,
}

pub struct RadioPlayer {
    backend: Box<dyn RadioPlaybackBackend>,
    callbacks: RadioPlayerCallbacks,
    state: PlaybackState,
    volume: f64,
    current_url: String,
    last_error: Option<String>,
}

impl RadioPlayer {
    pub fn without_backend() -> Self {
        Self::with_backend(Box::new(MissingPlaybackBackend))
    }

    pub fn with_backend_available() -> Self {
        Self::with_backend(Box::new(StateOnlyPlaybackBackend))
    }

    pub fn with_backend(backend: Box<dyn RadioPlaybackBackend>) -> Self {
        Self::with_backend_and_callbacks(backend, RadioPlayerCallbacks::default())
    }

    pub fn with_backend_and_callbacks(
        backend: Box<dyn RadioPlaybackBackend>,
        callbacks: RadioPlayerCallbacks,
    ) -> Self {
        Self {
            backend,
            callbacks,
            state: PlaybackState::Stopped,
            volume: 1.0,
            current_url: String::new(),
            last_error: None,
        }
    }

    pub fn play(&mut self, url: &str) -> bool {
        if self.state != PlaybackState::Stopped {
            self.stop_with_notify(false);
        }
        match self.backend.play(url, self.volume) {
            Ok(()) => {
                self.current_url = url.to_string();
                self.state = PlaybackState::Playing;
                self.last_error = None;
                if let Some(callback) = self.callbacks.on_playing.as_mut() {
                    callback();
                }
                true
            }
            Err(error) => {
                self.fail(error);
                false
            }
        }
    }

    pub fn pause(&mut self) -> bool {
        if self.state != PlaybackState::Playing {
            return false;
        }
        match self.backend.pause() {
            Ok(true) => {
                self.state = PlaybackState::Paused;
                true
            }
            Ok(false) => false,
            Err(error) => {
                self.fail(error);
                false
            }
        }
    }

    pub fn resume(&mut self) -> bool {
        if self.state != PlaybackState::Paused {
            return false;
        }
        match self.backend.resume() {
            Ok(true) => {
                self.state = PlaybackState::Playing;
                true
            }
            Ok(false) => false,
            Err(error) => {
                self.fail(error);
                false
            }
        }
    }

    pub fn toggle_pause(&mut self) -> bool {
        match self.state {
            PlaybackState::Playing => self.pause(),
            PlaybackState::Paused => self.resume(),
            PlaybackState::Stopped => false,
        }
    }

    pub fn stop(&mut self) {
        self.stop_with_notify(true);
    }

    fn stop_with_notify(&mut self, notify: bool) {
        let was_playing = self.state == PlaybackState::Playing;
        if self.state != PlaybackState::Stopped {
            self.backend.stop();
        }
        self.state = PlaybackState::Stopped;
        self.current_url.clear();
        if notify && was_playing {
            if let Some(callback) = self.callbacks.on_stopped.as_mut() {
                callback();
            }
        }
    }

    fn fail(&mut self, message: String) {
        self.state = PlaybackState::Stopped;
        self.current_url.clear();
        self.last_error = Some(message);
        if let (Some(callback), Some(error)) =
            (self.callbacks.on_error.as_mut(), self.last_error.as_deref())
        {
            callback(error);
        }
    }

    pub fn set_volume(&mut self, level: f64) {
        self.volume = if level.is_finite() {
            level.clamp(0.0, 1.0)
        } else {
            1.0
        };
        match self.backend.set_volume(self.volume) {
            Ok(()) => {
                self.last_error = None;
            }
            Err(error) => {
                self.last_error = Some(error.clone());
                if let Some(callback) = self.callbacks.on_error.as_mut() {
                    callback(&error);
                }
            }
        }
    }

    pub fn change_volume(&mut self, delta: f64) -> f64 {
        self.set_volume(self.volume + delta);
        self.volume
    }

    pub fn get_volume(&self) -> f64 {
        self.volume
    }

    pub fn state(&self) -> PlaybackState {
        self.state
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

struct MissingPlaybackBackend;

impl RadioPlaybackBackend for MissingPlaybackBackend {
    fn play(&mut self, _url: &str, _volume: f64) -> Result<(), String> {
        Err("audio playback backend is not available. Build with a streaming backend.".to_string())
    }

    fn pause(&mut self) -> Result<bool, String> {
        Ok(false)
    }

    fn resume(&mut self) -> Result<bool, String> {
        Ok(false)
    }

    fn stop(&mut self) {}
}

struct StateOnlyPlaybackBackend;

impl RadioPlaybackBackend for StateOnlyPlaybackBackend {
    fn play(&mut self, _url: &str, _volume: f64) -> Result<(), String> {
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
