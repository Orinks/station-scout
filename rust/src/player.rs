#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

pub trait RadioPlaybackBackend {
    fn play(&mut self, url: &str, volume: f64) -> Result<(), String>;
    fn pause(&mut self) -> Result<bool, String>;
    fn resume(&mut self) -> Result<bool, String>;
    fn stop(&mut self);
}

pub trait MediaControl {
    fn load_uri(&mut self, uri: &str) -> bool;
    fn set_volume(&mut self, volume: f64) -> bool;
    fn play(&mut self) -> bool;
    fn pause(&mut self) -> bool;
    fn stop(&mut self) -> bool;
}

pub struct MediaControlRadioBackend<T> {
    control: T,
}

impl<T> MediaControlRadioBackend<T>
where
    T: MediaControl,
{
    pub fn new(control: T) -> Self {
        Self { control }
    }
}

impl<T> RadioPlaybackBackend for MediaControlRadioBackend<T>
where
    T: MediaControl,
{
    fn play(&mut self, url: &str, volume: f64) -> Result<(), String> {
        if !self.control.load_uri(url) {
            return Err(format!("Failed to load radio stream: {url}"));
        }
        self.control.set_volume(volume);
        if !self.control.play() {
            return Err("Failed to start radio stream playback.".to_string());
        }
        Ok(())
    }

    fn pause(&mut self) -> Result<bool, String> {
        Ok(self.control.pause())
    }

    fn resume(&mut self) -> Result<bool, String> {
        Ok(self.control.play())
    }

    fn stop(&mut self) {
        self.control.stop();
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
