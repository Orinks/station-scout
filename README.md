# Station Scout

Station Scout is an accessible Rust desktop app for internet radio and podcasts. The
desktop UI is built with wxDragon, with wx controls created in label-before-control order
for reliable screen reader announcements.

## Features

- Search and browse working internet radio stations from the public Radio Browser directory.
- Open StreamURL.link searches by station name or call letters, then play copied direct stream URLs.
- Keep favorite stations, recent station history, tune-in timers, and playlist tracking logs.
- Connect Last.fm and Spotify through browser-based authorization flows.
- Discover podcasts, subscribe by feed URL or search result, import and export OPML, poll feeds,
  queue episodes, mark episodes played, and review podcast notifications.
- Store settings and podcast data locally with JSON and SQLite. SQLite is provided through
  `rusqlite` with bundled SQLite.

## Run

```powershell
cargo run --features gui
```

The desktop app uses GStreamer for internal playback of radio stations and podcast
episodes. On Windows, install the pinned GStreamer runtime/development files before
running the GUI:

```powershell
.\scripts\install-gstreamer-windows.ps1
cargo run --features gui
```

## Test

```powershell
cargo fmt --check
cargo test
cargo check --bin station-scout --features gui
```

Rust builds require the GStreamer native development files because playback is part of
the application. GitHub Actions installs the native GStreamer packages before tests and
GUI builds.

## Packaging

The Windows installer script expects the release executable under `dist/windows/` and writes
installer output to `dist/`.

```powershell
cargo build --release --features gui
New-Item -ItemType Directory -Force dist/windows
Copy-Item target/release/station-scout.exe dist/windows/StationScout.exe
```
