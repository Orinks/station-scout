from __future__ import annotations

import base64
import hashlib
import secrets
import time
from dataclasses import dataclass
from urllib.parse import urlencode

import requests


AUTH_URL = "https://accounts.spotify.com/authorize"
TOKEN_URL = "https://accounts.spotify.com/api/token"
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


def _code_challenge(code_verifier: str) -> str:
    digest = hashlib.sha256(code_verifier.encode("ascii")).digest()
    return base64.urlsafe_b64encode(digest).decode("ascii").rstrip("=")
