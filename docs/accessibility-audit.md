# Station Scout wxDragon Accessibility Notes

Date: 2026-05-28

Scope: `rust/src/ui.rs`

Station Scout now uses the Rust wxDragon frontend. The UI keeps the wx accessibility convention:
create each visible label immediately before its matching text box, choice, list, or other named
control unless Rust or wxDragon API constraints make that impossible.

## Preserved Rules

| Rule | Status | Notes |
| --- | --- | --- |
| WX-A11Y-001 label-before-control order | Preserved | Radio, podcast, queue, notification, settings, and dialog controls are modeled in the Rust UI blueprint and constructed in label-first order. |
| WX-A11Y-002 keyboard access | Preserved | Primary radio and podcast commands are exposed as buttons and menu/accelerator actions in the wxDragon frame. |
| WX-A11Y-003 list activation | Preserved | Station, favorites, recent, podcast search, subscription, episode, queue, and inbox lists all have explicit action buttons for keyboard users. |
| WX-A11Y-004 standard dialogs | Preserved | Rust dialogs use standard wx dialog patterns where wxDragon exposes them. |
| WX-A11Y-005 meaningful initial focus | Preserved | The UI blueprint identifies the initial focus target and startup surface. |

## Verification

```powershell
cargo fmt --check
cargo test
cargo check --bin station-scout --features gui
```

Manual screen reader testing is still needed once the next runnable Rust build is exercised with
NVDA on Windows.
