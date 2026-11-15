use crate::runtime::Runtime;
use singalong_core::model::RuntimeAction;
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
#[cfg(windows)]
pub mod hooks;
pub mod overlay;
pub fn show(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        if let Some(runtime) = app.try_state::<Runtime>() {
            runtime.main_visible(true);
        }
    }
}
pub fn tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Singalong", true, None::<&str>)?;
    let start = MenuItem::with_id(app, "start", "Start tracking", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Stop tracking", true, None::<&str>)?;
    let overlay = MenuItem::with_id(app, "overlay", "Show floating lyrics", true, None::<&str>)?;
    let reset = MenuItem::with_id(
        app,
        "reset",
        "Reset main window position",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit Singalong", true, None::<&str>)?;
    let hide_overlay = MenuItem::with_id(
        app,
        "hide-overlay",
        "Hide floating lyrics",
        true,
        None::<&str>,
    )?;
    let recover = MenuItem::with_id(
        app,
        "recover-overlay",
        "Recover floating lyrics",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &start,
            &stop,
            &overlay,
            &hide_overlay,
            &recover,
            &reset,
            &quit,
        ],
    )?;
    TrayIconBuilder::with_id("singalong-tray")
        .icon(tauri::image::Image::new_owned(
            [200, 170, 255, 255].repeat(32 * 32),
            32,
            32,
        ))
        .tooltip("Singalong — lyrics companion")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "overlay" => {
                let _ = overlay::manual(app, true);
            }
            "hide-overlay" => {
                let _ = overlay::manual(app, false);
            }
            "recover-overlay" => {
                if let Some(w) = app.get_webview_window("overlay") {
                    let _ = w.set_ignore_cursor_events(false);
                    let _ = w.center();
                }
                let _ = overlay::manual(app, true);
            }
            "reset" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.center();
                }
                show_main(app);
            }
            "start" | "stop" => {
                let runtime = app.state::<Runtime>().inner().clone();
                let enabled = event.id.as_ref() == "start";
                tauri::async_runtime::spawn_blocking(move || {
                    let _ = runtime.action(RuntimeAction::Tracking { enabled });
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
fn show_main(app: &tauri::AppHandle) {
    show(app);
}
