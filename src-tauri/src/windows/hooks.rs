use crate::runtime::Runtime;
use tauri::Manager;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::RemoteDesktop::{
        NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
    },
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND, WM_NCDESTROY,
            WM_POWERBROADCAST, WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
        },
    },
};
const HOOK: usize = 0x53474c;
struct State {
    app: tauri::AppHandle,
    locked: bool,
    suspended: bool,
}
pub fn install(hwnd: HWND, app: tauri::AppHandle) -> windows::core::Result<()> {
    // All subclass access and destruction happen on the owning window thread.
    unsafe {
        WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION)?;
        let state = Box::into_raw(Box::new(State {
            app,
            locked: false,
            suspended: false,
        }));
        if !SetWindowSubclass(hwnd, Some(callback), HOOK, state as usize).as_bool() {
            drop(Box::from_raw(state));
            let _ = WTSUnRegisterSessionNotification(hwnd);
            return Err(windows::core::Error::from_win32());
        }
    }
    Ok(())
}
unsafe extern "system" fn callback(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    unsafe {
        let state = &mut *(data as *mut State);
        let mut changed = false;
        if message == WM_WTSSESSION_CHANGE {
            if wparam.0 == WTS_SESSION_LOCK as usize {
                state.locked = true;
                changed = true;
            }
            if wparam.0 == WTS_SESSION_UNLOCK as usize {
                state.locked = false;
                changed = true;
            }
        }
        if message == WM_POWERBROADCAST {
            if wparam.0 == PBT_APMSUSPEND as usize {
                state.suspended = true;
                changed = true;
            }
            if wparam.0 == PBT_APMRESUMEAUTOMATIC as usize
                || wparam.0 == PBT_APMRESUMESUSPEND as usize
            {
                state.suspended = false;
                changed = true;
            }
        }
        if changed {
            let suspended = state.locked || state.suspended;
            if suspended {
                for window in state.app.webview_windows().values() {
                    let _ = window.hide();
                }
            }
            state.app.state::<Runtime>().suspend(suspended);
        }
        if message == WM_NCDESTROY {
            let _ = WTSUnRegisterSessionNotification(hwnd);
            let _ = RemoveWindowSubclass(hwnd, Some(callback), HOOK);
            drop(Box::from_raw(data as *mut State));
        }
        DefSubclassProc(hwnd, message, wparam, lparam)
    }
}
