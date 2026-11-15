//! Application construction and shared P0 probe utilities.
pub mod commands;
pub mod storage;

#[cfg(feature = "desktop")]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::RuntimeState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_diagnostics
        ])
        .setup(|app| {
            use tauri::Manager;
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            app.manage(storage::Storage::open(app_data.join("library.sqlite"))?);
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("Singalong · Development")
            .inner_size(1120.0, 760.0)
            .min_inner_size(620.0, 480.0)
            .center()
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("run Singalong");
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
