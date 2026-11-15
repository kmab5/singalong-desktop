# Portable release build

Build an optimized Windows x64 executable with the production frontend embedded:

```powershell
npm run build
cargo build --locked --release --manifest-path src-tauri/Cargo.toml --features desktop,custom-protocol --bin singalong-desktop -j 1
npm run check:native-manifest -- -Executable src-tauri/target/release/singalong-desktop.exe
```

The `custom-protocol` feature enables Tauri's production asset protocol. Release builds use the Windows GUI subsystem rather than opening a console. No development server or Node installation is needed to run the resulting executable.

Local test packaging goes under ignored `artifacts/portable/Singalong-windows-x64/`: copy the executable as `Singalong.exe`, plus LICENSE, NOTICE.md, SOURCE-NOTICES.md, PRIVACY.md and release-notes/. Include running instructions, build identity and SHA-256 checksums. Keep plans and agent instructions outside the package.

This is installation-free packaging, not a separate portable-data mode: the app still uses its normal Windows per-user app-data directory. Microsoft Edge WebView2 Runtime is required. Quit any older running Singalong instance from its tray menu before opening the new executable, since single-instance handling may activate the older process. Copying the package does not install WebView2 or register startup.

A local release build does not establish native compatibility, signing or public release readiness. Building and inspecting it does not require launching the application. Installer and live behavior verification remain separate.

## Local build result

Built successfully with Cargo's optimized release profile, desktop and custom-protocol features. Production frontend/typecheck and release Common Controls v6 manifest checks passed. PE inspection confirmed x64 Windows GUI and identified the two Visual C++ runtime dependencies; their signed release redistributable DLLs are included alongside the executable. WebView2 remains a system prerequisite. No native app was launched.

Executable: artifacts/portable/Singalong-windows-x64/Singalong.exe (16,829,952 bytes).
SHA-256: 8af36b2d93e06be77c4e0f16d2e34069bd9c661105dfb541ec219f87f0f6b1dc.
Archive: artifacts/portable/Singalong-windows-x64.zip.
The artifact includes notices, release notes, run instructions and file hashes. It has not been signed or published.
