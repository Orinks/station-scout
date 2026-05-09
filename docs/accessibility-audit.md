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

## Follow-up Label-Order Pass

Date: 2026-05-09

The second pass focused on controls that looked labeled visually but were still likely to be announced as unlabeled by screen readers because the native wx creation order placed the edit/list control before the `wx.StaticText` label.

| # | Rule | Severity | File | Description | Resolution |
| --- | --- | --- | --- | --- | --- |
| 1 | WX-A11Y-001 | Critical | `src/station_scout/app.py` | Search source and search edit boxes were created before their labels. | Reordered construction so each label is created immediately before its matching input/select. |
| 2 | WX-A11Y-001 | Critical | `src/station_scout/app.py` | Station results, favorites, recents, and timers did not have reliable label-before-control order. | Added/reordered static labels immediately before each list control. |
| 3 | WX-A11Y-001 | Serious | `src/station_scout/app.py` | The timer dialog show-name edit box was created before its label. | Created the show-name label before the edit box and kept them adjacent in the sizer. |
| 4 | WX-A11Y-001 | Moderate | `src/station_scout/app.py` | The Settings playlist-log folder edit box label was not in the same immediate row. | Reworked the folder row to label, edit box, then choose-folder button. |

Additional fallback help text/tooltips were added to labeled controls, but those are secondary to the label-before-control wx ordering.
