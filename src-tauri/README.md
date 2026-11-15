# P0 native feasibility harness

These are development probes alongside the P1 application foundation, not a usable Singalong preview. Run from the repository root on Windows. Dependencies are pinned in Cargo.toml and Cargo.lock; Rust 1.97.1 is pinned in `rust-toolchain.toml`. The first build needs the MSVC toolchain, Windows build tools and WebView2. Use Node 24.22.1 for frontend commands.

Run `npm ci` and `npm run desktop` for the native foundation. `npm run dev:fixture` opens a local-only testing server. Build web assets with `npm run build` before compiling the `desktop` feature or running all-feature Rust tests; native production assets come from `dist/`.

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --locked --example media_probe -- 30
cargo run --manifest-path src-tauri/Cargo.toml --locked --features window-probe --example window_probe
cargo run --manifest-path src-tauri/Cargo.toml --locked --features window-probe --example window_probe -- --smoke
npm run check:native-manifest
npm run probe:browser
```

The media duration defaults to 15 seconds and accepts 1–300. Each asynchronous Windows request has a three-second timeout; request completion can extend the sampling duration. The probe observes all exposed sessions for compatibility testing only. It does not implement production tracking or allowlisting and must not be reused as an always-on collector. Output is ephemeral JSON lines: app identity, per-run session IDs, metadata presence, status and timing. Titles, artists, lyrics, artwork and file paths are not printed. Path-like app IDs are reduced to their last component and flagged; those redacted IDs are not suitable for persisted identity. Do not commit captured listening/timing logs.

While the media probe runs, exercise play/pause, seek while paused, resume, track change, player exit and two simultaneous players. Record aggregate outcomes and exact app versions in `plan/evidence/platform-probes.md`. There is no media-control code in this probe.

The window probe uses local synthetic text, a solid background and a generated original test icon. It has no JavaScript bridge, custom commands, remote content or plugin permissions. Native title-bar dragging and edge resizing are available. Close hides it; the tray menu can show/reset, disable click-through through recovery, hide, enable click-through or quit. Recovery centers and focuses the window. Lock/suspend hooks hide it; automatic restoration is deliberately absent from the probe. Quit terminates the process; no startup registration or persistence is created.

`--smoke` launches the actual native window, checks hide/show, topmost, offscreen recovery and resize calls, sends synthetic lock and suspend notifications to the installed native handler, and exits. This checks API plumbing only, not actual lock-screen privacy, input pass-through, focus interaction, tray usability or physical monitor topology changes. Never run real lock/suspend tests unattended. `check:native-manifest` uses the Windows SDK manifest tool to catch missing Common Controls v6 resources before launching the example. `probe:browser` uses an isolated Chrome profile and locally served silent PCM; it checks playing, paused seek, missing artist and session removal, reporting the observed concurrent session count. It neither visits streaming services nor uses the user's browser profile.

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked --workspace --all-features --all-targets
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --workspace --all-features --all-targets -- -D warnings
```

Add `--offline` when dependencies are already cached. Build artifacts stay in `src-tauri/target/`; generated Tauri schemas are ignored. The Rust-derived frontend schema/types/validator are committed source artifacts, regenerated with `npm run contract:generate` and checked with `npm run contract:check`. No build touches `docs/`. Browser tests run with `npm run test:e2e`; bridge tests with `npm run test -- --run`.

P2 integration: build the examples with `cargo build --manifest-path src-tauri/Cargo.toml --locked --offline --features desktop --examples`, then run `node scripts/probe-browser.mjs --runtime`. This uses isolated synthetic Chrome playback and a temporary SQLite database. It does not change the installed app's tracking preferences or register startup. See `plan/evidence/tracking.md` for lifecycle limits.
