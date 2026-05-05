# Uninstall: remove autostart entries and stop any running instance.

$ErrorActionPreference = 'SilentlyContinue'

$runKey = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"

# Remove both current and legacy autostart entries.
foreach ($entry in @("ViperTray", "RazerBatteryTray")) {
    if (Get-ItemProperty $runKey -Name $entry -ErrorAction SilentlyContinue) {
        Remove-ItemProperty $runKey -Name $entry -Force
        Write-Host "Removed autostart entry: $entry" -ForegroundColor Green
    }
}

$killed = 0
Get-Process viper-tray, razer-viper-tray -ErrorAction SilentlyContinue | ForEach-Object {
    Stop-Process -Id $_.Id -Force; $killed++
}
Get-WmiObject Win32_Process -Filter "Name='powershell.exe'" |
    Where-Object { $_.CommandLine -match 'razer-battery' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force; $killed++ }

if ($killed -gt 0) { Write-Host "Stopped $killed running instance(s)." -ForegroundColor Green }
Write-Host "Done. You can delete this folder if you want."
