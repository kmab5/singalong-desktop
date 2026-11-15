fn main() {
    if std::env::var_os("CARGO_FEATURE_WINDOW_PROBE").is_some()
        || std::env::var_os("CARGO_FEATURE_DESKTOP").is_some()
    {
        tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&["get_snapshot", "get_diagnostics"]),
        ))
        .expect("build Tauri resources and permissions");
        // embed-resource targets only binaries once Cargo.toml declares a bin.
        // Cargo examples need the same compiled manifest/icon resource explicitly.
        if std::env::var("TARGET").is_ok_and(|target| target.ends_with("windows-msvc")) {
            let resource =
                std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("resource.lib");
            println!("cargo:rustc-link-arg-examples={}", resource.display());
        }
    }
}
