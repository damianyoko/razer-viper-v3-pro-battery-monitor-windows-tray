# Install: register Viper Tray to autostart on login, and start it now.
# No admin required; writes only to HKCU.
# Prefers the Rust .exe; falls back to the PowerShell launcher.

$ErrorActionPreference = 'Stop'

$here   = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe    = Join-Path $here "viper-tray.exe"
$exeOld = Join-Path $here "razer-viper-tray.exe"  # pre-rename binary
$vbs    = Join-Path $here "start-hidden.vbs"
$runKey = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
$entry  = "ViperTray"
$entryOld = "RazerBatteryTray"  # pre-rename Run entry; clean up if present

# Clean up any prior name so we don't end up with two autostart entries.
if (Get-ItemProperty $runKey -Name $entryOld -ErrorAction SilentlyContinue) {
    Remove-ItemProperty $runKey -Name $entryOld -Force
    Write-Host "Removed legacy autostart entry: $entryOld" -ForegroundColor DarkYellow
}

# Pick the best available launcher.
if (Test-Path $exe) {
    $launch = "`"$exe`""
    Write-Host "Using Rust binary: viper-tray.exe" -ForegroundColor Cyan
} elseif (Test-Path $exeOld) {
    $launch = "`"$exeOld`""
    Write-Host "Using legacy binary: razer-viper-tray.exe (consider re-downloading)" -ForegroundColor DarkYellow
} elseif (Test-Path $vbs) {
    $launch = "wscript.exe `"$vbs`""
    Write-Host "Using PowerShell fallback: start-hidden.vbs" -ForegroundColor Cyan
} else {
    throw "No launcher found in $here (expected viper-tray.exe or start-hidden.vbs)"
}

Set-ItemProperty -Path $runKey -Name $entry -Value $launch -Type String
Write-Host "Autostart registered: HKCU\...\Run\$entry" -ForegroundColor Green

# Stop any currently-running instances (any name).
Get-Process viper-tray, razer-viper-tray -ErrorAction SilentlyContinue | Stop-Process -Force
Get-WmiObject Win32_Process -Filter "Name='powershell.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -match 'razer-battery' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }

# Launch.
if (Test-Path $exe) {
    Start-Process -FilePath $exe
} elseif (Test-Path $exeOld) {
    Start-Process -FilePath $exeOld
} else {
    Start-Process -FilePath "wscript.exe" -ArgumentList "`"$vbs`"" -WorkingDirectory $here -WindowStyle Hidden
}
Write-Host "Started. Look for the battery icon in your system tray." -ForegroundColor Green
