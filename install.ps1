# Install: register Viper Tray to autostart on login, and start it now.
# No admin required; writes only to the user's profile.
# Uses a Startup folder shortcut (more reliable than HKCU\Run on Windows 11
# for unsigned binaries; Run keys get silently skipped under various conditions).

$ErrorActionPreference = 'Stop'

$here   = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe    = Join-Path $here "viper-tray.exe"
$exeOld = Join-Path $here "razer-viper-tray.exe"  # pre-rename binary
$vbs    = Join-Path $here "start-hidden.vbs"

# Pick the best available target.
if (Test-Path $exe) {
    $target = $exe
    Write-Host "Using Rust binary: viper-tray.exe" -ForegroundColor Cyan
} elseif (Test-Path $exeOld) {
    $target = $exeOld
    Write-Host "Using legacy binary: razer-viper-tray.exe" -ForegroundColor DarkYellow
} elseif (Test-Path $vbs) {
    $target = "wscript.exe"
    $vbsArg = $vbs
    Write-Host "Using PowerShell fallback: start-hidden.vbs" -ForegroundColor Cyan
} else {
    throw "No launcher found in $here (expected viper-tray.exe or start-hidden.vbs)"
}

# Clean up any prior install state.
$startupDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup"
$lnk        = Join-Path $startupDir "Viper Tray.lnk"
$runKey     = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
$saKey      = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
foreach ($n in @("ViperTray", "RazerBatteryTray")) {
    if (Get-ItemProperty $runKey -Name $n -ErrorAction SilentlyContinue) {
        Remove-ItemProperty $runKey -Name $n -Force
        Write-Host "Removed legacy HKCU\Run entry: $n" -ForegroundColor DarkYellow
    }
    if (Get-ItemProperty $saKey -Name $n -ErrorAction SilentlyContinue) {
        Remove-ItemProperty $saKey -Name $n -Force
    }
}

# Create the Startup folder shortcut.
$ws = New-Object -ComObject WScript.Shell
$sc = $ws.CreateShortcut($lnk)
$sc.TargetPath = $target
if ($vbsArg) { $sc.Arguments = "`"$vbsArg`"" }
$sc.WorkingDirectory = $here
$sc.WindowStyle = 7  # minimised
$sc.Description = "Viper Tray battery indicator"
$sc.Save()
Write-Host "Autostart shortcut created: $lnk" -ForegroundColor Green

# Stop any currently-running instance to avoid duplicates.
Get-Process viper-tray, razer-viper-tray -ErrorAction SilentlyContinue | Stop-Process -Force
Get-WmiObject Win32_Process -Filter "Name='powershell.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -match 'razer-battery' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }

# Launch immediately.
if ($vbsArg) {
    Start-Process -FilePath $target -ArgumentList "`"$vbsArg`"" -WorkingDirectory $here -WindowStyle Hidden
} else {
    Start-Process -FilePath $target
}
Write-Host "Started. Look for the battery icon in your system tray." -ForegroundColor Green
