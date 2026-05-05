# Viper V3 Pro Battery Tray

A tiny system tray battery indicator for the **Razer Viper V3 Pro** wireless mouse. Pure PowerShell. No Razer Synapse, no kernel drivers, no admin required.

## Why

Synapse is bloatware. It runs three services, installs kernel-mode drivers, and phones home — just to show you a battery percentage. This script does the same job in ~150 lines of PowerShell, talks to the mouse over standard HID feature reports, and uses about 30 MB of RAM.

## What you get

- A horizontal battery icon in the system tray, colour-coded:
  - **Green** ≥ 60%
  - **Orange** 20–60%
  - **Red** < 20%
- Left-click → menu showing "Razer Viper V3 Pro: NN%"
- Auto-refresh every 5 minutes
- "Refresh now" and "Exit" in the same menu
- Survives reboots once installed

## Requirements

- Windows 10 or 11
- PowerShell 5.1+ (built into Windows)
- A Razer Viper V3 Pro (USB dongle plugged in)

## Install

1. Clone or download this repo somewhere stable, e.g. `C:\Users\<you>\Documents\Scripts\RazerBattery`.
2. From an unprivileged PowerShell prompt in that folder:
   ```powershell
   .\install.ps1
   ```
   This adds an entry to `HKCU\…\Run` so the tray launches at every login. No admin needed.
3. Either log out and back in, or run `start-hidden.vbs` to start it immediately.

## Uninstall

```powershell
.\uninstall.ps1
```
Removes the autostart entry and stops any running instance. Delete the folder afterwards if you want.

## How it works

The Viper V3 Pro exposes its mouse HID interface (UsagePage `0x01`, Usage `0x02`) on the dongle (VID `0x1532`, PID `0x00C0` / `0x00C1`). Razer's command protocol piggybacks on that interface as a 90-byte feature report.

The script:
1. Enumerates HID devices for VID `0x1532`
2. Opens each candidate with `desired_access = 0` to bypass `mouclass`'s exclusive lock (mice can't be opened with read/write while in use, but feature-report-only access works)
3. Filters for the mouse collection (UP=1, Usage=2, FeatureReportByteLength=91)
4. Sends a Razer feature report: `command_class = 0x07`, `command_id = 0x80` (get battery level), transaction ID `0x1F`
5. Reads the response; battery byte is `arguments[1]` (offset 10), scaled `0–255 → 0–100%`

The protocol details come from the [OpenRazer](https://github.com/openrazer/openrazer) project.

## Why it only works for Viper V3 Pro

The PID filter and transaction ID are hard-coded for this mouse. Other Razer mice use different PIDs and some older ones use TID `0x3F`. If you have a different Razer mouse, [xzeldon/razer-battery-report](https://github.com/xzeldon/razer-battery-report) supports more devices.

## License

MIT. See [LICENSE](LICENSE).
