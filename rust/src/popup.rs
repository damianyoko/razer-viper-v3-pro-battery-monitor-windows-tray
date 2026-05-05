// Tiny borderless popup window near the cursor, like Windows' tray flyouts.
// Click anywhere or lose focus to dismiss. Toggle off if already open.

use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use std::ffi::c_void;

static OPEN_POPUP_HWND: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static mut POPUP_TEXT: Option<Vec<u16>> = None;
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
            lpszClassName: w!("RazerViperV3ProBatteryPopup"),
        };
        let _ = RegisterClassW(&wc);
        CLASS_REGISTERED.store(true, Ordering::Release);
    }

    // Cache text for WM_PAINT.
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    POPUP_TEXT = Some(wide);

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

    // Position: centered above cursor.
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let x = pt.x - win_w / 2;
    let y = pt.y - win_h - 12;

    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        w!("RazerViperV3ProBatteryPopup"),
        w!(""),
        WS_POPUP | WS_BORDER,
        x,
        y,
        win_w,
        win_h,
        HWND(null_mut()),
        HMENU(null_mut()),
        hinst,
        None,
    )
    .unwrap_or(HWND(null_mut()));

    if !hwnd.0.is_null() {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = UpdateWindow(hwnd);
    }
    hwnd
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

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut rc = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);

            // Background fill: dark grey
            let bg = CreateSolidBrush(COLORREF(0x00282828));
            let _ = FillRect(hdc, &rc, bg);
            let _ = DeleteObject(bg);

            // Text: white, Segoe UI semibold
            let font = create_font();
            let old_font = SelectObject(hdc, font);
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, COLORREF(0x00FFFFFF));

            if let Some(ref t) = POPUP_TEXT {
                let mut text_buf: Vec<u16> = if t.last() == Some(&0) {
                    t[..t.len() - 1].to_vec()
                } else {
                    t.clone()
                };
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
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_KILLFOCUS => {
            OPEN_POPUP_HWND.store(null_mut(), Ordering::Release);
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            OPEN_POPUP_HWND.store(null_mut(), Ordering::Release);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
