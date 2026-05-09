from __future__ import annotations

import base64
import hashlib
import secrets
import time
from dataclasses import dataclass
from urllib.parse import quote, urlencode

import requests


AUTH_URL = "https://accounts.spotify.com/authorize"
TOKEN_URL = "https://accounts.spotify.com/api/token"
API_URL = "https://api.spotify.com/v1"
DEFAULT_SCOPES = ("playlist-modify-private", "playlist-modify-public", "user-read-private")


@dataclass(frozen=True, slots=True)
class SpotifyAuthRequest:
    url: str
    state: str
    code_verifier: str


@dataclass(frozen=True, slots=True)
class SpotifyTokenSet:
    access_token: str
    refresh_token: str
    expires_at: int


@dataclass(frozen=True, slots=True)
class SpotifyPlaylistResult:
    name: str
    url: str
    matched_tracks: int
    skipped_tracks: int


def build_spotify_auth_request(
    *,
    client_id: str,
    redirect_uri: str,
    scopes: tuple[str, ...] = DEFAULT_SCOPES,
) -> SpotifyAuthRequest:
    state = secrets.token_urlsafe(24)
    code_verifier = secrets.token_urlsafe(64)[:128]
    challenge = _code_challenge(code_verifier)
    query = urlencode(
        {
            "client_id": client_id,
            "response_type": "code",
            "redirect_uri": redirect_uri,
            "scope": " ".join(scopes),
            "state": state,
            "code_challenge_method": "S256",
            "code_challenge": challenge,
        }
    )
    return SpotifyAuthRequest(f"{AUTH_URL}?{query}", state, code_verifier)


class SpotifyClient:
    def __init__(
        self,
        *,
        client_id: str,
        redirect_uri: str,
        token_url: str = TOKEN_URL,
        timeout: float = 12.0,
    ) -> None:
        self.client_id = client_id
        self.redirect_uri = redirect_uri
        self.token_url = token_url
        self.timeout = timeout
        self.session = requests.Session()

    def exchange_code(self, *, code: str, code_verifier: str) -> SpotifyTokenSet:
        response = self.session.post(
            self.token_url,
            data={
                "client_id": self.client_id,
                "grant_type": "authorization_code",
                "code": code,
                "redirect_uri": self.redirect_uri,
                "code_verifier": code_verifier,
            },
            timeout=self.timeout,
        )
        response.raise_for_status()
        payload = response.json()
        return SpotifyTokenSet(
            access_token=str(payload["access_token"]),
            refresh_token=str(payload.get("refresh_token") or ""),
            expires_at=int(time.time()) + int(payload.get("expires_in") or 3600),
        )

    def refresh_access_token(self, *, refresh_token: str) -> SpotifyTokenSet:
        response = self.session.post(
            self.token_url,
            data={
                "client_id": self.client_id,
                "grant_type": "refresh_token",
                "refresh_token": refresh_token,
            },
            timeout=self.timeout,
        )
        response.raise_for_status()
        payload = response.json()
        return SpotifyTokenSet(
            access_token=str(payload["access_token"]),
            refresh_token=str(payload.get("refresh_token") or refresh_token),
            expires_at=int(time.time()) + int(payload.get("expires_in") or 3600),
        )

    def create_playlist_from_tracks(
        self,
        *,
        access_token: str,
        name: str,
        tracks: list[tuple[str, str]],
        public: bool = False,
    ) -> SpotifyPlaylistResult:
        user_id = self._current_user_id(access_token)
        playlist = self._create_playlist(
            access_token=access_token,
            user_id=user_id,
            name=name,
            public=public,
        )
        playlist_id = str(playlist["id"])
        playlist_url = str(playlist.get("external_urls", {}).get("spotify") or "")
        uris: list[str] = []
        skipped = 0
        for artist, title in tracks:
            uri = self._search_track_uri(access_token=access_token, artist=artist, title=title)
            if uri:
                uris.append(uri)
            else:
                skipped += 1
        if uris:
            self._add_items_to_playlist(access_token=access_token, playlist_id=playlist_id, uris=uris)
        return SpotifyPlaylistResult(
            name=name,
            url=playlist_url,
            matched_tracks=len(uris),
            skipped_tracks=skipped,
        )

    def _current_user_id(self, access_token: str) -> str:
        response = self.session.get(
            f"{API_URL}/me",
            headers=self._auth_headers(access_token),
            timeout=self.timeout,
        )
        response.raise_for_status()
        return str(response.json()["id"])

    def _create_playlist(
        self,
        *,
        access_token: str,
        user_id: str,
        name: str,
        public: bool,
    ) -> dict[str, object]:
        response = self.session.post(
            f"{API_URL}/users/{quote(user_id, safe='')}/playlists",
            headers=self._auth_headers(access_token),
            json={
                "name": name,
                "public": public,
                "description": "Created from a Station Scout tracking session.",
            },
            timeout=self.timeout,
        )
        response.raise_for_status()
        return response.json()

    def _search_track_uri(self, *, access_token: str, artist: str, title: str) -> str:
        response = self.session.get(
            f"{API_URL}/search",
            headers=self._auth_headers(access_token),
            params={
                "q": f'track:"{title}" artist:"{artist}"',
                "type": "track",
                "limit": 1,
            },
            timeout=self.timeout,
        )
        response.raise_for_status()
        items = response.json().get("tracks", {}).get("items", [])
        if not items:
            return ""
        return str(items[0].get("uri") or "")

    def _add_items_to_playlist(self, *, access_token: str, playlist_id: str, uris: list[str]) -> None:
        for start in range(0, len(uris), 100):
            response = self.session.post(
                f"{API_URL}/playlists/{quote(playlist_id, safe='')}/tracks",
                headers=self._auth_headers(access_token),
                json={"uris": uris[start : start + 100]},
                timeout=self.timeout,
            )
            response.raise_for_status()

    def _auth_headers(self, access_token: str) -> dict[str, str]:
        return {"Authorization": f"Bearer {access_token}"}


def _code_challenge(code_verifier: str) -> str:
    digest = hashlib.sha256(code_verifier.encode("ascii")).digest()
    return base64.urlsafe_b64encode(digest).decode("ascii").rstrip("=")
