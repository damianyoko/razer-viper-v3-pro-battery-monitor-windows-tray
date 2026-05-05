# Razer mouse battery — system tray indicator
# No Synapse, no kernel driver. Pure HID feature reports via the mouse interface.
# Works on Razer V3-era mice (Viper V3 Pro, DeathAdder V3 Pro, etc.)

[CmdletBinding()]
param(
    [int]$RefreshMinutes = 5
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

public static class Hid {
    [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
    public static extern SafeFileHandle CreateFile(string n, uint a, uint s, IntPtr sa, uint cd, uint fa, IntPtr h);

    [DllImport("hid.dll", SetLastError=true)]
    public static extern bool HidD_SetFeature(SafeFileHandle h, byte[] b, int len);

    [DllImport("hid.dll", SetLastError=true)]
    public static extern bool HidD_GetFeature(SafeFileHandle h, byte[] b, int len);

    [DllImport("hid.dll", SetLastError=true)]
    public static extern bool HidD_GetPreparsedData(SafeFileHandle h, ref IntPtr p);

    [DllImport("hid.dll", SetLastError=true)]
    public static extern bool HidD_FreePreparsedData(IntPtr p);

    [DllImport("hid.dll", SetLastError=true)]
    public static extern int HidP_GetCaps(IntPtr p, ref Caps c);

    [DllImport("user32.dll", SetLastError=true)]
    public static extern bool DestroyIcon(IntPtr hIcon);

    [StructLayout(LayoutKind.Sequential)]
    public struct Caps {
        public ushort Usage;
        public ushort UsagePage;
        public ushort InputReportByteLength;
        public ushort OutputReportByteLength;
        public ushort FeatureReportByteLength;
        [MarshalAs(UnmanagedType.ByValArray, SizeConst=17)]
        public ushort[] Reserved;
        public ushort NumberLinkCollectionNodes;
        public ushort NumberInputButtonCaps;
        public ushort NumberInputValueCaps;
        public ushort NumberInputDataIndices;
        public ushort NumberOutputButtonCaps;
        public ushort NumberOutputValueCaps;
        public ushort NumberOutputDataIndices;
        public ushort NumberFeatureButtonCaps;
        public ushort NumberFeatureValueCaps;
        public ushort NumberFeatureDataIndices;
    }

    public const uint RW = 0xC0000000;
    public const uint SH = 3;
    public const uint OE = 3;
}
"@

function Get-RazerBattery {
    $hidGuid = "{4d1e55b2-f16f-11cf-88cb-001111000030}"
    # All HID-enumerated devices for Razer (VID 1532), regardless of Windows device class
    $devs = Get-PnpDevice -ErrorAction SilentlyContinue |
        Where-Object { $_.InstanceId -match '^HID\\VID_1532&PID_00C[01]' }

    foreach ($d in $devs) {
        $sym  = ($d.InstanceId -replace '\\','#')
        $path = "\\?\$sym#$hidGuid"

        # Open with desired_access=0 to bypass mouclass exclusive lock; still allows feature reports
        $h = [Hid]::CreateFile($path, 0, [Hid]::SH, [IntPtr]::Zero, [Hid]::OE, 0, [IntPtr]::Zero)
        if ($h.IsInvalid) { continue }

        try {
            # Filter: only the mouse HID collection (UsagePage=1, Usage=2)
            $pp = [IntPtr]::Zero
            if (-not [Hid]::HidD_GetPreparsedData($h, [ref]$pp)) { continue }
            $caps = New-Object Hid+Caps
            $caps.Reserved = New-Object uint16[] 17
            [Hid]::HidP_GetCaps($pp, [ref]$caps) | Out-Null
            [Hid]::HidD_FreePreparsedData($pp) | Out-Null

            if ($caps.UsagePage -ne 0x01 -or $caps.Usage -ne 0x02) { continue }

            # Build Razer feature report (91 bytes: report-id + 90-byte payload)
            $p = New-Object byte[] 91
            $p[0] = 0x00          # Report ID
            $p[1] = 0x00          # status
            $p[2] = 0x1F          # transaction_id (V3-gen Razer mice)
            $p[3] = 0x00; $p[4] = 0x00  # remaining_packets
            $p[5] = 0x00          # protocol_type
            $p[6] = 0x02          # data_size
            $p[7] = 0x07          # command_class (power)
            $p[8] = 0x80          # command_id (get battery level)
            # arguments [9..88] all zero
            $crc = 0
            for ($i = 3; $i -le 88; $i++) { $crc = $crc -bxor $p[$i] }
            $p[89] = $crc

            if (-not [Hid]::HidD_SetFeature($h, $p, 91)) { continue }
            Start-Sleep -Milliseconds 80

            $r = New-Object byte[] 91
            $r[0] = 0x00
            if (-not [Hid]::HidD_GetFeature($h, $r, 91)) { continue }

            # status: byte 1 (0x02 = success, 0x01 = busy)
            # Battery byte: arguments[1] -> response index 10
            if ($r[1] -in 0x01, 0x02) {
                $b = $r[10]
                if ($b -gt 0) {
                    return [math]::Round(($b / 255.0) * 100)
                }
            }
        } finally { $h.Dispose() }
    }
    return -1
}

# ====== TRAY UI ======
$ni = New-Object System.Windows.Forms.NotifyIcon
$ni.Text    = "Razer Battery"
$ni.Visible = $true

$menu        = New-Object System.Windows.Forms.ContextMenuStrip
$refreshItem = $menu.Items.Add("Refresh now")
$exitItem    = $menu.Items.Add("Exit")
$ni.ContextMenuStrip = $menu

# Single shared popup - destroyed and recreated on each click
$script:popup = $null

function Update-TrayIcon {
    $pct = Get-RazerBattery
    $offline = ($pct -lt 0)

    if ($offline) {
        $ni.Text = "Razer mouse: offline"
        $color   = [System.Drawing.Color]::FromArgb(150, 150, 150)
    } else {
        $ni.Text = "Razer mouse: $pct%"
        if     ($pct -lt 20) { $color = [System.Drawing.Color]::FromArgb(255,  70,  70) }
        elseif ($pct -lt 60) { $color = [System.Drawing.Color]::FromArgb(255, 165,   0) }
        else                 { $color = [System.Drawing.Color]::FromArgb( 80, 220, 100) }
    }

    # 32x32 horizontal battery icon (the clean original)
    $bmp = New-Object System.Drawing.Bitmap 32, 32
    $g   = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode     = 'None'
    $g.TextRenderingHint = 'SingleBitPerPixelGridFit'
    $g.Clear([System.Drawing.Color]::Transparent)

    $bodyRect = [System.Drawing.Rectangle]::new(2, 8, 24, 16)
    $capRect  = [System.Drawing.Rectangle]::new(26, 12, 3, 8)
    $outlineColor = [System.Drawing.Color]::White
    $outlinePen   = New-Object System.Drawing.Pen $outlineColor, 2
    $g.DrawRectangle($outlinePen, $bodyRect)
    $capBrush = New-Object System.Drawing.SolidBrush $outlineColor
    $g.FillRectangle($capBrush, $capRect)

    $innerW = 20; $innerH = 12
    if ($offline) {
        $qFont  = New-Object System.Drawing.Font "Segoe UI", 11, ([System.Drawing.FontStyle]::Bold)
        $qBrush = New-Object System.Drawing.SolidBrush $color
        $sf = New-Object System.Drawing.StringFormat
        $sf.Alignment = 'Center'; $sf.LineAlignment = 'Center'
        $g.DrawString("?", $qFont, $qBrush, [System.Drawing.RectangleF]::new(2, 6, 24, 20), $sf)
        $qFont.Dispose(); $qBrush.Dispose()
    } else {
        $fillW = [int][math]::Round($innerW * ($pct / 100.0))
        if ($fillW -gt 0) {
            $fillBrush = New-Object System.Drawing.SolidBrush $color
            $fillRect  = [System.Drawing.Rectangle]::new(4, 10, $fillW, $innerH)
            $g.FillRectangle($fillBrush, $fillRect)
            $fillBrush.Dispose()
        }
    }
    $g.Dispose(); $outlinePen.Dispose(); $capBrush.Dispose()

    $hIcon = $bmp.GetHicon()
    $oldIco = $ni.Icon
    $oldHandle = if ($oldIco) { $oldIco.Handle } else { [IntPtr]::Zero }
    $ni.Icon = [System.Drawing.Icon]::FromHandle($hIcon)
    if ($oldIco) { $oldIco.Dispose() }
    # Release the underlying HICON for the previous icon (Icon.Dispose alone leaks it)
    if ($oldHandle -ne [IntPtr]::Zero) { [Hid]::DestroyIcon($oldHandle) | Out-Null }
    $bmp.Dispose()
}

$refreshItem.add_Click({ Update-TrayIcon })
$exitItem.add_Click({
    $ni.Visible = $false
    $ni.Dispose()
    [System.Windows.Forms.Application]::Exit()
})

function Show-Popup {
    # Close existing popup if any
    if ($script:popup -and -not $script:popup.IsDisposed) {
        $script:popup.Close()
        $script:popup.Dispose()
        $script:popup = $null
        return  # toggle off
    }

    Update-TrayIcon
    $pct = Get-RazerBattery
    $text = if ($pct -lt 0) { "Razer mouse: offline" } else { "Razer Viper V3 Pro: $pct%" }

    $form = New-Object System.Windows.Forms.Form
    $form.FormBorderStyle = 'None'
    $form.StartPosition   = 'Manual'
    $form.TopMost         = $true
    $form.ShowInTaskbar   = $false
    $form.BackColor       = [System.Drawing.Color]::FromArgb(40, 40, 40)
    $form.Padding         = New-Object System.Windows.Forms.Padding 1

    $lbl = New-Object System.Windows.Forms.Label
    $lbl.Text      = $text
    $lbl.ForeColor = [System.Drawing.Color]::White
    $lbl.Font      = New-Object System.Drawing.Font "Segoe UI", 10
    $lbl.AutoSize  = $false
    $lbl.TextAlign = 'MiddleCenter'
    $lbl.Dock      = 'Fill'
    $form.Controls.Add($lbl)

    # Size to text
    $g = $form.CreateGraphics()
    $sz = $g.MeasureString($text, $lbl.Font)
    $g.Dispose()
    $form.Size = New-Object System.Drawing.Size ([int]$sz.Width + 24), 32

    # Position above cursor
    $cursor = [System.Windows.Forms.Cursor]::Position
    $screen = [System.Windows.Forms.Screen]::FromPoint($cursor).WorkingArea
    $x = [Math]::Min([Math]::Max($cursor.X - [int]($form.Width / 2), $screen.Left + 4), $screen.Right - $form.Width - 4)
    $y = $cursor.Y - $form.Height - 8
    if ($y -lt $screen.Top + 4) { $y = $cursor.Y + 16 }
    $form.Location = New-Object System.Drawing.Point $x, $y

    # Close on losing focus or clicking the popup
    $form.add_Deactivate({ $form.Close() })
    $form.add_Click({ $form.Close() })
    $lbl.add_Click({ $form.Close() })
    $form.add_FormClosed({
        if ($script:popup -eq $form) { $script:popup = $null }
        $form.Dispose()
    })

    $script:popup = $form
    $form.Show()
    $form.Activate()
}

# Left-click toggles popup
$ni.add_MouseClick({
    param($sender, $e)
    if ($e.Button -eq [System.Windows.Forms.MouseButtons]::Left) {
        Show-Popup
    }
})

Update-TrayIcon

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = [Math]::Max(60, $RefreshMinutes * 60) * 1000
$timer.add_Tick({ Update-TrayIcon })
$timer.Start()

[System.Windows.Forms.Application]::Run()
