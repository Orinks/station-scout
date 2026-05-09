from __future__ import annotations

from dataclasses import dataclass
from urllib.parse import urlparse
from uuid import UUID

from station_scout.models import Station


class CastError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class CastDevice:
    name: str
    uuid: str


def station_content_type(station: Station, url: str) -> str:
    codec = station.codec.strip().lower()
    parsed = urlparse(url)
    path = parsed.path.lower()
    if station.hls or path.endswith((".m3u8", ".m3u")):
        return "application/x-mpegURL"
    if codec in {"mp3", "mpeg"} or path.endswith(".mp3"):
        return "audio/mpeg"
    if codec in {"aac", "aac+", "he-aac"} or path.endswith((".aac", ".m4a")):
        return "audio/aac"
    if codec == "ogg" or path.endswith(".ogg"):
        return "audio/ogg"
    if codec == "opus" or path.endswith(".opus"):
        return "audio/opus"
    if codec == "flac" or path.endswith(".flac"):
        return "audio/flac"
    return "audio/mpeg"


def discover_chromecasts(timeout: int = 8) -> list[CastDevice]:
    try:
        import pychromecast
    except ImportError as exc:
        raise CastError("Chromecast support requires pychromecast to be installed.") from exc

    chromecasts, browser = pychromecast.get_chromecasts(timeout=timeout)
    try:
        devices = [
            CastDevice(name=cast.cast_info.friendly_name, uuid=str(cast.cast_info.uuid))
            for cast in chromecasts
        ]
    finally:
        pychromecast.discovery.stop_discovery(browser)
    return sorted(devices, key=lambda device: device.name.casefold())


def play_on_chromecast(device: CastDevice, station: Station, url: str) -> None:
    cast, browser, pychromecast = _find_chromecast(device)
    try:
        controller = cast.media_controller
        controller.play_media(
            url,
            station_content_type(station, url),
            title=station.name,
            stream_type="LIVE",
        )
        controller.block_until_active()
    finally:
        pychromecast.discovery.stop_discovery(browser)


def stop_chromecast(device: CastDevice) -> None:
    cast, browser, pychromecast = _find_chromecast(device)
    try:
        cast.media_controller.stop()
    finally:
        pychromecast.discovery.stop_discovery(browser)


def toggle_chromecast_pause(device: CastDevice) -> bool:
    cast, browser, pychromecast = _find_chromecast(device)
    try:
        controller = cast.media_controller
        state = str(getattr(controller.status, "player_state", "")).upper()
        if state == "PLAYING":
            controller.pause()
            return True
        controller.play()
        return False
    finally:
        pychromecast.discovery.stop_discovery(browser)


def change_chromecast_volume(device: CastDevice, delta: float) -> float:
    cast, browser, pychromecast = _find_chromecast(device)
    try:
        current = float(getattr(cast.status, "volume_level", 0.5) or 0.0)
        level = max(0.0, min(1.0, current + delta))
        cast.set_volume(level)
        return level
    finally:
        pychromecast.discovery.stop_discovery(browser)


def discover_sonos(timeout: int = 8) -> list[CastDevice]:
    try:
        import soco
    except ImportError as exc:
        raise CastError("Sonos support requires soco to be installed.") from exc

    speakers = soco.discover(timeout=timeout) or set()
    devices = [
        CastDevice(name=str(speaker.player_name), uuid=str(speaker.uid))
        for speaker in speakers
        if getattr(speaker, "player_name", "") and getattr(speaker, "uid", "")
    ]
    return sorted(devices, key=lambda device: device.name.casefold())


def play_on_sonos(device: CastDevice, station: Station, url: str) -> None:
    speaker = _find_sonos(device)
    speaker.play_uri(url, title=station.name, force_radio=True)


def stop_sonos(device: CastDevice) -> None:
    speaker = _find_sonos(device)
    speaker.stop()


def toggle_sonos_pause(device: CastDevice) -> bool:
    speaker = _find_sonos(device)
    state = str(speaker.get_current_transport_info().get("current_transport_state", "")).upper()
    if state == "PLAYING":
        speaker.pause()
        return True
    speaker.play()
    return False


def change_sonos_volume(device: CastDevice, delta: float) -> float:
    speaker = _find_sonos(device)
    current = int(getattr(speaker, "volume", 50) or 0)
    volume = max(0, min(100, current + round(delta * 100)))
    speaker.volume = volume
    return volume / 100


def _find_chromecast(device: CastDevice):
    try:
        import pychromecast
    except ImportError as exc:
        raise CastError("Chromecast support requires pychromecast to be installed.") from exc

    chromecasts, browser = pychromecast.get_listed_chromecasts(uuids=[UUID(device.uuid)])
    if not chromecasts:
        pychromecast.discovery.stop_discovery(browser)
        raise CastError(f"Could not find Chromecast: {device.name}")
    cast = chromecasts[0]
    cast.wait()
    return cast, browser, pychromecast


def _find_sonos(device: CastDevice):
    try:
        import soco
    except ImportError as exc:
        raise CastError("Sonos support requires soco to be installed.") from exc

    speakers = soco.discover(timeout=8) or set()
    for speaker in speakers:
        if str(getattr(speaker, "uid", "")) == device.uuid:
            return speaker
    raise CastError(f"Could not find Sonos speaker: {device.name}")
