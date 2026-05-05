# Install: register the tray app to autostart on login, and start it now.
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

# Idempotent: just set/overwrite the value
Set-ItemProperty -Path $runKey -Name $entry -Value "wscript.exe `"$vbs`"" -Type String
Write-Host "Autostart registered: HKCU\...\Run\$entry" -ForegroundColor Green

# Stop any already-running instance to avoid duplicates
$running = Get-WmiObject Win32_Process -Filter "Name='powershell.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -match 'razer-battery' }
foreach ($p in $running) { Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue }

# Launch immediately so the tray icon appears now (no logout needed)
Start-Process -FilePath "wscript.exe" -ArgumentList "`"$vbs`"" -WorkingDirectory $here -WindowStyle Hidden
Write-Host "Started. Look for the battery icon in your system tray." -ForegroundColor Green
