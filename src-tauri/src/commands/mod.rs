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
    if matches!(action, RuntimeAction::Quit) {
        window.app_handle().exit(0);
        return Ok(());
    }
    let runtime = state.inner().clone();
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
    if hide {
        window
            .hide()
            .map_err(|_| failure("Could not hide the main window."))?;
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
