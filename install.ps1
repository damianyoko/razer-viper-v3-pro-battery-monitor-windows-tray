# Install: register the tray app to autostart on login.
# No admin required; writes only to HKCU.

$ErrorActionPreference = 'Stop'

$here    = Split-Path -Parent $MyInvocation.MyCommand.Path
$script  = Join-Path $here "razer-battery.ps1"
$vbs     = Join-Path $here "start-hidden.vbs"
$runKey  = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
$entry   = "RazerBatteryTray"

foreach ($f in @($script, $vbs)) {
    if (-not (Test-Path $f)) { throw "Missing file: $f" }
}

Set-ItemProperty -Path $runKey -Name $entry -Value "wscript.exe `"$vbs`"" -Type String
Write-Host "Installed. Starts on next login (or run start-hidden.vbs now)." -ForegroundColor Green
Write-Host "  Autostart entry: HKCU\…\Run\$entry"
