# Windows window feasibility evidence

Date: 2026-11-15. Task P0.2: **partial; manual lifecycle gate open**.

Environment: Windows 11 Education 10.0.26200, x64 MSVC, Rust 1.97.1. Tauri 2.11.5, tauri-build 2.6.3; WebView2 runtime 157.0.4341.46. Sources: `src-tauri/examples/window_probe.rs` and `examples/support/window_hooks.rs`. The test text and icon are original synthetic assets.

## Implemented and exercised

The probe creates a solid local-content, resizable, topmost native Tauri window with normal title-bar drag. The tray offers recovery/reset, hide, click-through and quit. Close requests hide the window. Recovery disables click-through, unminimizes, centers, shows and focuses. Native WTS session and power messages hide on lock/suspend; window destruction unregisters the session notification and subclass.

`cargo run --manifest-path src-tauri/Cargo.toml --locked --offline --features window-probe --example window_probe -- --smoke` built and exited 0. Native calls verified hide/show visibility, topmost property, click-through enable/recovery calls and hiding in response to a **simulated** lock message sent to the installed handler. No actual lock or suspend was initiated. Tauri's [tray API](https://v2.tauri.app/learn/system-tray/) and the locally pinned crate API informed the harness.

The first build required an ICO even with installer bundling disabled; an original generated fixture now supplies it. After adding the application binary, the probe reported `TaskDialogIndirect` entry point not found. Windows SDK `mt.exe` confirmed the example had no manifest resource. The resource helper links only binary targets when the package declares a binary, so linking the library alone was insufficient. `build.rs` now explicitly links the generated `resource.lib` to Windows MSVC examples. Rebuilding embedded the Common Controls v6 manifest and the native smoke run passed. `npm run check:native-manifest` checks the actual executable resource to catch regressions.

The subsequent smoke run also verified resize and recovery from position (-32000, -32000) into the available monitor work area, plus simulated lock and suspend messages. One monitor was present. Solid local content was visually inspected. These checks do not establish real mixed-DPI, monitor unplug or OS lock/suspend behavior.

WebView2 emitted `Failed to unregister class Chrome_WidgetWin_0`, error 1412, during smoke shutdown despite exit 0. This warning remains recorded for investigation when consolidating the application lifecycle; clean visual/process shutdown is not established by exit code alone.

## Manual checks still required

- Verify rendered local content, drag, resize, focus and actual click-through with tray recovery.
- Close via X, recover through the visible tray menu and quit through that menu.
- Lock/unlock and suspend/resume using real OS transitions; ensure content stays hidden until explicitly restored.
- Fullscreen player behavior and focus competition.
- Mixed DPI, monitors with negative coordinates, monitor unplug, offscreen reset and edge docking decisions.
- Verify solid background appearance and accessibility in high contrast; record runtime/display versions.
- Confirm no residual window/probe/WebView processes after exit, and investigate the WebView warning.

No manual compatibility claim is made for these pending cases. P1 work continued under the user's direction; these remain release gates. The probes do not implement auto-show preferences, application tracking, lyrics fetching or persistent window settings.
