// Tiny borderless popup window near the cursor, like a tray flyout.
// Uses SetCapture for proper click-anywhere-to-dismiss; GWLP_USERDATA for the
// per-window text payload (no static mut). Clamps position to the monitor work area.

use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::*;

use std::ffi::c_void;

static OPEN_POPUP_HWND: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static CLASS_REGISTERED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

const PADDING_X: i32 = 14;
const PADDING_Y: i32 = 8;

pub fn toggle_popup(text: &str) {
    unsafe {
        // Toggle off if already showing.
        let existing = OPEN_POPUP_HWND.swap(null_mut(), Ordering::AcqRel);
        if !existing.is_null() {
            let _ = DestroyWindow(HWND(existing));
            return;
        }
        let hwnd = create_popup(text);
        if !hwnd.0.is_null() {
            OPEN_POPUP_HWND.store(hwnd.0, Ordering::Release);
        }
    }
}

unsafe fn create_popup(text: &str) -> HWND {
    let hmodule = GetModuleHandleW(None).unwrap_or_default();
    let hinst = HINSTANCE(hmodule.0);

    if !CLASS_REGISTERED.load(Ordering::Acquire) {
        let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or(HCURSOR(null_mut()));
        let wc = WNDCLASSW {
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: HICON(null_mut()),
            hCursor: cursor,
            hbrBackground: HBRUSH(null_mut()),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: w!("ViperTrayPopup"),
        };
        let _ = RegisterClassW(&wc);
        CLASS_REGISTERED.store(true, Ordering::Release);
    }

    // Box up the wide-encoded text; pointer goes through CREATESTRUCT::lpCreateParams,
    // we stash it via GWLP_USERDATA in WM_NCCREATE, free in WM_NCDESTROY.
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    let boxed: Box<Vec<u16>> = Box::new(wide);
    let create_param: *mut Vec<u16> = Box::into_raw(boxed);

    // Measure the text to size the window.
    let hdc_screen = GetDC(HWND(null_mut()));
    let font = create_font();
    let old = SelectObject(hdc_screen, font);
    let mut sz = SIZE::default();
    let text_wide_no_null: Vec<u16> = text.encode_utf16().collect();
    let _ = GetTextExtentPoint32W(hdc_screen, &text_wide_no_null, &mut sz);
    SelectObject(hdc_screen, old);
    let _ = DeleteObject(font);
    ReleaseDC(HWND(null_mut()), hdc_screen);

    let win_w = sz.cx + PADDING_X * 2;
    let win_h = sz.cy + PADDING_Y * 2;

    // Position: centered above cursor, clamped to the relevant monitor's work area.
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let (x, y) = clamp_to_work_area(pt, win_w, win_h);

    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        w!("ViperTrayPopup"),
        w!(""),
        WS_POPUP | WS_BORDER,
        x,
        y,
        win_w,
        win_h,
        HWND(null_mut()),
        HMENU(null_mut()),
        hinst,
        Some(create_param as *const c_void),
    )
    .unwrap_or(HWND(null_mut()));

    if hwnd.0.is_null() {
        // Window creation failed — reclaim the boxed text so we don't leak.
        let _ = Box::from_raw(create_param);
        return HWND(null_mut());
    }

    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    let _ = UpdateWindow(hwnd);

    // Capture the mouse so clicks anywhere — including outside the popup — come to us.
    SetCapture(hwnd);
    hwnd
}

unsafe fn clamp_to_work_area(cursor: POINT, win_w: i32, win_h: i32) -> (i32, i32) {
    // Pick the monitor the cursor is on; fall back gracefully if MonitorFromPoint fails.
    let mon = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
    let mut info: MONITORINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let work = if GetMonitorInfoW(mon, &mut info).as_bool() {
        info.rcWork
    } else {
        RECT { left: 0, top: 0, right: 1920, bottom: 1080 }
    };

    // Default position: centered above cursor.
    let mut x = cursor.x - win_w / 2;
    let mut y = cursor.y - win_h - 12;

    // Horizontal clamp.
    if x + win_w > work.right {
        x = work.right - win_w - 4;
    }
    if x < work.left {
        x = work.left + 4;
    }

    // Vertical: if it would spill above the work area, flip below the cursor.
    if y < work.top {
        y = cursor.y + 16;
    }
    if y + win_h > work.bottom {
        y = work.bottom - win_h - 4;
    }

    (x, y)
}

unsafe fn create_font() -> HFONT {
    CreateFontW(
        -14,
        0,
        0,
        0,
        FW_SEMIBOLD.0 as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32 | (FF_DONTCARE.0 as u32) << 4,
        w!("Segoe UI"),
    )
}

unsafe fn dismiss(hwnd: HWND) {
    // Release capture first so it doesn't leak to whichever window becomes foreground.
    let _ = ReleaseCapture();
    OPEN_POPUP_HWND.compare_exchange(
        hwnd.0,
        null_mut(),
        Ordering::AcqRel,
        Ordering::Acquire,
    )
    .ok();
    let _ = DestroyWindow(hwnd);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // WM_NCCREATE is the earliest reliable hook — earlier than WM_CREATE.
        // We pull our boxed text pointer out of CREATESTRUCT and stash it via GWLP_USERDATA.
        WM_NCCREATE => {
            let cs = lp.0 as *const CREATESTRUCTW;
            if !cs.is_null() {
                let p = (*cs).lpCreateParams as isize;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, p);
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }

        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut rc = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);

            // Background fill: dark grey
            let bg = CreateSolidBrush(COLORREF(0x00282828));
            let _ = FillRect(hdc, &rc, bg);
            let _ = DeleteObject(bg);

            let font = create_font();
            let old_font = SelectObject(hdc, font);
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, COLORREF(0x00FFFFFF));

            // Pull the boxed wide text out of GWLP_USERDATA. We only read it.
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Vec<u16>;
            if !p.is_null() {
                let v: &Vec<u16> = &*p;
                let mut text_buf: Vec<u16> =
                    if v.last() == Some(&0) { v[..v.len() - 1].to_vec() } else { v.clone() };
                let _ = DrawTextW(
                    hdc,
                    &mut text_buf,
                    &mut rc,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );
            }

            SelectObject(hdc, old_font);
            let _ = DeleteObject(font);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        // Clicks anywhere — inside or outside the popup — dismiss. Capture made it possible.
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN
        | WM_NCLBUTTONDOWN | WM_NCRBUTTONDOWN | WM_NCMBUTTONDOWN => {
            dismiss(hwnd);
            LRESULT(0)
        }

        // If something else steals capture (system dialog, alt-tab, another popup), bail.
        WM_CAPTURECHANGED => {
            // Only dismiss if capture moved to a different window than ours.
            if HWND(lp.0 as *mut c_void) != hwnd {
                OPEN_POPUP_HWND.compare_exchange(
                    hwnd.0,
                    null_mut(),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .ok();
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }

        WM_NCDESTROY => {
            // Reclaim the boxed text we stashed in WM_NCCREATE.
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if p != 0 {
                let _ = Box::from_raw(p as *mut Vec<u16>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            OPEN_POPUP_HWND.compare_exchange(
                hwnd.0,
                null_mut(),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .ok();
            DefWindowProcW(hwnd, msg, wp, lp)
        }

        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
