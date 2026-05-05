// Razer Viper V3 Pro battery tray. No console window.
#![windows_subsystem = "windows"]

mod battery;
mod icon;
mod popup;

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

use muda::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const REFRESH_SECS: i64 = 300;
const APP_TITLE: &str = "Viper Tray";

// HANDLE is *mut c_void which isn't Send by default. We share kernel handles between
// the UI thread and the worker thread; that's safe — Win32 sync handles are designed
// for cross-thread use.
#[derive(Clone, Copy)]
struct SendHandle(HANDLE);
unsafe impl Send for SendHandle {}
unsafe impl Sync for SendHandle {}

const WM_APP_REFRESH: u32 = WM_APP + 1;
const WM_APP_TRAY_LCLICK: u32 = WM_APP + 2;
const WM_APP_MENU: u32 = WM_APP + 3;

static MAIN_THREAD_ID: AtomicI32 = AtomicI32::new(0);

fn main() {
    // Single-instance: Local\-scoped named mutex (no privilege required).
    let _guard = match acquire_singleton() {
        Some(h) => h,
        None => return, // another instance is already running
    };

    MAIN_THREAD_ID.store(unsafe { GetCurrentThreadId() } as i32, Ordering::Release);

    // Build tray + menu
    let menu = Menu::new();
    let item_refresh = MenuItem::new("Refresh now", true, None);
    let item_exit = MenuItem::new("Exit", true, None);
    menu.append(&item_refresh).unwrap();
    menu.append(&item_exit).unwrap();
    let id_refresh = item_refresh.id().clone();
    let id_exit = item_exit.id().clone();

    let pct = Arc::new(AtomicI32::new(-1)); // -1 offline, 0..=100 pct

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false) // left-click = our popup, right-click = menu
        .with_tooltip(format!("{}: --", APP_TITLE))
        .with_icon(build_icon(None))
        .build()
        .expect("failed to build tray");

    // Waitable timer: drives both periodic and manual refreshes.
    // Manual refresh = SetWaitableTimer(due_time = -1ms) to fire immediately.
    let timer = SendHandle(
        unsafe { CreateWaitableTimerW(None, false, PCWSTR::null()) }
            .expect("CreateWaitableTimer"),
    );
    arm_timer(timer, 0); // first read fires immediately on startup

    // Worker: waits on the timer, queries the mouse, posts WM_APP_REFRESH to UI thread.
    {
        let pct = pct.clone();
        thread::spawn(move || loop {
            // Block until the timer fires (system-coalesced).
            wait_handle(timer);
            let val = battery::read_battery_percent();
            pct.store(val.map(|v| v as i32).unwrap_or(-1), Ordering::Release);
            unsafe {
                let _ = PostThreadMessageW(
                    MAIN_THREAD_ID.load(Ordering::Acquire) as u32,
                    WM_APP_REFRESH,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
            // Re-arm for the next periodic check.
            arm_timer(timer, REFRESH_SECS);
        });
    }

    // Tray-icon event handler: route left-click into our message pump.
    TrayIconEvent::set_event_handler(Some(|event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            unsafe {
                let _ = PostThreadMessageW(
                    MAIN_THREAD_ID.load(Ordering::Acquire) as u32,
                    WM_APP_TRAY_LCLICK,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    }));

    // Menu event handler: encode item id into LPARAM.
    let menu_event = Arc::new(AtomicI32::new(0));
    {
        let id_refresh = id_refresh.clone();
        let id_exit = id_exit.clone();
        let menu_event = menu_event.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let v = if event.id == id_refresh {
                1
            } else if event.id == id_exit {
                2
            } else {
                0
            };
            if v != 0 {
                menu_event.store(v, Ordering::Release);
                unsafe {
                    let _ = PostThreadMessageW(
                        MAIN_THREAD_ID.load(Ordering::Acquire) as u32,
                        WM_APP_MENU,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
            }
        }));
    }

    // Main message loop on the UI thread.
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            match msg.message {
                m if m == WM_APP_REFRESH => {
                    let p = pct.load(Ordering::Acquire);
                    let opt = if p < 0 { None } else { Some(p as u8) };
                    let _ = tray.set_icon(Some(build_icon(opt)));
                    let tip = match opt {
                        Some(v) => format!("{}: {}%", APP_TITLE, v),
                        None => format!("{}: offline", APP_TITLE),
                    };
                    let _ = tray.set_tooltip(Some(tip));
                }
                m if m == WM_APP_TRAY_LCLICK => {
                    let p = pct.load(Ordering::Acquire);
                    let label = if p < 0 { "offline".into() } else { format!("{}%", p) };
                    popup::toggle_popup(&label);
                    // fire a fresh read so the next click shows newer data
                    arm_timer(timer, 0);
                }
                m if m == WM_APP_MENU => {
                    let v = menu_event.swap(0, Ordering::AcqRel);
                    if v == 1 {
                        arm_timer(timer, 0);
                    } else if v == 2 {
                        PostQuitMessage(0);
                    }
                }
                _ => {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }

    unsafe {
        let _ = CloseHandle(timer.0);
    }
}



fn arm_timer(h: SendHandle, secs: i64) {
    // Negative = relative, units = 100ns. 0 secs => fire immediately (-1 == 100ns).
    let due_ns: i64 = if secs <= 0 { -1 } else { -secs * 10_000_000 };
    // Loud-fail: if the timer can't be armed, the worker thread will block forever
    // and the tray will go stale silently. Better to crash than hang invisibly.
    unsafe {
        SetWaitableTimer(h.0, &due_ns, 0, None, None, false)
            .expect("SetWaitableTimer failed — worker would be dead");
    }
}

fn wait_handle(h: SendHandle) {
    unsafe {
        let _ = WaitForSingleObject(h.0, INFINITE);
    }
}

fn build_icon(pct: Option<u8>) -> Icon {
    let (w, h) = icon::icon_dim();
    let rgba = icon::build_icon_rgba(pct);
    Icon::from_rgba(rgba, w, h).expect("icon")
}

fn acquire_singleton() -> Option<HANDLE> {
    unsafe {
        // CreateMutexW returns a valid handle even when the mutex already exists;
        // GetLastError() distinguishes the two cases. Local\ scope = no extra privilege.
        let h = match CreateMutexW(None, true, w!("Local\\ViperTray-singleton")) {
            Ok(h) => h,
            Err(_) => return None,
        };
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(h);
            return None;
        }
        Some(h)
    }
}
