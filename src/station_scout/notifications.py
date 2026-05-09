from __future__ import annotations

import asyncio
import platform
import threading
from dataclasses import dataclass
from typing import Protocol

import wx
import wx.adv


APP_ID = "Orinks.StationScout"
APP_NAME = "Station Scout"


class Notifier(Protocol):
    def notify(self, title: str, message: str) -> None:
        pass


@dataclass(slots=True)
class WxNotifier:
    parent: wx.Window | None = None

    def notify(self, title: str, message: str) -> None:
        notification = wx.adv.NotificationMessage(title, message, parent=self.parent)
        notification.Show(timeout=wx.adv.NotificationMessage.Timeout_Auto)


class ToastedNotifier:
    def __init__(self, app_id: str = APP_ID, app_name: str = APP_NAME) -> None:
        from toasted import Text, Toast

        self.text_cls = Text
        self.toast_cls = Toast
        self.app_id = Toast.register_app_id(app_id, app_name)

    def notify(self, title: str, message: str) -> None:
        toast = self.toast_cls(app_id=self.app_id)
        toast.elements = [self.text_cls(title), self.text_cls(message)]
        toast.show()


class DesktopNotifierAdapter:
    def __init__(self, app_name: str = APP_NAME) -> None:
        from desktop_notifier import DesktopNotifier

        self.notifier = DesktopNotifier(app_name=app_name)

    def notify(self, title: str, message: str) -> None:
        threading.Thread(
            target=lambda: asyncio.run(self.notifier.send(title=title, message=message)),
            daemon=True,
        ).start()


def create_notifier(
    *,
    system: str | None = None,
    parent: wx.Window | None = None,
) -> Notifier:
    system_name = system or platform.system()
    if system_name == "Windows":
        try:
            return ToastedNotifier()
        except (ImportError, OSError, RuntimeError):
            return WxNotifier(parent)
    if system_name == "Darwin":
        try:
            return DesktopNotifierAdapter()
        except (ImportError, OSError, RuntimeError):
            return WxNotifier(parent)
    return WxNotifier(parent)

