#[cfg(feature = "desktop")]
use singalong_core::model::Capabilities;
use singalong_core::model::{CommandFailure, FailureCode, RuntimeSnapshot};

#[derive(Clone, Copy)]
pub enum Command {
    Snapshot,
    Diagnostics,
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

pub struct RuntimeState(pub RuntimeSnapshot);
impl Default for RuntimeState {
    fn default() -> Self {
        Self(RuntimeSnapshot::foundation())
    }
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub fn get_snapshot(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, RuntimeState>,
) -> Result<RuntimeSnapshot, CommandFailure> {
    authorize(window.label(), Command::Snapshot)?;
    Ok(state.0.clone())
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub fn get_diagnostics(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, RuntimeState>,
) -> Result<Capabilities, CommandFailure> {
    authorize(window.label(), Command::Diagnostics)?;
    Ok(state.0.capabilities.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlay_reads_state_but_cannot_use_main_only_commands() {
        assert!(authorize("overlay", Command::Snapshot).is_ok());
        assert!(authorize("overlay", Command::Diagnostics).is_err());
        assert!(authorize("main", Command::Diagnostics).is_ok());
        for label in ["", "Main", "probe", "main-extra"] {
            assert!(authorize(label, Command::Snapshot).is_err());
            assert!(authorize(label, Command::Diagnostics).is_err());
        }
    }
}
