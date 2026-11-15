use crate::runtime::Runtime;
use singalong_core::model::{
    LookupStatus, OverlayMode, PlaybackStatus, RuntimeSnapshot, WindowGeometry,
};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;

#[derive(Default)]
struct Policy {
    enabled: bool,
    suppressed: bool,
    visible: bool,
    system_hidden: bool,
    resume_visible: bool,
    minimum_revision: u64,
    pause_since: Option<Instant>,
    missing_since: Option<Instant>,
    song: Option<String>,
    collapsed: bool,
    docked: bool,
    applied_mode: Option<OverlayMode>,
    restore_size: Option<tauri::PhysicalSize<u32>>,
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
    .min_inner_size(160.0, 80.0)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .center()
    .build()?;
    if let Ok(settings) = app.state::<crate::storage::Storage>().settings() {
        if let Some(geometry) = settings.overlay_geometry {
            let _ = restore_geometry(&window, geometry);
            app.state::<Control>().0.lock().unwrap().applied_mode = Some(settings.overlay_mode);
        }
    }
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
        tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
            let persist = handle
                .state::<Control>()
                .0
                .try_lock()
                .is_ok_and(|policy| !policy.collapsed && !policy.docked);
            if persist && !tracked.is_minimized().unwrap_or(false) {
                if let Some(geometry) = remember(&tracked) {
                    handle.state::<Runtime>().geometry(geometry);
                }
            }
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
    let was_docked;
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
        was_docked = policy.docked;
        policy.manual(visible);
        if visible {
            policy.collapsed = false;
            policy.docked = false;
            if was_docked {
                policy.restore_size = None;
            }
            policy.pause_since = None;
            policy.missing_since = None;
            policy.applied_mode = None;
        }
        policy.resume_visible = false;
    }
    if visible {
        if was_docked {
            resize_mode(&window, app.state::<Runtime>().snapshot().overlay_mode)?;
        }
        window.unminimize()?;
        window.show()?;
    } else {
        window.hide()?;
    }
    if visible {
        app.state::<Runtime>().docked(false);
        app.state::<Runtime>().collapsed(false);
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
    let selected = snapshot
        .sessions
        .iter()
        .find(|s| Some(&s.session_id) == snapshot.selected_session_id.as_ref());
    let song = selected
        .and_then(|s| s.recording.as_ref())
        .map(|r| r.id.clone());
    if song != policy.song {
        policy.song = song;
        policy.pause_since = None;
        policy.missing_since = None;
    }
    let paused = selected.is_some_and(|s| s.status == PlaybackStatus::Paused);
    let missing = selected.and_then(|s| s.recording.as_ref()).is_some()
        && snapshot.reader.version.is_none()
        && matches!(
            snapshot.reader.status,
            LookupStatus::Failed
                | LookupStatus::NotFound
                | LookupStatus::Offline
                | LookupStatus::InsufficientMetadata
        );
    if paused {
        policy.pause_since.get_or_insert_with(Instant::now);
    } else {
        policy.pause_since = None;
    }
    if missing {
        policy.missing_since.get_or_insert_with(Instant::now);
    } else {
        policy.missing_since = None;
    }
    let collapse = policy
        .pause_since
        .is_some_and(|at| at.elapsed() >= Duration::from_secs(15))
        || policy
            .missing_since
            .is_some_and(|at| at.elapsed() >= Duration::from_secs(8));
    if policy.visible && !policy.suppressed && !policy.docked {
        let next = if collapse {
            true
        } else if !paused && !missing {
            false
        } else {
            policy.collapsed
        };
        if next != policy.collapsed {
            if next {
                policy.restore_size = app
                    .get_webview_window("overlay")
                    .and_then(|w| w.inner_size().ok());
            }
            policy.collapsed = next;
            app.state::<Runtime>().collapsed(next);
        }
    }
    let mode = if policy.collapsed {
        OverlayMode::Minimal
    } else {
        snapshot.overlay_mode
    };
    if !policy.docked && policy.applied_mode != Some(mode) {
        if let Some(window) = app.get_webview_window("overlay") {
            if !policy.collapsed {
                if let Some(size) = policy.restore_size.take() {
                    let _ = window.set_size(size);
                    let _ = clamp(&window);
                } else {
                    let _ = resize_mode(&window, mode);
                }
            } else {
                let _ = resize_mode(&window, mode);
            }
        }
        policy.applied_mode = Some(mode);
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
        policy.pause_since = None;
        policy.missing_since = None;
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

fn resize_mode(window: &tauri::WebviewWindow, mode: OverlayMode) -> tauri::Result<()> {
    window.set_decorations(true)?;
    window.set_min_size(Some(tauri::LogicalSize::new(160.0, 80.0)))?;
    let (width, height) = match mode {
        OverlayMode::Compact => (440.0, 360.0),
        OverlayMode::Expanded => (560.0, 720.0),
        OverlayMode::Minimal => (320.0, 110.0),
    };
    window.set_size(tauri::LogicalSize::new(width, height))?;
    clamp(window)
}
fn remember(window: &tauri::WebviewWindow) -> Option<WindowGeometry> {
    let monitor = window.current_monitor().ok().flatten()?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let position = window.outer_position().ok()?;
    let size = window.inner_size().ok()?.to_logical::<f64>(scale);
    Some(WindowGeometry {
        monitor: monitor.name().cloned(),
        x: (f64::from(position.x) - f64::from(area.position.x)) / scale,
        y: (f64::from(position.y) - f64::from(area.position.y)) / scale,
        width: size.width,
        height: size.height,
    })
}
fn restore_geometry(window: &tauri::WebviewWindow, value: WindowGeometry) -> tauri::Result<()> {
    if ![value.x, value.y, value.width, value.height]
        .iter()
        .all(|v| v.is_finite())
    {
        return Ok(());
    }
    let monitors = window.available_monitors()?;
    if let Some(monitor) = monitors
        .iter()
        .find(|m| m.name() == value.monitor.as_ref())
        .or_else(|| monitors.first())
    {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        window.set_position(tauri::PhysicalPosition::new(
            (f64::from(area.position.x) + value.x * scale) as i32,
            (f64::from(area.position.y) + value.y * scale) as i32,
        ))?;
        window.set_size(tauri::PhysicalSize::new(
            (value.width.clamp(160.0, 10000.0) * scale) as u32,
            (value.height.clamp(80.0, 10000.0) * scale) as u32,
        ))?;
    }
    clamp(window)
}

fn clamp(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let position = window.outer_position()?;
    let size = window.outer_size()?;
    let monitors = window.available_monitors()?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let area = monitor.work_area();
            let x = i64::from(position.x) + i64::from(size.width) / 2;
            let y = i64::from(position.y) + i64::from(size.height) / 2;
            x >= i64::from(area.position.x)
                && y >= i64::from(area.position.y)
                && x < i64::from(area.position.x) + i64::from(area.size.width)
                && y < i64::from(area.position.y) + i64::from(area.size.height)
        })
        .or_else(|| monitors.first());
    if let Some(monitor) = monitor {
        let area = monitor.work_area();
        if size.width > area.size.width || size.height > area.size.height {
            let inner = window.inner_size()?;
            window.set_size(tauri::PhysicalSize::new(
                inner
                    .width
                    .saturating_sub(size.width.saturating_sub(area.size.width))
                    .max(160),
                inner
                    .height
                    .saturating_sub(size.height.saturating_sub(area.size.height))
                    .max(80),
            ))?;
        }
        let size = window.outer_size()?;
        let x = i64::from(position.x).clamp(
            i64::from(area.position.x),
            i64::from(area.position.x) + i64::from(area.size.width.saturating_sub(size.width)),
        );
        let y = i64::from(position.y).clamp(
            i64::from(area.position.y),
            i64::from(area.position.y) + i64::from(area.size.height.saturating_sub(size.height)),
        );
        window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32))?;
    }
    Ok(())
}
pub fn mode(app: &tauri::AppHandle, value: OverlayMode) -> tauri::Result<()> {
    if let Some(control) = app.try_state::<Control>() {
        let mut policy = control.0.lock().unwrap();
        policy.collapsed = false;
        policy.pause_since = None;
        policy.missing_since = None;
        policy.applied_mode = Some(value);
        policy.docked = false;
        policy.restore_size = None;
    }
    app.state::<Runtime>().collapsed(false);
    app.state::<Runtime>().docked(false);
    if let Some(window) = app.get_webview_window("overlay") {
        resize_mode(&window, value)?;
    }
    Ok(())
}
pub fn click_through(app: &tauri::AppHandle, enabled: bool) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("overlay") {
        window.set_ignore_cursor_events(enabled)?;
    }
    Ok(())
}
pub fn dock(app: &tauri::AppHandle, side: &str) -> tauri::Result<()> {
    if let Some(control) = app.try_state::<Control>() {
        control.0.lock().unwrap().docked = true;
    }
    app.state::<Runtime>().docked(true);
    if let Some(window) = app.get_webview_window("overlay") {
        window.set_decorations(false)?;
        window.set_min_size(Some(tauri::LogicalSize::new(72.0, 72.0)))?;
        window.set_size(tauri::LogicalSize::new(80.0, 80.0))?;
        if let Some(monitor) = window.current_monitor()?.or(window.primary_monitor()?) {
            let area = monitor.work_area();
            let size = window.outer_size()?;
            let position = window.outer_position()?;
            let x = if side == "left" {
                area.position.x
            } else {
                (i64::from(area.position.x) + i64::from(area.size.width.saturating_sub(size.width)))
                    as i32
            };
            window.set_position(tauri::PhysicalPosition::new(x, position.y))?;
            clamp(&window)?;
        }
    }
    Ok(())
}
pub fn recover(app: &tauri::AppHandle) -> tauri::Result<()> {
    click_through(app, false)?;
    if let Some(window) = app.get_webview_window("overlay") {
        window.center()?;
        resize_mode(&window, OverlayMode::Compact)?;
    }
    mode(app, OverlayMode::Compact)?;
    manual(app, true)
}
