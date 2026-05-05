# Uninstall: remove autostart entry and stop any running instance.

$ErrorActionPreference = 'SilentlyContinue'

$runKey = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
$entry  = "RazerBatteryTray"

if (Get-ItemProperty $runKey -Name $entry -ErrorAction SilentlyContinue) {
    Remove-ItemProperty $runKey -Name $entry -Force
    Write-Host "Removed autostart entry." -ForegroundColor Green
} else {
    Write-Host "No autostart entry found."
}

$killed = 0
Get-WmiObject Win32_Process -Filter "Name='powershell.exe'" |
    Where-Object { $_.CommandLine -match 'razer-battery' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force; $killed++ }

if ($killed -gt 0) { Write-Host "Stopped $killed running instance(s)." -ForegroundColor Green }
Write-Host "Done. You can delete this folder if you want."
