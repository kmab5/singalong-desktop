# Windows media feasibility evidence

Date: 2026-11-15. Task P0.1: **partial**, with its single-native-session gate passed. Source: `src-tauri/examples/media_probe.rs`. No application support claim or installer release.

## Environment and commands

- Windows 11 Education, version 10.0.26200, build 26200, x64 MSVC target.
- Rust 1.97.1, Cargo 1.97.1; windows crate 0.61.3, windows-future 0.2.1. Locked dependency graph in `src-tauri/Cargo.lock`.
- Windows Media Player package Microsoft.ZuneMusic 11.2609.11.0.
- Android reference inspected: `tracking/AndroidPlaybackObserver.kt` at the audited baseline. Its callback registration/cleanup pattern informs this probe; Android selection policy is not ported.
- `cargo run --manifest-path src-tauri/Cargo.toml --offline --example media_probe -- 3`: succeeded with zero sessions, exit 0.
- Direct debug executable runs of 8 and 60 seconds with user-started Media Player playback: succeeded, exit 0, no cleanup errors. Each run created a fresh manager and registrations; in-process rapid restart still needs testing.

No elevation was requested and no permission prompt appeared during these executions. This establishes access for this desktop process/environment, not every packaging or managed-machine configuration.

## Observations

| Check | Observed result |
| --- | --- |
| App identity | Stable packaged player ID exposed across runs |
| Session identity | Same COM session retained during track changes; probe ID remains local to one run |
| Metadata | Title, artist and album presence available; values not logged |
| Playback | Playing and paused states and playback callbacks observed |
| Timeline | Nonnegative position/end and finite rate available; timeline callbacks observed |
| Quiet intervals | Repeated queries returned unchanged timeline anchors between updates, roughly six seconds apart during ordinary playback |
| Paused rate | Rate could remain positive while status was paused; status must govern extrapolation/qualification |
| User seek sequence | User confirmed pause, play and seek actions. Position changed after resume; immediate paused-seek publication was not established |
| Track changes | Metadata and timeline notifications observed; the reads are separate and can briefly describe different transitions |
| Shutdown | Bounded runs exited normally; explicit event removal reported no errors |

No raw listening logs, metadata values or recording-specific durations are stored in this evidence.

## Adapter decisions

WinRT TimeSpan values are signed 100 ns ticks. Convert nonnegative positions to integer milliseconds by division by 10,000; negative values remain unavailable. Two unit tests exercise negative/zero/fractional/maximum conversion and were observed failing before implementation, then passing.

`LastUpdatedTime.UniversalTime` is a Windows date/time value, not monotonic elapsed time. The probe prints its raw value as a string alongside separate process-monotonic observation/event times. Production must not subtract this value from a monotonic clock. P2 must reconcile fresh timeline anchors, seeks and resumes and avoid continually re-anchoring an unchanged stale position at every poll. Missing or unverifiable position falls back to manual reading after refresh; unknown/default-zero timeline semantics need player-specific verification. EndTime alone is not a normalized recording duration; StartTime/ranges need consideration in the production adapter.

Probe IDs are ephemeral and cannot reconnect a stream across process runs. SourceAppUserModelId is a candidate persistent app key; browser-level IDs do not prove a particular tab's identity. Path-redacted diagnostic IDs must never become stored player keys. Poll reconciliation covers coalesced callback notifications; callbacks do no provider or storage work. Partial registration failures and normal exits remove all acquired tokens through guards. Forced process termination cannot run Rust destructors.

## Remaining matrix

| Target | Status |
| --- | --- |
| Windows Media Player | Observation, paused seek via native UI, exit/relaunch and concurrent Chrome session observed; access loss remains unverified |
| Spotify desktop | Not exercised |
| YouTube in Chrome / Edge / Firefox | Each unverified |
| Spotify / Deezer web in Chrome / Edge / Firefox | Each unverified |
| VLC / KMPlayer / other players | VLC 3.0.24 isolated synthetic WAV run exposed no session; this does not establish blanket incompatibility. KMPlayer and other players unverified |

The user directed autonomous probes and continued development. P1 proceeded while remaining P0 cases stay release gates. Player-specific tests continue as installations/sessions become available. Windows source API: [GSMTC session manager](https://learn.microsoft.com/en-us/uwp/api/windows.media.control/globalsystemmediatransportcontrolssessionmanager).

Subsequent autonomous checks: `npm run probe:browser` used Chrome 157.0.8103.51 in an isolated temporary profile with local synthetic audio/MediaSession metadata. Playing, seek while paused, missing artist and session removal after exit passed. Concurrent Windows Media Player and Chrome produced two sessions. HTTP byte-range support was required in the local fixture for seeking. This does not verify YouTube, Spotify or Deezer behavior. Spotify desktop was launched but usable playback was not established; Firefox, Edge and KMPlayer playback remain unverified. Probe-owned browser/player windows were closed after checks; no raw listening log was retained.

## Implementation handoff

Branch: `feat/windows-feasibility`. No commit, tag, push, installer or Pages deployment performed. P0 task IDs remain partial. P1 results are recorded separately in [foundation](foundation.md) and [storage](storage.md). The file/command list below records the initial probe checkpoint before P1.

Files added: `.gitignore`; `src-tauri/Cargo.toml`, `Cargo.lock`, `build.rs`, `tauri.conf.json`, `README.md`, `src/lib.rs`, `examples/media_probe.rs`, `examples/window_probe.rs`, `examples/support/window_hooks.rs`, `probe-ui/index.html`, `icons/icon.ico`, `icons/generate_probe_icon.py`; this evidence and `window-probes.md`. Status/checklist updates: root `README.md`, `AGENTS.md`, `plan/README.md`, `plan/05-implementation-plan.md`. The `docs/` directory is unchanged.

Verification on 2026-11-15:

- `cargo build --manifest-path src-tauri/Cargo.toml --offline --features window-probe --examples`: passed after adding the icon fixture. The later smoke command rebuilt the final window example and passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline --all-features --all-targets`: passed, two unit tests; both example test targets compiled, no tests in those targets.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-features --all-targets -- -D warnings`: passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` and `git diff --check`: passed.
- Native media runs and native window smoke: results above and in the separate window record. No `media_probe` or `window_probe` process remained at the post-check inspection; WebView child-process cleanup was not independently verified.

These checks establish implemented source, successful compilation, focused tests and limited native evidence separately. Remaining behavior and the next dependency are listed above, not silently treated as completed.
