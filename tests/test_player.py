import sys
from types import ModuleType
from unittest.mock import MagicMock, patch

from station_scout import player as player_module
from station_scout.player import RadioPlayer


def test_play_creates_url_stream() -> None:
    stream = MagicMock()
    stream.is_playing = True
    stream_module = ModuleType("sound_lib.stream")
    stream_module.URLStream = MagicMock(return_value=stream)  # type: ignore[attr-defined]

    with (
        patch.object(player_module, "_sound_lib_available", True),
        patch.object(player_module, "_sound_lib_initialized", True),
        patch.dict(sys.modules, {"sound_lib.stream": stream_module}),
    ):
        on_playing = MagicMock()
        radio = RadioPlayer(on_playing=on_playing)

        assert radio.play("https://example.test/live") is True

    stream_module.URLStream.assert_called_once_with(  # type: ignore[attr-defined]
        url="https://example.test/live",
        unicode=True,
    )
    stream.play.assert_called_once()
    on_playing.assert_called_once()


def test_ensure_sound_lib_initializes_output() -> None:
    output_instance = MagicMock()
    output_module = ModuleType("sound_lib.output")
    output_module.Output = MagicMock(return_value=output_instance)  # type: ignore[attr-defined]
    stream_module = ModuleType("sound_lib.stream")
    stream_module.URLStream = MagicMock()  # type: ignore[attr-defined]

    with (
        patch.object(player_module, "_sound_lib_available", False),
        patch.object(player_module, "_sound_lib_initialized", False),
        patch.object(player_module, "_sound_lib_output", None),
        patch.dict(sys.modules, {"sound_lib.output": output_module, "sound_lib.stream": stream_module}),
    ):
        assert player_module._ensure_sound_lib() is True
        assert player_module._sound_lib_output is output_instance

    output_module.Output.assert_called_once_with()  # type: ignore[attr-defined]


def test_play_reports_missing_sound_lib() -> None:
    on_error = MagicMock()
    radio = RadioPlayer(on_error=on_error)

    with (
        patch.object(player_module, "_sound_lib_available", False),
        patch.object(player_module, "_sound_lib_initialized", True),
    ):
        assert radio.play("https://example.test/live") is False

    assert "sound_lib" in on_error.call_args[0][0]


def test_stop_frees_active_stream() -> None:
    stream = MagicMock()
    stream.is_playing = True
    radio = RadioPlayer()
    radio._stream = stream

    radio.stop()

    stream.stop.assert_called_once()
    stream.free.assert_called_once()
    assert radio._stream is None
