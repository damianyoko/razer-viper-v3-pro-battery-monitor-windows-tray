# Changelog

## v1.0.0 — 2026-05-05

Initial release.

- HID feature-report battery query for Razer Viper V3 Pro (PID `0x00C0` / `0x00C1`, transaction ID `0x1F`)
- System tray icon: horizontal battery shape with colour-coded fill (green ≥60%, orange 20–60%, red <20%, grey when offline)
- Left-click toggles a small popup near the cursor with "Razer Viper V3 Pro: NN%"
- Right-click menu: Refresh now / Exit
- 5-minute auto-refresh
- `install.ps1` / `uninstall.ps1` — HKCU autostart, no admin required
