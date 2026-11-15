use crate::runtime::Runtime;
use singalong_core::model::{PlaybackStatus, RuntimeSnapshot};
use std::sync::Mutex;
use tauri::Manager;

#[derive(Default)]
struct Policy {
    enabled: bool,
    suppressed: bool,
    visible: bool,
    system_hidden: bool,
    resume_visible: bool,
    minimum_revision: u64,
}
impl Policy {
    fn preference(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    fn manual(&mut self, visible: bool) {
        self.visible = visible;
        self.suppressed = !visible;
    }
    fn automatic(&mut self, eligible: bool) -> bool {
        if !self.system_hidden
            && !self.suppressed
            && (self.resume_visible || (self.enabled && eligible))
        {
            self.resume_visible = false;
            self.visible = true;
        }
        self.visible
    }
}
struct Control(Mutex<Policy>);
pub fn install(app: &tauri::App) -> tauri::Result<()> {
    app.manage(Control(Mutex::new(Policy::default())));
    let window = tauri::WebviewWindowBuilder::new(
        app,
        "overlay",
        tauri::WebviewUrl::App("index.html?overlay=1".into()),
    )
    .title("Singalong · Floating lyrics")
    .inner_size(440.0, 540.0)
    .min_inner_size(320.0, 240.0)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .center()
    .build()?;
    let handle = app.handle().clone();
    let tracked = window.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = manual(&handle, false);
        }
        tauri::WindowEvent::Resized(_) if tracked.is_minimized().unwrap_or(false) => {
            let _ = manual(&handle, false);
        }
        _ => {}
    });
    Ok(())
}
pub fn manual(app: &tauri::AppHandle, visible: bool) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("overlay") else {
        return Ok(());
    };
    let state = app.state::<Control>();
    {
        let mut policy = state.0.lock().unwrap();
        policy.minimum_revision = app
            .state::<Runtime>()
            .snapshot()
            .revision
            .value()
            .saturating_add(1);
        if visible && policy.system_hidden {
            return Ok(());
        }
        policy.manual(visible);
        policy.resume_visible = false;
    }
    if visible {
        window.unminimize()?;
        window.show()?;
    } else {
        window.hide()?;
    }
    app.state::<Runtime>().overlay_visible(visible);
    Ok(())
}
pub fn sync(app: &tauri::AppHandle, snapshot: &RuntimeSnapshot) {
    let Some(control) = app.try_state::<Control>() else {
        return;
    };
    let current = app.state::<Runtime>().snapshot();
    if current.revision != snapshot.revision || current.generation != snapshot.generation {
        return;
    }
    let eligible = snapshot.tracking_enabled
        && snapshot.sessions.iter().any(|s| {
            Some(&s.session_id) == snapshot.selected_session_id.as_ref()
                && s.status == PlaybackStatus::Playing
                && s.recording.is_some()
        });
    let mut policy = control.0.lock().unwrap();
    if policy.system_hidden || snapshot.revision.value() < policy.minimum_revision {
        return;
    }
    policy.preference(snapshot.auto_overlay);
    let previous = policy.visible;
    let visible = policy.automatic(eligible);
    if visible
        && !previous
        && let Some(window) = app.get_webview_window("overlay")
    {
        if window.show().is_ok() {
            app.state::<Runtime>().overlay_visible(true);
        } else {
            policy.visible = false;
        }
    }
}

/// Only an explicit preference action may release manual suppression.
pub fn preference(app: &tauri::AppHandle, enabled: bool) {
    if let Some(control) = app.try_state::<Control>() {
        let mut policy = control.0.lock().unwrap();
        if enabled {
            policy.suppressed = false;
        }
        policy.enabled = enabled;
    }
    sync(app, &app.state::<Runtime>().snapshot());
}
/// Lock/suspend is temporary and never impersonates a user's Hide command.
pub fn system_hidden(app: &tauri::AppHandle, hidden: bool) {
    let Some(control) = app.try_state::<Control>() else {
        return;
    };
    {
        let mut policy = control.0.lock().unwrap();
        if hidden && !policy.system_hidden {
            policy.resume_visible = policy.visible;
            policy.visible = false;
        }
        policy.system_hidden = hidden;
        policy.minimum_revision = app
            .state::<Runtime>()
            .snapshot()
            .revision
            .value()
            .saturating_add(1);
    }
    if hidden {
        if let Some(window) = app.get_webview_window("overlay") {
            let _ = window.hide();
        }
        app.state::<Runtime>().overlay_visible(false);
    }
}

#[cfg(test)]
mod tests {
    use super::Policy;
    #[test]
    fn automatic_display_is_opt_in_and_manual_hide_wins() {
        let mut p = Policy::default();
        assert!(!p.automatic(true));
        p.preference(true);
        assert!(p.automatic(true));
        p.manual(false);
        assert!(!p.automatic(true));
        p.preference(true);
        assert!(!p.automatic(true));
        p.manual(true);
        assert!(p.automatic(false));
        p.manual(false);
        p.preference(false);
        p.preference(true);
        assert!(!p.automatic(true));
        p.suppressed = false;
        assert!(p.automatic(true));
    }
}
