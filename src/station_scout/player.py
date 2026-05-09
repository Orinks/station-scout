from __future__ import annotations

from collections.abc import Callable
import logging

logger = logging.getLogger(__name__)

_sound_lib_initialized = False
_sound_lib_available = False
_sound_lib_output = None


def _ensure_sound_lib() -> bool:
    global _sound_lib_available, _sound_lib_initialized, _sound_lib_output
    if _sound_lib_initialized:
        return _sound_lib_available
    _sound_lib_initialized = True
    try:
        from sound_lib import output
        from sound_lib.stream import URLStream  # noqa: F401

        _sound_lib_output = output.Output()
        _sound_lib_available = True
    except ImportError:
        logger.warning("sound_lib is not installed")
    except Exception as exc:
        logger.warning("sound_lib check failed: %s", exc)
    return _sound_lib_available


class RadioPlayer:
    def __init__(
        self,
        *,
        on_playing: Callable[[], None] | None = None,
        on_stopped: Callable[[], None] | None = None,
        on_error: Callable[[str], None] | None = None,
    ) -> None:
        self._stream = None
        self._volume = 1.0
        self._on_playing = on_playing
        self._on_stopped = on_stopped
        self._on_error = on_error

    def play(self, url: str) -> bool:
        if not _ensure_sound_lib():
            return self._fail("Audio playback is not available. Please ensure sound_lib is installed.")
        self.stop(notify=False)
        try:
            from sound_lib.stream import URLStream

            self._stream = URLStream(url=url, unicode=True)
            self._stream.volume = self._volume
            self._stream.play()
        except Exception as exc:
            self._stream = None
            return self._fail(f"Failed to start stream: {exc}")
        if self._on_playing:
            self._on_playing()
        return True

    def stop(self, *, notify: bool = True) -> None:
        was_playing = self.is_playing()
        if self._stream is not None:
            try:
                self._stream.stop()
                self._stream.free()
            except Exception as exc:
                logger.debug("Error stopping stream: %s", exc)
            finally:
                self._stream = None
        if notify and was_playing and self._on_stopped:
            self._on_stopped()

    def set_volume(self, level: float) -> None:
        self._volume = max(0.0, min(1.0, level))
        if self._stream is not None:
            try:
                self._stream.volume = self._volume
            except Exception as exc:
                logger.debug("Error setting stream volume: %s", exc)

    def get_volume(self) -> float:
        return self._volume

    def is_playing(self) -> bool:
        if self._stream is None:
            return False
        try:
            return bool(self._stream.is_playing)
        except Exception:
            return False

    def _fail(self, message: str) -> bool:
        logger.error(message)
        if self._on_error:
            self._on_error(message)
        return False
