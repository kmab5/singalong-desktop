//! Native Tauri feasibility harness; no JavaScript bridge or remote content.
// build.rs explicitly links the manifest resource into this example target.
#[cfg(windows)]
#[path = "support/window_hooks.rs"]
mod window_hooks;
use tauri::{
    Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

fn recover(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("probe") {
        window.set_ignore_cursor_events(false)?;
        window.unminimize()?;
        window.center()?;
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

fn main() {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    tauri::Builder::default()
        .setup(move |app| {
            let window = WebviewWindowBuilder::new(app, "probe", WebviewUrl::App("index.html".into()))
                .title("Singalong · P0 window probe")
                .inner_size(440.0, 300.0)
                .min_inner_size(300.0, 180.0)
                .always_on_top(true)
                .resizable(true)
                .center()
                .build()?;
            #[cfg(windows)]
            window_hooks::install(window.hwnd()?)?;
            let restore = MenuItem::with_id(app, "restore", "Show / reset position", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
            let pass = MenuItem::with_id(app, "pass", "Enable click-through (restore to disable)", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit probe", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&restore, &hide, &pass, &quit])?;
            // Original solid-color test icon. No external assets or licenses.
            let rgba = [110, 180, 245, 255].repeat(32 * 32);
            TrayIconBuilder::with_id("probe-tray")
                .icon(tauri::image::Image::new_owned(rgba, 32, 32))
                .tooltip("Singalong feasibility probe")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let result = match event.id.as_ref() {
                        "restore" => recover(app),
                        "hide" => app.get_webview_window("probe").map_or(Ok(()), |w| w.hide()),
                        "pass" => app.get_webview_window("probe").map_or(Ok(()), |w| w.set_ignore_cursor_events(true)),
                        "quit" => { app.exit(0); Ok(()) },
                        _ => Ok(()),
                    };
                    if let Err(error) = result { eprintln!("Window operation failed: {error}"); }
                })
                .build(app)?;
            let close_window = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Err(error) = close_window.hide() { eprintln!("Hide failed: {error}"); }
                }
            });
            println!("window_created: solid background, resizable, always-on-top");
            if smoke {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    let controller = handle.clone();
                    handle.run_on_main_thread(move || {
                        let result = (|| -> tauri::Result<()> {
                            let window = controller.get_webview_window("probe").expect("probe exists");
                            window.hide()?;
                            assert!(!window.is_visible()?);
                            window.set_ignore_cursor_events(true)?;
                            recover(&controller)?;
                            assert!(window.is_visible()?);
                            assert!(window.is_always_on_top()?);
                            window.set_position(tauri::PhysicalPosition::new(-32000, -32000))?;
                            recover(&controller)?;
                            let position = window.outer_position()?;
                            let monitors = window.available_monitors()?;
                            assert!(monitors.iter().any(|monitor| {
                                let area = monitor.work_area();
                                position.x >= area.position.x && position.y >= area.position.y &&
                                i64::from(position.x) < i64::from(area.position.x) + i64::from(area.size.width) &&
                                i64::from(position.y) < i64::from(area.position.y) + i64::from(area.size.height)
                            }), "Recovery must return the title bar to an available work area");
                            window.set_size(tauri::LogicalSize::new(520.0, 360.0))?;
                            assert!(window.inner_size()?.width > 0);
                            println!("geometry_passed: offscreen recovery, resize; monitors={}", monitors.len());
                            #[cfg(windows)]
                            {
                                window_hooks::simulate_lock(window.hwnd()?);
                                assert!(!window.is_visible()?);
                                recover(&controller)?;
                                window_hooks::simulate_suspend(window.hwnd()?);
                                assert!(!window.is_visible()?);
                                recover(&controller)?;
                            }
                            println!("smoke_passed: hide, visibility, click-through calls, recovery, topmost, simulated lock message; not manual compatibility evidence");
                            Ok(())
                        })();
                        if let Err(error) = result { eprintln!("smoke_failed: {error}"); controller.exit(1); }
                        else { controller.exit(0); }
                    }).expect("schedule smoke check");
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!("tauri.probe.conf.json"))
        .expect("run window probe");
}
