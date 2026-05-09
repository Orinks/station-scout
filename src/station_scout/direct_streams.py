from __future__ import annotations

from urllib.parse import quote, urlparse

from station_scout.models import Station


STREAMURL_BASE = "https://streamurl.link"
STREAM_SCHEMES = ("http", "https", "icy")


def streamurl_search_url(query: str, *, base_url: str = STREAMURL_BASE) -> str:
    return f"{base_url.rstrip('/')}/s/{quote(query.strip())}/"


def station_from_direct_url(
    *,
    name: str,
    url: str,
    homepage: str = "",
) -> Station:
    clean_url = url.strip()
    if not is_stream_url_candidate(clean_url):
        raise ValueError("Enter a direct stream URL beginning with http, https, or icy.")
    return Station.direct_stream(
        name=name.strip() or clean_url,
        url=clean_url,
        homepage=homepage.strip(),
        source="Direct stream URL",
    )


def is_stream_url_candidate(url: str) -> bool:
    parsed = urlparse(url.strip())
    return parsed.scheme.lower() in STREAM_SCHEMES and bool(parsed.netloc)

