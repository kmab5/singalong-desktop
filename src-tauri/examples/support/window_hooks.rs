//! Window-thread-only lifecycle hook. No app state or borrowed callback pointers.
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::RemoteDesktop::{
        NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
    },
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            PBT_APMSUSPEND, SW_HIDE, ShowWindow, WM_NCDESTROY, WM_POWERBROADCAST,
            WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK,
        },
    },
};

const HOOK_ID: usize = 0x5347;

/// Exercise the installed message handler without locking the user's desktop.
/// This is plumbing evidence only; real OS delivery needs a manual check.
pub fn simulate_lock(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::SendMessageW;
    // SAFETY: synchronous call on the owning UI thread, no pointer payload.
    unsafe {
        SendMessageW(
            hwnd,
            WM_WTSSESSION_CHANGE,
            Some(WPARAM(WTS_SESSION_LOCK as usize)),
            Some(LPARAM(0)),
        );
    }
}

pub fn simulate_suspend(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::SendMessageW;
    // SAFETY: synchronous notification on the owning window thread, no pointers.
    unsafe {
        SendMessageW(
            hwnd,
            WM_POWERBROADCAST,
            Some(WPARAM(PBT_APMSUSPEND as usize)),
            Some(LPARAM(0)),
        );
    }
}

pub fn install(hwnd: HWND) -> windows::core::Result<()> {
    // SAFETY: called during Tauri setup on the owning window thread. Windows
    // owns the HWND lifetime; the subclass removes itself on WM_NCDESTROY.
    unsafe {
        WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION)?;
        if !SetWindowSubclass(hwnd, Some(lifecycle), HOOK_ID, 0).as_bool() {
            let error = windows::core::Error::from_win32();
            let _ = WTSUnRegisterSessionNotification(hwnd);
            return Err(error);
        }
    }
    Ok(())
}

unsafe extern "system" fn lifecycle(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    // SAFETY: all calls use the HWND supplied by Windows on its owning thread;
    // no Rust references are retained by the native callback.
    unsafe {
        if (message == WM_WTSSESSION_CHANGE && wparam.0 == WTS_SESSION_LOCK as usize)
            || (message == WM_POWERBROADCAST && wparam.0 == PBT_APMSUSPEND as usize)
        {
            let _ = ShowWindow(hwnd, SW_HIDE);
            println!("lifecycle_hide: lock_or_suspend; explicit tray recovery required");
        }
        if message == WM_NCDESTROY {
            let _ = WTSUnRegisterSessionNotification(hwnd);
            let _ = RemoveWindowSubclass(hwnd, Some(lifecycle), HOOK_ID);
        }
        DefSubclassProc(hwnd, message, wparam, lparam)
    }
}
