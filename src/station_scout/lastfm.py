from __future__ import annotations

import hashlib
import json
from pathlib import Path
from xml.etree import ElementTree

import requests

from station_scout.tracklog import TrackEntry


API_ROOT = "https://ws.audioscrobbler.com/2.0/"
RETRYABLE_ERROR_CODES = {"11", "16"}


class LastFmError(RuntimeError):
    pass


class LastFmClient:
    def __init__(
        self,
        *,
        api_key: str,
        api_secret: str,
        session_key: str,
        api_root: str = API_ROOT,
        timeout: float = 12.0,
    ) -> None:
        self.api_key = api_key
        self.api_secret = api_secret
        self.session_key = session_key
        self.api_root = api_root
        self.timeout = timeout
        self.session = requests.Session()

    def request_token(self) -> str:
        payload = {"method": "auth.getToken", "api_key": self.api_key}
        payload["api_sig"] = sign_lastfm_request(payload, self.api_secret)
        response = self.session.post(self.api_root, data=payload, timeout=self.timeout)
        response.raise_for_status()
        root = ElementTree.fromstring(response.text)
        if root.attrib.get("status") != "ok":
            _raise_for_lfm_failure(response.text)
        token = root.findtext("token", "")
        if not token:
            raise LastFmError("Last.fm did not return an auth token")
        return token

    def auth_url(self, token: str) -> str:
        return f"https://www.last.fm/api/auth/?api_key={self.api_key}&token={token}"

    def create_session(self, token: str) -> tuple[str, str]:
        payload = {
            "method": "auth.getSession",
            "api_key": self.api_key,
            "token": token,
        }
        payload["api_sig"] = sign_lastfm_request(payload, self.api_secret)
        response = self.session.post(self.api_root, data=payload, timeout=self.timeout)
        response.raise_for_status()
        root = ElementTree.fromstring(response.text)
        if root.attrib.get("status") != "ok":
            _raise_for_lfm_failure(response.text)
        session = root.find("session")
        if session is None:
            raise LastFmError("Last.fm did not return a session")
        session_key = session.findtext("key", "")
        username = session.findtext("name", "")
        if not session_key:
            raise LastFmError("Last.fm did not return a session key")
        return session_key, username

    def update_now_playing(self, entry: TrackEntry) -> None:
        if entry.uncertain or not entry.artist or not entry.title:
            return
        self._post_signed(
            {
                "method": "track.updateNowPlaying",
                "artist": entry.artist,
                "track": entry.title,
            }
        )

    def scrobble(self, entry: TrackEntry) -> None:
        if entry.uncertain or not entry.artist or not entry.title:
            return
        self._post_signed(
            {
                "method": "track.scrobble",
                "artist": entry.artist,
                "track": entry.title,
                "timestamp": str(int(entry.timestamp.timestamp())),
                "chosenByUser": "0",
            }
        )

    def _post_signed(self, params: dict[str, str]) -> None:
        payload = {
            **params,
            "api_key": self.api_key,
            "sk": self.session_key,
        }
        payload["api_sig"] = sign_lastfm_request(payload, self.api_secret)
        response = self.session.post(self.api_root, data=payload, timeout=self.timeout)
        response.raise_for_status()
        _raise_for_lfm_failure(response.text)


class LastFmScrobbleCache:
    def __init__(self, path: Path) -> None:
        self.path = path

    def append(self, entry: TrackEntry) -> None:
        if entry.uncertain or not entry.artist or not entry.title:
            return
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.path.open("a", encoding="utf-8") as file:
            file.write(
                json.dumps(
                    {
                        "artist": entry.artist,
                        "title": entry.title,
                        "raw": entry.raw,
                        "timestamp": int(entry.timestamp.timestamp()),
                    }
                )
                + "\n"
            )


def sign_lastfm_request(params: dict[str, str], api_secret: str) -> str:
    signature_base = "".join(f"{key}{params[key]}" for key in sorted(params) if key != "format")
    return hashlib.md5(f"{signature_base}{api_secret}".encode("utf-8")).hexdigest()


def _raise_for_lfm_failure(xml_text: str) -> None:
    root = ElementTree.fromstring(xml_text)
    if root.attrib.get("status") == "ok":
        return
    error = root.find("error")
    code = error.attrib.get("code", "") if error is not None else ""
    message = error.text if error is not None else "Last.fm request failed"
    retry = "retryable" if code in RETRYABLE_ERROR_CODES else "not retryable"
    raise LastFmError(f"{message} ({retry}, code {code})")
