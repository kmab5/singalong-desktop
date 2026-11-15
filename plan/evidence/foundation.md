# Application foundation — P1.1

Implemented and verified on Windows 11 Education 10.0.26200, 2026-11-15. No installer, tag, commit, push or Pages deployment is included. P1.2 storage is now implemented; P2 tracking is next. This is a development shell, not Preview A.

The root npm project now builds React/TypeScript/Vite assets exclusively to `dist/`. The native app loads those bundled assets and creates a main window. The probe retains its separate local HTML/configuration. `docs/` and the Android repository remain unchanged. Rust owns the foundation snapshot; the UI cannot turn on tracking or invent a native session.

Rust models in `src-tauri/crates/singalong-core` produce JSON Schema, TypeScript declarations and a precompiled validator through `npm run contract:generate`. `npm run contract:check` rejects drift. Validation is generated at build time so the native CSP does not need `unsafe-eval`. Numeric counters, durations and positions use canonical decimal u64 strings; nullable timing stays unknown. Lyric versions retain provenance, confidence and supplied/unsynced timing separately from any future estimates. No provider or storage operation is exposed over IPC yet.

The native commands are restricted both through Tauri's generated command permissions/capabilities and Rust caller-label validation. Main can read the snapshot and diagnostics; overlay can only read the snapshot and listen. No filesystem, shell, arbitrary URL or remote-webview capability is granted. See [Tauri capability semantics](https://v2.tauri.app/security/capabilities/).

The bridge subscribes before fetching its initial snapshot, compares generation/revision with BigInt, rejects malformed/stale data, and unregisters after errors or unmount—even if listener registration completes late. P1 introduces the state contract but no running collector or event-producing coordinator; those belong to P2.

`npm run dev:fixture` supplies explicitly labelled synthetic state. `npm run dev` and a normal web build do not silently enable it. The fixture’s playback scenario buttons only alter local test data. Native Library and Settings describe unavailable functionality rather than offering nonworking controls. Android colors were read from `ui/theme/Color.kt`; typography uses system fonts and all sample words/artwork are original.

## Verified commands and outcomes

- `npm install --prefer-online --no-audit --no-fund`: successful after refreshing inconsistent cached registry metadata; exact versions recorded in `package-lock.json`.
- `npm run contract:generate` and `npm run contract:check`: passed.
- `npm run test -- --run`: five bridge tests passed, after the unimplemented bridge was observed failing them.
- `npm run build`: typecheck and production build passed.
- `npm run test:e2e`: two Chrome browser tests passed: fixture state/navigation/keyboard/minimum-width behavior and refusal to enable fixture mode in a production browser load.
- `cargo test --manifest-path src-tauri/Cargo.toml --workspace --all-features --all-targets --offline`: five Rust tests passed before storage work; two contract, two timing and one caller-authorization test. The caller test exercises the same guard used by commands, not an end-to-end malicious-webview attack.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --all-targets --locked --offline -- -D warnings`: passed before storage work.
- `cargo run --manifest-path src-tauri/Cargo.toml --locked --offline --features desktop --bin singalong-desktop`: compiled and launched. Native accessibility state showed the Rust-supplied capability explanation; Settings navigation and rendered native content were inspected through Computer Use.

## Boundary and follow-up

P1.1 items are complete. The user explicitly requested autonomous probes and continued development; hardware/account-specific P0 gaps are carried as release gates instead of blocking unrelated foundation work. P0 is not retrospectively marked complete. See the media and window evidence for the distinction between synthetic/native/manual checks.

P1.2 adds transactional storage and identity resolution on a worker; see [storage evidence](storage.md). P2 will connect actual allowlisted players, qualified plays and tray lifecycle. The P1 main window currently closes normally because there is no background tracking; close-to-tray belongs to P2. Native permissions must be extended only as commands are implemented and tested.

Final review added finite-number validation and immediate listener cleanup after malformed initial/event snapshots, including registration races. All seven bridge tests and contract drift checks passed. Final production build/typecheck passed. Browser E2E passed both tests on rerun; an earlier run timed out during initial fixture navigation, without an application assertion failure. The timeout did not recur, and its cause is not established. After storage integration all 16 Rust tests and all-feature/all-target Clippy passed. The rebuilt native app was opened, Settings navigation verified, and closed through the window controls.
