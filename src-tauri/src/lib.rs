//! Application construction and shared P0 probe utilities.
#[cfg(feature = "desktop")]
mod backup;
pub mod commands;
#[cfg(feature = "desktop")]
mod diagnostics;
#[cfg(feature = "desktop")]
mod files;
pub mod platform;
pub mod providers;
pub mod runtime;
pub mod storage;

#[cfg(feature = "desktop")]
mod windows;
#[cfg(feature = "desktop")]
#[derive(Default)]
pub(crate) struct Shutdown {
    pub started: std::sync::atomic::AtomicBool,
    pub finished: std::sync::atomic::AtomicBool,
    pub epoch: std::sync::Arc<std::sync::atomic::AtomicU64>,
}
#[cfg(feature = "desktop")]
pub fn run() {
    use tauri::{Emitter, Manager};
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            windows::show(app)
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--tray"]),
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_diagnostics,
            commands::product_command,
            commands::runtime_action
        ])
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            let db = storage::Storage::open(app_data.join("library.sqlite"))?;
            let handle = app.handle().clone();
            let runtime = runtime::Runtime::native(
                db.clone(),
                std::sync::Arc::new(move |snapshot| {
                    let app = handle.clone();
                    let _ = handle.run_on_main_thread(move || {
                        if app
                            .try_state::<Shutdown>()
                            .is_some_and(|s| s.started.load(std::sync::atomic::Ordering::SeqCst))
                        {
                            return;
                        }
                        for window in app.webview_windows().values() {
                            if window.is_visible().unwrap_or(false)
                                && !window.is_minimized().unwrap_or(false)
                            {
                                let _ = window.emit("runtime-state", &snapshot);
                            }
                        }
                        windows::overlay::sync(&app, &snapshot);
                    });
                }),
            )
            .map_err(std::io::Error::other)?;
            app.manage(Shutdown::default());
            app.manage(runtime.clone());
            app.manage(diagnostics::Reports::default());
            app.manage(backup::Manager::new(db.clone(), runtime.clone()));
            app.manage(db.clone());
            runtime.main_visible(!std::env::args().any(|arg| arg == "--tray"));
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("Singalong · Development")
            .inner_size(1120.0, 760.0)
            .min_inner_size(620.0, 480.0)
            .center()
            .visible(!std::env::args().any(|arg| arg == "--tray"))
            .build()?;
            windows::tray(app)?;
            windows::overlay::install(app)?;
            #[cfg(windows)]
            windows::hooks::install(window.hwnd()?, app.handle().clone())?;
            let close_window = window.clone();
            window.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Resized(_)) {
                    runtime.main_visible(
                        close_window.is_visible().unwrap_or(false)
                            && !close_window.is_minimized().unwrap_or(false),
                    );
                }
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    // Settings read on a blocking task; the window event thread stays responsive.
                    let db = db.clone();
                    let runtime = runtime.clone();
                    let window = close_window.clone();
                    tauri::async_runtime::spawn_blocking(move || match db.settings() {
                        Ok(settings) if settings.close_notice_ack => {
                            if window.hide().is_ok() {
                                runtime.main_visible(false);
                            }
                        }
                        _ => {
                            runtime.close_notice();
                            let _ = window.show();
                            runtime.main_visible(true);
                        }
                    });
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("build Singalong")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                use std::sync::atomic::Ordering;
                let shutdown = app.state::<Shutdown>();
                if !shutdown.finished.load(Ordering::SeqCst) {
                    api.prevent_exit();
                    if !shutdown.started.swap(true, Ordering::SeqCst) {
                        shutdown.epoch.fetch_add(1, Ordering::SeqCst);
                        app.state::<backup::Manager>().begin_shutdown();
                        app.state::<runtime::Runtime>().begin_shutdown();
                        let app = app.clone();
                        tauri::async_runtime::spawn_blocking(move || {
                            app.state::<backup::Manager>().shutdown();
                            app.state::<runtime::Runtime>().shutdown();
                            app.state::<Shutdown>()
                                .finished
                                .store(true, Ordering::SeqCst);
                            app.exit(0);
                        });
                    }
                }
            }
        });
}

/// WinRT TimeSpan uses signed 100 ns ticks. Invalid positions remain unavailable.
pub fn position_ms(ticks: i64) -> Option<u64> {
    u64::try_from(ticks).ok().map(|ticks| ticks / 10_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_position_is_unavailable_instead_of_wrapping() {
        assert_eq!(position_ms(-10_000), None);
    }

    #[test]
    fn converts_100ns_ticks_without_rounding_into_the_future() {
        assert_eq!(position_ms(0), Some(0));
        assert_eq!(position_ms(12_349_999), Some(1234));
        assert_eq!(position_ms(i64::MAX), Some(922_337_203_685_477));
    }
}
