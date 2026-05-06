# Uninstall: remove autostart shortcut + any legacy Run entries, stop the tray.

$ErrorActionPreference = 'SilentlyContinue'

# Startup folder shortcut.
$lnk = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\Viper Tray.lnk"
if (Test-Path $lnk) {
    Remove-Item $lnk -Force
    Write-Host "Removed Startup folder shortcut." -ForegroundColor Green
}

# Legacy HKCU\Run entries from older installs.
$runKey = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
$saKey  = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
foreach ($n in @("ViperTray", "RazerBatteryTray")) {
    if (Get-ItemProperty $runKey -Name $n -ErrorAction SilentlyContinue) {
        Remove-ItemProperty $runKey -Name $n -Force
        Write-Host "Removed legacy autostart entry: $n" -ForegroundColor Green
    }
    if (Get-ItemProperty $saKey -Name $n -ErrorAction SilentlyContinue) {
        Remove-ItemProperty $saKey -Name $n -Force
    }
}

# Stop running instances.
$killed = 0
Get-Process viper-tray, razer-viper-tray -ErrorAction SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.Id -Force; $killed++ }
Get-WmiObject Win32_Process -Filter "Name='powershell.exe'" |
    Where-Object { $_.CommandLine -match 'razer-battery' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force; $killed++ }
if ($killed -gt 0) { Write-Host "Stopped $killed running instance(s)." -ForegroundColor Green }

Write-Host "Done. You can delete this folder if you want."
