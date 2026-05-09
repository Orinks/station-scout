# Station Scout wxPython Accessibility Audit

Date: 2026-05-09

Scope: `src/station_scout/app.py`

| # | Rule | Severity | File | Description | Resolution |
| --- | --- | --- | --- | --- | --- |
| 1 | WX-A11Y-002 | Critical | `src/station_scout/app.py` | The main frame had no accelerator table, so core actions depended on tabbing or pointer interaction. | Added a menu bar and accelerators for search focus, playback, stop, timers, tracking, and settings. |
| 2 | WX-A11Y-003 | Critical | `src/station_scout/app.py` | Favorites and recent stations only supported double-click activation. | Added Enter-key activation for the focused favorites or recents list. |
| 3 | WX-A11Y-004 | Serious | `src/station_scout/app.py` | The timer dialog used nonstandard dialog buttons and did not explicitly handle Escape. | Switched to `CreateStdDialogButtonSizer` and added Escape-to-cancel handling. |
| 4 | WX-A11Y-005 | Serious | `src/station_scout/app.py` | The timer dialog opened without setting a meaningful initial focus. | Focus now starts on the time picker. |
| 5 | Navigation noise | Serious | `src/station_scout/app.py` | Last.fm, Spotify, and log-folder controls were mixed into the main station search surface. | Moved those controls into a dedicated Settings dialog with grouped sections and its own Escape handling. |

Verification:

- `uv run ruff check .`
- `uv run pytest`
- `uv run python -m build`
- `uv run python -c "from station_scout.app import StationScoutFrame, SettingsDialog, TimerDialog; print('ui imports ok')"`

Manual screen reader testing is still needed once the next runnable build is exercised with NVDA on Windows and VoiceOver on macOS.
