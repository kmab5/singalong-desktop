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
    let mouse = MenuItem::with_id(
        app,
        "overlay-mouse",
        "Toggle floating click-through",
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
            &mouse,
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
                let _ = overlay::recover(app);
                let runtime = app.state::<Runtime>().inner().clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let _ = runtime.product(singalong_core::model::ProductCommand::ClickThrough {
                        enabled: false,
                    });
                    let _ = runtime.product(singalong_core::model::ProductCommand::Overlay {
                        mode: singalong_core::model::OverlayMode::Compact,
                    });
                });
            }
            "overlay-mouse" => {
                let runtime = app.state::<Runtime>().inner().clone();
                let enabled = !runtime.snapshot().click_through;
                if overlay::click_through(app, enabled).is_ok() {
                    tauri::async_runtime::spawn_blocking(move || {
                        let _ =
                            runtime.product(singalong_core::model::ProductCommand::ClickThrough {
                                enabled,
                            });
                    });
                }
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

/// Called from command worker threads only. Keep policy locks and window calls on the UI thread.
pub fn dispatch(
    app: &tauri::AppHandle,
    operation: impl FnOnce(&tauri::AppHandle) -> tauri::Result<()> + Send + 'static,
) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        if handle
            .try_state::<crate::Shutdown>()
            .is_some_and(|s| s.started.load(std::sync::atomic::Ordering::SeqCst))
        {
            let _ = tx.send(Err("Singalong is quitting.".into()));
            return;
        }
        let result =
            operation(&handle).map_err(|_| "Could not update the native window.".to_string());
        let _ = tx.send(result);
    })
    .map_err(|_| "Native window unavailable.")?;
    rx.recv()
        .map_err(|_| "Native window operation interrupted.")?
}
