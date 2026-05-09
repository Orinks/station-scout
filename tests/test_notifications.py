from __future__ import annotations

import sys
import types

from station_scout import notifications


def test_windows_uses_toasted_when_available(monkeypatch) -> None:
    fake_module = types.SimpleNamespace(Toast=FakeToast, Text=FakeText)
    monkeypatch.setitem(sys.modules, "toasted", fake_module)

    notifier = notifications.create_notifier(system="Windows")

    assert isinstance(notifier, notifications.ToastedNotifier)


def test_windows_falls_back_to_wx_when_toasted_missing(monkeypatch) -> None:
    monkeypatch.setitem(sys.modules, "toasted", None)

    notifier = notifications.create_notifier(system="Windows")

    assert isinstance(notifier, notifications.WxNotifier)


def test_macos_uses_desktop_notifier_when_available(monkeypatch) -> None:
    fake_module = types.SimpleNamespace(DesktopNotifier=FakeDesktopNotifier)
    monkeypatch.setitem(sys.modules, "desktop_notifier", fake_module)

    notifier = notifications.create_notifier(system="Darwin")

    assert isinstance(notifier, notifications.DesktopNotifierAdapter)


def test_linux_uses_wx_notifier() -> None:
    notifier = notifications.create_notifier(system="Linux")

    assert isinstance(notifier, notifications.WxNotifier)


class FakeText:
    def __init__(self, value: str) -> None:
        self.value = value


class FakeToast:
    def __init__(self, *, app_id: str) -> None:
        self.app_id = app_id
        self.elements = []

    @staticmethod
    def register_app_id(app_id: str, app_name: str) -> str:
        return f"{app_id}:{app_name}"

    def show(self) -> None:
        return None


class FakeDesktopNotifier:
    def __init__(self, *, app_name: str) -> None:
        self.app_name = app_name

    async def send(self, *, title: str, message: str) -> None:
        return None

