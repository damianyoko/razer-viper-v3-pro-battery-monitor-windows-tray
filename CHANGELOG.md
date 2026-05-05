# Changelog

## v2.0.1 — 2026-05-05

Bug fixes from the v2.0.0 review.

- **Popup click-away dismissal works.** v2.0.0 used `WM_KILLFOCUS` which never fires when the window is shown with `SW_SHOWNOACTIVATE` — clicks outside the popup did nothing. Now uses `SetCapture` + `WM_CAPTURECHANGED` so any click anywhere dismisses the popup. README claim now matches behaviour.
- **Popup position clamped to monitor work area.** v2.0.0 could spill the popup off the right edge of the screen or onto an adjacent monitor when the cursor was near the boundary. Now uses `MonitorFromPoint` + `GetMonitorInfoW` and flips the popup below the cursor if it would overflow the top of the work area.
- **HID `busy` (status 0x01) responses are now properly retried** with full Set/Get cycles and escalating sleep (80 → 150 → 250 ms) instead of being accepted with stale data.
- **Replaced `static mut POPUP_TEXT` with `GWLP_USERDATA`** — proper per-window storage, no `unsafe` data race surface, future-proof against Rust 2024 lints.
- **`SetWaitableTimer` failure no longer silent** — the worker thread depends on it; failure now panics loudly rather than leaving a permanently stale tray icon.
- README: added footprint comparison vs Razer Synapse 3.

## v2.0.0 — 2026-05-05

Full Rust rewrite. Same protocol, much smaller footprint.

- Native Rust implementation: 213 KB single-file `.exe`, ~9 MB RAM (down from ~125 MB on PowerShell)
- Direct Win32 message pump — no `winit`, no `image`, no `tokio`, no GUI framework
- Waitable timer for the 5-minute refresh loop (system-coalesced, laptop-battery friendly)
- `CreateMutexW` single-instance guard
- Tray icon via `tray-icon` crate (Tauri's), proper `DestroyIcon` lifecycle
- Custom Win32 popup window for the click flyout (raw GDI rendering)
- HID query in raw `windows-rs` calls — `CreateFileW` with `dwDesiredAccess = 0` to bypass `mouclass`
- PowerShell version retained as a transparent reference implementation

### Migration from v1.0.0

```powershell
.\uninstall.ps1
# download release zip, extract over the same folder
.\install.ps1
```
The installer auto-detects the `.exe` and prefers it over the PowerShell fallback.

## v1.0.0 — 2026-05-05

Initial PowerShell release.

- HID feature-report battery query for Razer Viper V3 Pro (PID `0x00C0` / `0x00C1`, transaction ID `0x1F`)
- System tray icon: horizontal battery shape with colour-coded fill (green ≥60%, orange 20–60%, red <20%, grey when offline)
- Left-click toggles a small popup near the cursor with the percentage
- Right-click menu: Refresh now / Exit
- 5-minute auto-refresh
- `install.ps1` / `uninstall.ps1` — HKCU autostart, no admin required
