# Station Scout

Station Scout is a desktop internet radio explorer built around the public Radio Browser directory.

V1 focuses on:

- Searching working stations by name, country, language, and tag
- Opening StreamURL.link searches by station name or call letters, then playing copied direct stream URLs
- Playing streams with wxPython media controls
- Favorite stations and recent station history
- System tray controls for play, stop, show, and quit
- Desktop notifications
- Per-station tune-in timers
- Manual and timer-driven playlist tracking from stream metadata

## Playlist Tracking

Station Scout can track the metadata a station publishes while you listen. Manual tracking and
timer-driven show tracking write plain text session files under the app settings folder. Show logs
use one line per song in this format:

```text
Artist - Title
```

Station names are intentionally kept out of show logs so the files are easy to read and search.
Metadata that does not look like a clear artist/title pair is still kept, but marked with `?` so it
can be reviewed later.

## Notifications

Station Scout uses wxPython for the desktop frontend. Native notifications are routed through
platform-specific backends:

- Windows: Toasted, so portable builds can register an app ID and appear correctly in Action Center.
- macOS: desktop-notifier, which follows the modern Notification Center path for signed Python apps
  or signed app bundles.
- Development fallback: `wx.adv.NotificationMessage` if the platform backend is unavailable.

## Run

```powershell
python -m venv .venv
.\.venv\Scripts\python -m pip install -e ".[dev]"
.\.venv\Scripts\station-scout
```

## Test

```powershell
.\.venv\Scripts\python -m pytest
.\.venv\Scripts\python -m ruff check .
```
