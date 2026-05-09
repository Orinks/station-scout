import sys
from types import ModuleType
from unittest.mock import MagicMock
from uuid import UUID

from station_scout.cast import (
    CastDevice,
    change_chromecast_volume,
    change_sonos_volume,
    discover_chromecasts,
    discover_sonos,
    play_on_chromecast,
    play_on_sonos,
    station_content_type,
    stop_sonos,
    toggle_chromecast_pause,
    toggle_sonos_pause,
)
from station_scout.models import Station


def test_station_content_type_uses_hls_flag() -> None:
    station = Station(stationuuid="abc", name="Example", url_resolved="https://example.test/live", hls=1)

    assert station_content_type(station, station.url_resolved) == "application/x-mpegURL"


def test_station_content_type_uses_codec() -> None:
    station = Station(
        stationuuid="abc",
        name="Example",
        url_resolved="https://example.test/live",
        codec="AAC",
    )

    assert station_content_type(station, station.url_resolved) == "audio/aac"


def test_station_content_type_defaults_to_mpeg() -> None:
    station = Station(stationuuid="abc", name="Example", url_resolved="https://example.test/live")

    assert station_content_type(station, station.url_resolved) == "audio/mpeg"


def test_discover_chromecasts_returns_sorted_devices(monkeypatch) -> None:
    browser = object()
    first = _cast("Kitchen", "11111111-1111-1111-1111-111111111111")
    second = _cast("Bedroom", "22222222-2222-2222-2222-222222222222")
    pychromecast = _pychromecast_module()
    pychromecast.get_chromecasts = MagicMock(return_value=([first, second], browser))  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "pychromecast", pychromecast)

    assert discover_chromecasts() == [
        CastDevice(name="Bedroom", uuid="22222222-2222-2222-2222-222222222222"),
        CastDevice(name="Kitchen", uuid="11111111-1111-1111-1111-111111111111"),
    ]
    pychromecast.discovery.stop_discovery.assert_called_once_with(browser)  # type: ignore[attr-defined]


def test_play_on_chromecast_loads_live_station(monkeypatch) -> None:
    browser = object()
    cast = _cast("Kitchen", "11111111-1111-1111-1111-111111111111")
    pychromecast = _pychromecast_module()
    pychromecast.get_listed_chromecasts = MagicMock(return_value=([cast], browser))  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "pychromecast", pychromecast)
    station = Station(
        stationuuid="abc",
        name="Example FM",
        url_resolved="https://example.test/live.aac",
        codec="AAC",
    )

    play_on_chromecast(
        CastDevice(name="Kitchen", uuid="11111111-1111-1111-1111-111111111111"),
        station,
        station.url_resolved,
    )

    pychromecast.get_listed_chromecasts.assert_called_once_with(  # type: ignore[attr-defined]
        uuids=[UUID("11111111-1111-1111-1111-111111111111")]
    )
    cast.wait.assert_called_once_with()
    cast.media_controller.play_media.assert_called_once_with(
        "https://example.test/live.aac",
        "audio/aac",
        title="Example FM",
        stream_type="LIVE",
    )
    cast.media_controller.block_until_active.assert_called_once_with()
    pychromecast.discovery.stop_discovery.assert_called_once_with(browser)  # type: ignore[attr-defined]


def test_discover_sonos_returns_sorted_speakers(monkeypatch) -> None:
    living_room = _sonos("Living Room", "RINCON_111")
    office = _sonos("Office", "RINCON_222")
    soco = _soco_module({living_room, office})
    monkeypatch.setitem(sys.modules, "soco", soco)

    assert discover_sonos() == [
        CastDevice(name="Living Room", uuid="RINCON_111"),
        CastDevice(name="Office", uuid="RINCON_222"),
    ]
    soco.discover.assert_called_once_with(timeout=8)  # type: ignore[attr-defined]


