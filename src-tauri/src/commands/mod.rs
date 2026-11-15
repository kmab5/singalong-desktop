#[cfg(feature = "desktop")]
use singalong_core::model::{Capabilities, RuntimeSnapshot};
use singalong_core::model::{CommandFailure, FailureCode};

#[derive(Clone, Copy)]
pub enum Command {
    Snapshot,
    Diagnostics,
    Control,
}

pub fn authorize(label: &str, command: Command) -> Result<(), CommandFailure> {
    if label == "main" || (label == "overlay" && matches!(command, Command::Snapshot)) {
        Ok(())
    } else {
        Err(CommandFailure {
            code: FailureCode::PermissionDenied,
            message: "This window cannot perform that action.".into(),
            retry_after_ms: None,
        })
    }
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub fn get_snapshot(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, crate::runtime::Runtime>,
) -> Result<RuntimeSnapshot, CommandFailure> {
    authorize(window.label(), Command::Snapshot)?;
    Ok(state.snapshot())
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub fn get_diagnostics(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, crate::runtime::Runtime>,
) -> Result<Capabilities, CommandFailure> {
    authorize(window.label(), Command::Diagnostics)?;
    Ok(state.snapshot().capabilities)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlay_reads_state_but_cannot_use_main_only_commands() {
        assert!(authorize("overlay", Command::Snapshot).is_ok());
        assert!(authorize("overlay", Command::Diagnostics).is_err());
        assert!(authorize("main", Command::Diagnostics).is_ok());
        assert!(authorize("main", Command::Control).is_ok());
        assert!(authorize("overlay", Command::Control).is_err());
        for label in ["", "Main", "probe", "main-extra"] {
            assert!(authorize(label, Command::Snapshot).is_err());
            assert!(authorize(label, Command::Diagnostics).is_err());
        }
    }
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn runtime_action(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, crate::runtime::Runtime>,
    action: singalong_core::model::RuntimeAction,
) -> Result<(), CommandFailure> {
    use singalong_core::model::RuntimeAction;
    use tauri::Manager;
    use tauri_plugin_autostart::ManagerExt;
    authorize(window.label(), Command::Control)?;
    ensure_open(window.app_handle())?;
    if let RuntimeAction::ShowOverlay { visible } = action {
        let app = window.app_handle().clone();
        return tauri::async_runtime::spawn_blocking(move || {
            crate::windows::dispatch(&app, move |app| {
                crate::windows::overlay::manual(app, visible)
            })
        })
        .await
        .map_err(|_| failure("Could not change floating reader visibility."))?
        .map_err(|e| failure(&e));
    }
    if matches!(action, RuntimeAction::Quit) {
        window.app_handle().exit(0);
        return Ok(());
    }
    let runtime = state.inner().clone();
    let overlay_preference = if let RuntimeAction::AutoOverlay { enabled } = &action {
        Some(*enabled)
    } else {
        None
    };
    let hide = matches!(action, RuntimeAction::CloseToTray);
    let native_window = window.clone();
    tauri::async_runtime::spawn_blocking(move || {
        static STARTUP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let startup = if let RuntimeAction::Startup { enabled } = action {
            Some(enabled)
        } else {
            None
        };
        let _guard = if startup.is_some() {
            Some(
                STARTUP_LOCK
                    .lock()
                    .map_err(|_| failure("Startup preference is unavailable."))?,
            )
        } else {
            None
        };
        let previous = runtime.snapshot().start_at_sign_in;
        if let Some(enabled) = startup {
            let manager = native_window.autolaunch();
            (if enabled {
                manager.enable()
            } else {
                manager.disable()
            })
            .map_err(|_| failure("Could not change Windows sign-in startup."))?;
        }
        if let Err(error) = runtime.action(action) {
            if startup.is_some() {
                let manager = native_window.autolaunch();
                let _ = if previous {
                    manager.enable()
                } else {
                    manager.disable()
                };
            }
            return Err(failure(&error));
        }
        Ok(())
    })
    .await
    .map_err(|_| failure("The tracking operation could not finish."))??;
    if let Some(enabled) = overlay_preference {
        let app = window.app_handle().clone();
        let handle = app.clone();
        app.run_on_main_thread(move || crate::windows::overlay::preference(&handle, enabled))
            .map_err(|_| failure("Could not update floating reader preference."))?;
    }
    if hide {
        window
            .hide()
            .map_err(|_| failure("Could not hide the main window."))?;
        state.main_visible(false);
    }
    Ok(())
}

#[cfg(feature = "desktop")]
fn failure(message: &str) -> CommandFailure {
    CommandFailure {
        code: FailureCode::InvalidInput,
        message: message.into(),
        retry_after_ms: None,
    }
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn product_command(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, crate::runtime::Runtime>,
    command: singalong_core::model::ProductCommand,
) -> Result<singalong_core::model::ProductReply, CommandFailure> {
    use singalong_core::model::{OverlayMode, ProductCommand as C, ProductReply};
    use tauri::Manager;
    ensure_open(window.app_handle())?;
    let overlay = window.label() == "overlay";
    if overlay {
        if !matches!(
            &command,
            C::Reader { .. }
                | C::Overlay { .. }
                | C::ClickThrough { .. }
                | C::Dock { .. }
                | C::ResetOverlay
                | C::HideOverlay
        ) {
            return Err(failure("This window cannot perform that action."));
        }
    } else {
        authorize(window.label(), Command::Control)?;
    }
    let runtime = state.inner().clone();
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let command = match command {
            C::BackupPreview { limits } => {
                return app
                    .state::<crate::backup::Manager>()
                    .preview(limits)
                    .map(|state| ProductReply::Backup { state })
                    .map_err(|e| failure(&e));
            }
            C::BackupApply { token, settings } => {
                return app
                    .state::<crate::backup::Manager>()
                    .apply(token, settings)
                    .map(|state| ProductReply::Backup { state })
                    .map_err(|e| failure(&e));
            }
            C::BackupExport { limits } => {
                return app
                    .state::<crate::backup::Manager>()
                    .export(limits)
                    .map(|state| ProductReply::Backup { state })
                    .map_err(|e| failure(&e));
            }
            C::BackupPoll => {
                return Ok(ProductReply::Backup {
                    state: app.state::<crate::backup::Manager>().poll(),
                });
            }
            C::BackupCancel => {
                let manager = app.state::<crate::backup::Manager>();
                manager.cancel();
                return Ok(ProductReply::Backup {
                    state: manager.poll(),
                });
            }
            C::DiagnosticsPreview => {
                return app
                    .state::<crate::diagnostics::Reports>()
                    .preview(&runtime.snapshot())
                    .map(|report| ProductReply::Diagnostics { report })
                    .map_err(|e| failure(&e));
            }
            C::DiagnosticsExport => {
                ensure_open(&app)?;
                let epoch = app.state::<crate::Shutdown>().epoch.clone();
                let cancel = crate::platform::Cancellation {
                    expected: epoch.load(std::sync::atomic::Ordering::SeqCst),
                    epoch,
                };
                ensure_open(&app)?;
                use std::io::Write;
                let bytes = app
                    .state::<crate::diagnostics::Reports>()
                    .reviewed()
                    .map_err(|e| failure(&e))?;
                let Some(path) = crate::files::choose(true, true, Some(cancel.clone()))
                    .map_err(|e| failure(&e))?
                else {
                    return Ok(ProductReply::Done);
                };
                crate::files::atomic_write(
                    &path,
                    |file| {
                        file.write_all(&bytes)
                            .map_err(|_| "Could not export diagnostics.".into())
                    },
                    || cancel.cancelled(),
                )
                .map_err(|e| failure(&e))?;
                return Ok(ProductReply::Done);
            }
            C::HideOverlay => {
                crate::windows::dispatch(&app, |app| crate::windows::overlay::manual(app, false))
                    .map_err(|_| failure("Could not hide floating lyrics."))?;
                return Ok(ProductReply::Done);
            }
            C::Overlay { mode } => {
                crate::windows::dispatch(&app, move |app| crate::windows::overlay::mode(app, mode))
                    .map_err(|_| failure("Could not resize floating lyrics."))?;
                C::Overlay { mode }
            }
            C::ClickThrough { enabled } => {
                crate::windows::dispatch(&app, move |app| {
                    crate::windows::overlay::click_through(app, enabled)
                })
                .map_err(|_| failure("Could not change mouse interaction."))?;
                C::ClickThrough { enabled }
            }
            C::Dock { side } => {
                if side != "left" && side != "right" {
                    return Err(failure("Choose the left or right screen edge."));
                }
                crate::windows::dispatch(&app, |app| {
                    crate::windows::overlay::mode(app, OverlayMode::Minimal)
                })
                .map_err(|_| failure("Could not dock floating lyrics."))?;
                crate::windows::dispatch(&app, move |app| {
                    crate::windows::overlay::dock(app, &side)
                })
                .map_err(|_| failure("Could not dock floating lyrics."))?;
                C::Overlay {
                    mode: OverlayMode::Minimal,
                }
            }
            C::ResetOverlay => {
                crate::windows::dispatch(&app, crate::windows::overlay::recover)
                    .map_err(|_| failure("Could not recover floating lyrics."))?;
                runtime
                    .product_for(C::ClickThrough { enabled: false }, overlay)
                    .map_err(|e| failure(&e))?;
                C::Overlay {
                    mode: OverlayMode::Compact,
                }
            }
            C::OpenReleases => {
                open_releases()?;
                return Ok(ProductReply::Done);
            }
            command => command,
        };
        runtime
            .product_for(command, overlay)
            .map_err(|error| failure(&error))
    })
    .await
    .map_err(|_| failure("The operation could not finish."))?
}
#[cfg(all(feature = "desktop", windows))]
fn open_releases() -> Result<(), CommandFailure> {
    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        core::w,
    };
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            w!("https://github.com/kmab5/singalong-desktop/releases"),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err(failure("Could not open the release page."))
    } else {
        Ok(())
    }
}
#[cfg(all(feature = "desktop", not(windows)))]
fn open_releases() -> Result<(), CommandFailure> {
    Err(failure(
        "Open github.com/kmab5/singalong-desktop/releases in your browser.",
    ))
}

#[cfg(feature = "desktop")]
fn ensure_open(app: &tauri::AppHandle) -> Result<(), CommandFailure> {
    use tauri::Manager;
    if app
        .state::<crate::Shutdown>()
        .started
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        Err(failure("Singalong is quitting."))
    } else {
        Ok(())
    }
}
