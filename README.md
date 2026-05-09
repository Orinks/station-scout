# Station Scout

Station Scout is a desktop internet radio explorer built around the public Radio Browser directory.

V1 focuses on:

- Searching working stations by name, country, language, and tag
- Playing streams with wxPython media controls
- Favorite stations and recent station history
- System tray controls for play, stop, show, and quit
- Desktop notifications
- Per-station tune-in timers

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
