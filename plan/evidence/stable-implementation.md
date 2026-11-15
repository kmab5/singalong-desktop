# Stable implementation progress

> Subsequent authorization: design review, builds and automated synthetic tests resumed. Live native testing remains excluded. See [current verification](design-verification.md). The implementation-only record below describes the earlier pause.

User authorization: finish the entire confirmed first-stable scope, then test. Testing is paused from 2026-11-15; do not run tests/probes or native/browser automation during coding. Existing passing evidence applies only to the recorded earlier revisions. Source completion below is not verification or release readiness.

| Area | Coding state | Verification |
| --- | --- | --- |
| P1/P2 foundation, storage, tracking/tray | Implemented; reader controls now connected | Earlier evidence retained; native lifecycle/compatibility gates open |
| P3.1 lookup/cache | Implemented, shared worker/coordinator and offline cache | Earlier parser/provider/coordinator evidence; newest fixes untested |
| P4a basic floating reader | Implemented, narrow local-only window and shared lyrics | Live display/hide observed earlier; lifecycle fixes untested |
| P3.2 full reading controls | Source implemented; scoped source review fixes closed | Deferred |
| P4 floating modes/geometry/appearance | Source implemented; native behavior unverified | Deferred |
| P5.1 library/search/correction/cleanup | Source implemented; cleanup/cancellation source review fixes closed | Deferred |
| P6 Android/full desktop backups | Codec committed; storage merge/native file flow/UI implemented; scoped merge/shutdown source review fixes closed | Deferred |
| P9.1 settings/diagnostics/manual updates/packaging | Settings, About, reviewed diagnostic export, manual release link, NSIS config and candidate/check workflows implemented | Deferred; signing/distribution remain external gates |

Deferred features and the future landing page remain outside this work. No pushing, installer publication or Pages deployment is authorized by this implementation request. Checkpoints are local commits; unverified coding checkpoints are not tagged as verified releases.

Source handoff: current changes cover Rust runtime/storage/window coordination, generated IPC, shared reader UI and settings. The next dependency is the deferred verification pass once the user resumes testing. The development app may still be running an earlier build; no automation will touch it while testing is paused.


Current source-generation commands: `cargo run --offline --quiet --manifest-path src-tauri/Cargo.toml -p singalong-core --example export_contract` and `npm run contract:generate` completed. This generated IPC schemas/types/validators and refreshed three core lockfile dependency entries; it does not establish a desktop build or passing behavior tests. Formatting used rustfmt and `cargo fmt --manifest-path src-tauri/Cargo.toml --all`. No tests, native probes or browser automation ran during this phase.

P3.2/P5 source review found deletion publication/suppression, retained-selection reload, immutable search-save and stale search-cancel defects. Fixes preserve dirty reader state until publication, key deletion suppression by stable recording identity, reload retained selections, reuse existing immutable versions, and isolate search cleanup by context. Re-review remains distinct from deferred regression execution.


Additional source: shared provider contexts isolate manual/live searches while retaining one pacing/cooldown owner; Unicode-normalized local library search; explicit import markers protect imported provider copies during Clear downloads; pending occurrence retries are discarded on Clear history/restore. Native backup preview owns an immutable copy/digest, applies through the runtime/database owner and writes exports through a temporary file plus atomic replacement. Independent byte/parser limits are exposed in the backup UI. Quit closes job admission and joins workers off the UI thread.

Local checkpoint `9db35f6` contains source-reviewed backup codecs and license. Integration source is complete; scoped reviews have closed the reported defects. No new verified tags or pushes have been made.


Final integration review fixes: reserve all incoming recording/version IDs during deterministic remapping; preserve discarded original recording metadata and source/target relations in exportable inert capsules; keep links-only/full-version salted mappings convergent unless full immutable text actually conflicts; prevent shutdown-time job admission and UI-thread joins; restore normal reader bounds after edge docking. Imported full lyrics without an explicit selection use a presentation-only preferred version; backup export retains the original nullable selection.

Completed production-source task coverage: P3.2, P4, P5.1, P6 and P9.1 source/configuration, on top of prior P1/P2/P3.1/P4a. Test fixtures/regressions, desktop compilation/typecheck, native playback/window/backup checks, installer/upgrade/uninstall checks, dependency-license review, signing and distribution remain separate open gates. No executable release or Pages artifact has been built or published in this phase. Plans remain under plan/ and docs/ was not changed.

Cleanup no longer backfills missing selections with MIN(id): surviving lyrics use presentation-only preference, preserving nullable imported choices across both reading and deletion. Existing reader fixture source now inherits new reader fields; it was not executed.


## Implementation handoff

Integration checkpoint: `106128d` (`feat: implement first-stable reading library and backup workflows`), following codec checkpoint `9db35f6`. Both are local commits on `feat/windows-feasibility`. No push or new tag accompanied these unverified implementation checkpoints.

Changed source areas: `src/features/{now,library,settings}/`, `src/overlay/`, `src/bridge/`, `src-tauri/crates/singalong-core/src/{lyrics,model,backup}/`, `src-tauri/src/{runtime,providers,storage,windows,commands}/`, native `backup.rs`, `files.rs`, `diagnostics.rs`, application setup/capabilities, and shared styles. Supporting source includes `scripts/contracts.mjs`, `scripts/package-windows.ps1`, `.github/workflows/`, `release-notes/`, license/privacy/notices, plan checklists and durable agent guidance. The existing reader fixture was adjusted for the expanded contract without running it.

Commands performed: Rust formatting and Rust-to-TypeScript IPC generation completed successfully. Source review covered codec bounds/cancellation, merge identity/retention, cleanup races, search isolation, floating-window threading/docking and backup shutdown. Tests, desktop compilation, typechecking, native/browser automation, installer production and CI workflow execution were deliberately not performed. Earlier test evidence does not verify these revisions.

Next dependency, only after the user resumes testing: frontend typecheck and generated-contract drift check; Rust/frontend regression suites and synthetic backup round trips/collisions/cancellation; desktop build/manifest check; browser fixture behavior; native playback/seek/lock/tray/multi-monitor/click-through/backup checks; then installer/upgrade/uninstall, dependency-license inventory/review and signing. The exact Tauri 2 CLI version used for a candidate must be selected explicitly and recorded with that build. Keep Preview A/B/C and stable verification gates open until their corresponding evidence exists.
