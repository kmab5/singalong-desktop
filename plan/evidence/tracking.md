# Tracking and lifecycle — P2

Implemented P2.1 and the currently usable P2.2 tracking/tray slice on 2026-11-15. P2.2 remains partial where it depends on the P4 floating reader and native lifecycle gates. P1 checkpoint: v0.0.0-2; portable rules commit: acfa6d4. This is not Preview A or a full player-support claim.

## Implementation decisions

- One Rust coordinator owns the adapter, selection, clocks and qualification. Commands serialize through it; webviews only render revisioned snapshots. Cancellation invalidates pending reads before Stop/Quit can publish stale state.
- Automatic tie-break: playing before paused/buffering/unknown, then native player ID and session ID. Keep the current eligible non-stopped session through pause. Pins never fall back; app pins persist, session pins use process-unique IDs and never reconnect by app name. Browser stream pins are refused without trustworthy identity.
- Windows callbacks only wake the coordinator. Periodic refresh covers dropped events. Disabled apps expose identity for setup but their track metadata is not fetched. Discovery while stopped is explicit and does not collect history.
- Position anchors use monotonic time, with bounded UTC-age conversion at native receipt. Refresh precedes stale-position fallback. Qualification uses observed playing intervals, never media position; gaps over ten seconds are unobserved and discarded. Suspend invalidates intervals explicitly.
- Occurrence IDs are idempotent SQLite keys; history insert/count/date update is transactional. Recording/version identity and source strings remain intact.
- P2 commits separate the portable rules from adapter/runtime integration and lifecycle UI. P3 lookup and P4 floating-reader rendering remain later work; no synthetic lyrics appear for a native song.

## Portable rules checkpoint

Six new core behavior tests and one real SQLite history test pass, alongside the existing suite (23 Rust tests total). Seven bridge tests pass after regenerating the contract. Core tests were first observed failing for missing modules; the history test failed before its repository method existed. Workspace Clippy passed. Native integration and lifecycle verification remain pending.

## Windows integration and lifecycle

The adapter owns a WinRT apartment, manager and session event guards on the tracking thread. Reads are bounded/cancellable; callbacks increment session revisions and wake reconciliation. A session-local read failure retains that session's identity but reports unknown playback/position, without tearing down healthy subscriptions. Metadata reads invalidated by callbacks are discarded. A manager-wide failure clears exposed sessions and retries. Main IPC commands are caller-validated; the overlay cannot change tracking or preferences.

The main window now offers Discover players, enabled-app checkboxes, Start/Stop, automatic selection, app pins and temporary native-session pins. Known browsers refuse individual-stream pins. A pin to a disabled/missing target waits; only Return to automatic removes it. Main settings expose explicit sign-in startup, default off. The native app uses pinned official [single-instance](https://v2.tauri.app/plugin/single-instance/) 2.4.0 and [autostart](https://v2.tauri.app/plugin/autostart/) 2.5.0 plugins compatible with Tauri 2.11.5. No autostart IPC plugin permission is granted to webviews: the guarded Rust command changes registration only on the explicit preference, and attempts rollback on persistence failure.

Close-to-tray has a first-close explanation with acknowledgment. Tray items provide Show, Start/Stop, Reset main position and Quit. Settings also offers Quit. A second launch shows the existing window. `--tray` launches with the main window hidden; it does not itself enable tracking or register startup. Quit invokes coordinator cancellation/join before exit; observers and storage work are app-owned, not webview-owned.

P4 owns the floating reader. Until then the tray explicitly labels floating lyrics unavailable, and automatic display remains off. Show/Hide lyrics, click-through recovery and overlay position reset must be connected in P4; this checkpoint does not pretend those controls work.

## Review regressions and verification

Independent review identified queued Stop generations, recounting after observer interruptions, session-local failures tearing down healthy sessions, and stale pause/resume anchors. The implementation now assigns generations to processed requests, skips work while newer commands wait, checks cancellation before history work/publication, retains qualified occurrence continuity for an unambiguous same recording through observer interruption, and refuses position extrapolation through an unverified status transition. This continuity is internal to qualification and never widens a stream pin. Actual app-process restart begins a new observation run; restore/history deduplication remains P6 work.

Observed failing regressions for interruption recounting and stale status-transition anchors now pass. Tests also cover cancellation during pending startup and a queued settings change followed by Stop without starting another native refresh. One database worker owns transactional occurrence insertion/count updates; occurrence retries are idempotent. Stored recording IDs are reused on retry and same-session repeats.

Verified on Windows 11 Education 10.0.26200, WebView2 157.0.4341.46, Media Player 11.2609.11.0, Chrome 157.0.8103.51:

- `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline --workspace --all-features --all-targets`: 29 tests passed (12 core, four library guards/coordinator, four runtime integration, nine storage).
- `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --workspace --all-features --all-targets -- -D warnings`: passed. Formatting also checked.
- `npm run contract:generate`, contract drift check, `npm run build`: passed. The generated schema uses explicit camelCase enum-field renames; an extra bridge test checks pin payload validation.
- `npm run test -- --run`: eight tests passed. `npm run test:e2e`: three tests passed, covering fixture reading, production browser isolation, and explicit tracking/app-pin actions with synthetic state.
- The first browser navigation repeatedly exceeded 30 seconds. Diagnostic network events and Vite warnings showed the dev watcher traversing locked Rust outputs. Vite now ignores Rust/artifact directories and scans dependencies from `index.html` only. All three tests then passed in 7.5 seconds at the normal timeout; first-page scenario took 1.2 seconds.
- `node scripts/probe-browser.mjs --runtime`: native adapter/coordinator/SQLite integration passed against isolated synthetic Chrome playback, including explicit discovery, an unchecked new player, enablement, metadata, app pin, ten-second qualification, observer stop/restart without recounting, browser stream-pin refusal and shutdown. The harness uses a temporary database and logs only aggregate results. Chrome paused-seek and session removal also passed.
- Native UI: live Media Player metadata/status/position rendered; setup defaults inspected; first-close explanation and hide verified; second launch exited while restoring the existing process/window; `--tray` created no visible window; explicitly reopening resumed the remembered enabled tracking/player selection. No listening values or screenshots containing user music are retained in the repository.

## Remaining checks and handoff

P2.1 is complete for this development slice. P2.2 is partial: floating controls require P4; Quit's shared coordinator shutdown is exercised by the runtime harness, but the final native Quit button and tray-menu click need a direct UI check. Sign-in registration/actual OS login and real lock/unlock/suspend/resume have not been exercised; source handlers and the P0 simulated-message evidence are not substitutes. Other P0 player/hardware gates remain open. Fetch-cancellation-on-quit becomes testable when P3 introduces fetching. No sign-in registration was enabled during testing.

Files: core selection/qualification/clock/models/tests, native platform/runtime/window modules, caller-validated commands/capabilities, settings/history repository additions, generated IPC types/schema, tracking/settings UI, Vite watcher configuration, browser/native probes and test fixtures. Android and `docs/` are unchanged. Local commits are authorized at development junctions; no push, installer release or Pages deployment was performed. The development window may be running with the user's selected tracking settings. Next dependencies are the remaining P2.2 native checks, then P3 lookup and P4 reader-dependent tray actions.
