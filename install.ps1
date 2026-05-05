# Install: register the tray app to autostart on login, and start it now.
# No admin required; writes only to HKCU.
# Prefers the Rust .exe (smaller/faster); falls back to the PowerShell launcher.

$ErrorActionPreference = 'Stop'

$here   = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe    = Join-Path $here "razer-viper-tray.exe"
$vbs    = Join-Path $here "start-hidden.vbs"
$runKey = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
$entry  = "RazerBatteryTray"

if (Test-Path $exe) {
    $launch = "`"$exe`""
    Write-Host "Using Rust binary: razer-viper-tray.exe" -ForegroundColor Cyan
} elseif (Test-Path $vbs) {
    $launch = "wscript.exe `"$vbs`""
    Write-Host "Using PowerShell fallback: start-hidden.vbs" -ForegroundColor Cyan
} else {
    throw "Neither razer-viper-tray.exe nor start-hidden.vbs found in $here"
}

Set-ItemProperty -Path $runKey -Name $entry -Value $launch -Type String
Write-Host "Autostart registered: HKCU\...\Run\$entry" -ForegroundColor Green

# Stop any already-running instance to avoid duplicates
Get-Process razer-viper-tray -ErrorAction SilentlyContinue | Stop-Process -Force
Get-WmiObject Win32_Process -Filter "Name='powershell.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -match 'razer-battery' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }

# Launch immediately so the icon appears now (no logout needed)
if (Test-Path $exe) {
    Start-Process -FilePath $exe
} else {
    Start-Process -FilePath "wscript.exe" -ArgumentList "`"$vbs`"" -WorkingDirectory $here -WindowStyle Hidden
}
Write-Host "Started. Look for the battery icon in your system tray." -ForegroundColor Green