def test_play_on_sonos_loads_radio_stream(monkeypatch) -> None:
    speaker = _sonos("Living Room", "RINCON_111")
    soco = _soco_module({speaker})
    monkeypatch.setitem(sys.modules, "soco", soco)
    station = Station(stationuuid="abc", name="Example FM", url_resolved="https://example.test/live")

    play_on_sonos(CastDevice(name="Living Room", uuid="RINCON_111"), station, station.url_resolved)

    speaker.play_uri.assert_called_once_with(
        "https://example.test/live",
        title="Example FM",
        force_radio=True,
    )


def test_stop_sonos_stops_matching_speaker(monkeypatch) -> None:
    speaker = _sonos("Living Room", "RINCON_111")
    soco = _soco_module({speaker})
    monkeypatch.setitem(sys.modules, "soco", soco)

    stop_sonos(CastDevice(name="Living Room", uuid="RINCON_111"))

    speaker.stop.assert_called_once_with()


def test_toggle_sonos_pause_pauses_when_playing(monkeypatch) -> None:
    speaker = _sonos("Living Room", "RINCON_111")
    speaker.get_current_transport_info.return_value = {"current_transport_state": "PLAYING"}
    soco = _soco_module({speaker})
    monkeypatch.setitem(sys.modules, "soco", soco)

    assert toggle_sonos_pause(CastDevice(name="Living Room", uuid="RINCON_111")) is True

    speaker.pause.assert_called_once_with()


def test_change_sonos_volume_clamps_volume(monkeypatch) -> None:
    speaker = _sonos("Living Room", "RINCON_111")
    speaker.volume = 95
    soco = _soco_module({speaker})
    monkeypatch.setitem(sys.modules, "soco", soco)

    assert change_sonos_volume(CastDevice(name="Living Room", uuid="RINCON_111"), 0.2) == 1.0
    assert speaker.volume == 100


def test_toggle_chromecast_pause_pauses_when_playing(monkeypatch) -> None:
    browser = object()
    cast = _cast("Kitchen", "11111111-1111-1111-1111-111111111111")
    cast.media_controller.status.player_state = "PLAYING"
    pychromecast = _pychromecast_module()
    pychromecast.get_listed_chromecasts = MagicMock(return_value=([cast], browser))  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "pychromecast", pychromecast)

    assert toggle_chromecast_pause(
        CastDevice(name="Kitchen", uuid="11111111-1111-1111-1111-111111111111")
    ) is True

    cast.media_controller.pause.assert_called_once_with()


def test_change_chromecast_volume_clamps_volume(monkeypatch) -> None:
    browser = object()
    cast = _cast("Kitchen", "11111111-1111-1111-1111-111111111111")
    cast.status.volume_level = 0.95
    pychromecast = _pychromecast_module()
    pychromecast.get_listed_chromecasts = MagicMock(return_value=([cast], browser))  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "pychromecast", pychromecast)

    assert (
        change_chromecast_volume(
            CastDevice(name="Kitchen", uuid="11111111-1111-1111-1111-111111111111"),
            0.2,
        )
        == 1.0
    )
    cast.set_volume.assert_called_once_with(1.0)


def _cast(name: str, uuid: str) -> MagicMock:
    cast = MagicMock()
    cast.cast_info.friendly_name = name
    cast.cast_info.uuid = UUID(uuid)
    return cast


def _pychromecast_module() -> ModuleType:
    pychromecast = ModuleType("pychromecast")
    pychromecast.discovery = MagicMock()  # type: ignore[attr-defined]
    return pychromecast


def _sonos(name: str, uid: str) -> MagicMock:
    speaker = MagicMock()
    speaker.player_name = name
    speaker.uid = uid
    return speaker


def _soco_module(speakers: set[MagicMock]) -> ModuleType:
    soco = ModuleType("soco")
    soco.discover = MagicMock(return_value=speakers)  # type: ignore[attr-defined]
    return soco
