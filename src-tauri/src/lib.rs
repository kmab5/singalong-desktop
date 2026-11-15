//! Application construction and shared P0 probe utilities.
pub mod commands;
pub mod platform;
pub mod providers;
pub mod runtime;
pub mod storage;

#[cfg(feature = "desktop")]
mod windows;
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
                    let _ = handle.emit("runtime-state", snapshot);
                }),
            )
            .map_err(std::io::Error::other)?;
            app.manage(runtime.clone());
            app.manage(db.clone());
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
            #[cfg(windows)]
            windows::hooks::install(window.hwnd()?, app.handle().clone())?;
            let close_window = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    // Settings read on a blocking task; the window event thread stays responsive.
                    let db = db.clone();
                    let runtime = runtime.clone();
                    let window = close_window.clone();
                    tauri::async_runtime::spawn_blocking(move || match db.settings() {
                        Ok(settings) if settings.close_notice_ack => {
                            let _ = window.hide();
                        }
                        _ => {
                            runtime.close_notice();
                            let _ = window.show();
                        }
                    });
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("build Singalong")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                app.state::<runtime::Runtime>().shutdown();
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
