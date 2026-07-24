# Changelog

All notable user-facing changes to Station Scout are tracked here.

## [Unreleased]

### Added

- Linux builds are back, now as an AppImage: download one file, mark it executable, and run it on Fedora, Ubuntu, Arch, or openSUSE. It bundles the image codecs the app needs while using your distro's own GTK, GStreamer, and screen reader stack, so Orca keeps working. You'll need your distro's GStreamer plugins installed for playback.
- Rebuilt Station Scout as a Rust desktop app with native GStreamer playback for radio stations and podcasts.
- Restored Last.fm and Spotify sign-in flows in the Rust app, including browser approval and saved account state.

### Improved

- Split the main window into separate Radio and Podcasts views so each surface stands on its own instead of one long stacked list. Switch between them any time from the tabs, the new View menu, or with Ctrl+1 and Ctrl+2 - and whatever you're playing keeps going in the shared Now Playing strip below.

### Fixed

- Improved Spotify playlist export after tracking sessions so stale credentials refresh automatically when possible and clearly ask the listener to reconnect Spotify when authorization has expired.

## [1.0.0] - 2026-05-10

### Added

- Added the first Station Scout desktop release for searching public Radio Browser stations, playing streams, and managing favorites and recent stations.
- Added tune-in timers, direct stream URL playback, and platform notifications for radio listening workflows.
- Added stream metadata tracking, playlist history, Last.fm scrobbling, and Spotify playlist export for captured tracks.
- Added browser-based Last.fm and Spotify authorization that keeps app credentials out of local settings.
- Added Sonos and Chromecast output selection for routing radio playback to home speakers.
- Added accessible wxPython navigation with keyboard-first station browsing, menu accelerators, and screen-reader-friendly labels.

### Fixed

- Fixed stream title cleanup so provider metadata does not leak into track titles.
- Fixed station switching while tracking so playlist history follows the active station.
- Fixed local volume persistence across playback sessions.

### Improved

- Improved packaged builds across Windows, macOS, and Linux with release-ready Nuitka artifacts.
