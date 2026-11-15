# Tracking and lifecycle — P2

In progress, 2026-11-15. P1 checkpoint: v0.0.0-2. No Preview A or player support claim yet.

## Implementation decisions

- One Rust coordinator owns the adapter, selection, clocks and qualification. Commands serialize through it; webviews only render revisioned snapshots. Cancellation invalidates pending reads before Stop/Quit can publish stale state.
- Automatic tie-break: playing before paused/buffering/unknown, then native player ID and session ID. Keep the current eligible non-stopped session through pause. Pins never fall back; app pins persist, session pins use process-unique IDs and never reconnect by app name. Browser stream pins are refused without trustworthy identity.
- Windows callbacks only wake the coordinator. Periodic refresh covers dropped events. Disabled apps expose identity for setup but their track metadata is not fetched. Discovery while stopped is explicit and does not collect history.
- Position anchors use monotonic time, with bounded UTC-age conversion at native receipt. Refresh precedes stale-position fallback. Qualification uses observed playing intervals, never media position; gaps over ten seconds are unobserved and discarded. Suspend invalidates intervals explicitly.
- Occurrence IDs are idempotent SQLite keys; history insert/count/date update is transactional. Recording/version identity and source strings remain intact.
- P2 commits separate the portable rules from adapter/runtime integration and lifecycle UI. P3 lookup and P4 floating-reader rendering remain later work; no synthetic lyrics appear for a native song.

## Portable rules checkpoint

Six new core behavior tests and one real SQLite history test pass, alongside the existing suite (23 Rust tests total). Seven bridge tests pass after regenerating the contract. Core tests were first observed failing for missing modules; the history test failed before its repository method existed. Workspace Clippy passed. Native integration and lifecycle verification remain pending.
