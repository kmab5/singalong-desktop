# Desktop Migration Implementation Plan

> **For agentic workers:** Use the executing-plans workflow to implement requested tasks sequentially. Subagent-driven development is an alternative only when delegation is explicitly authorized. Checkboxes record implementation completion, not merely planning.

**Goal:** Port Singalong's Android companion behavior to Tauri desktop while preserving offline content and honest platform capability reporting.

**Architecture:** A portable Rust core owns domain behavior; a Tauri shell owns platform access and shared state. A React/TypeScript frontend renders the main and floating windows. SQLite and a strict Android backup codec preserve data.

**Tech stack:** Confirmed Tauri 2, Rust, SQLite, React, TypeScript, Vite; Rust unit/integration tests, Vitest and browser E2E tests. Pin versions at P1.

Authoritative product requirements: [confirmed release scope](09-confirmed-release-scope.md). Product approval does not mark any implementation checkbox complete.

## Global constraints

- Plans remain in `plan/`; the `docs/` root remains available for future Pages hosting.
- No audio recording/playback, scraping, accounts, cloud sync or paid/key-based providers.
- Never replace original metadata, provider lyrics, user selections or personal edits silently.
- Supplied and estimated timing remain distinguishable.
- No platform is marked supported solely because it compiles.
- The original planning checkpoint contains no app implementation. P0 implementation began on 2026-11-15; see linked evidence for actual verification. No deployment or publication is implied.
- File paths below are proposed create/modify targets. Establish their exact interfaces within the task before downstream work.
- Use synthetic fixtures; regression requirements are detailed in [verification](06-verification-release.md).
- Complete a task's focused verification before updating its checkbox. Record blocked native checks separately.
- After foundation exists, use `npm ci`, `npm run typecheck`, `npm run test -- --run`, `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml --workspace` and `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings` as appropriate. These scripts now exist; native checks use the desktop/window-probe features.

## Dependencies and release slices

Keep the first stable scope fixed and deliver usable previews:

- **Preview A:** Windows P0 → P1 → P2 → core P3 → P4a. Real detection, allowlisting, sticky selection/pins, lookup, local cache, basic floating reader and honest unavailable/manual states. Synthetic fixtures alone do not satisfy this preview.
- **Preview B:** Finish P3.2 and P4: three modes, timing controls/estimates/word tracking, essential appearance, click-through and independent reader contexts.
- **Preview C:** P5.1 → P6: standalone/manual lookup and correction, library tools, cleanup, Android import and full desktop backup/restore.
- **Stable candidate:** P9.1 plus mandatory Windows compatibility gates. Include opt-in sign-in startup and manual updates. No scope cuts without a new user decision.
- **Landing page:** P9.2 separately when requested; product/download page only.
- **Later:** P5.2 folder scan, P5.3 editor/import/History, P6.2 selective/reverse exports, P7 translation, P8 cards, Linux/macOS adapters and advanced effects.

P5.1 depends on P1/P3, P6 on P5.1; deferred P7/P8 use the same data model. P9 numbering reflects the retained roadmap, not a dependency on deferred P7/P8. A minimal temporary Cargo/Tauri harness may be created as part of P0 probes, then consolidated into P1; P0 examples must not depend on nonexistent P1 scaffolding.

### P0.1 — Prove native media observation

**Files:** `src-tauri/examples/media_probe.rs`, `plan/evidence/platform-probes.md`.
**Consumes:** Platform APIs and the Android PlaybackSnapshot fields.
**Produces:** Redacted observations and a documented adapter mapping; no public support claim.

- [x] Define the probe output: ephemeral session ID, persistent player ID, metadata presence, status, position/rate availability and event times.
- [x] Implement a minimal Windows probe using session-manager callbacks and explicit token cleanup; run as the intended ordinary user process.
- [ ] Exercise two players, paused seeks, missing fields, player exit and repeated start/stop.
- [ ] Record Windows 11 native Media Player and Spotify desktop results and YouTube in Chrome/Edge/Firefox separately; later platform probes are not a Windows release dependency.
- [x] Record actual access/entitlement needs and timestamp conversions; choose a fallback for absent position.
- [x] Gate: a native session produces valid snapshots and shuts down cleanly. A mock is not evidence for this gate.

Evidence: [2026-11-15 native Media Player probe](evidence/platform-probes.md). P0.1 remains partial: two concurrent players and paused seeks were observed; access-loss, rapid in-process restart and the remaining player matrix are unverified.

### P0.2 — Prove window lifecycle

**Files:** `src-tauri/examples/window_probe.rs`, `plan/evidence/window-probes.md`.
**Consumes:** Tauri native windows, tray and platform lock/suspend hooks.
**Produces:** Supported window operations per OS/compositor.

- [x] Make a local-content floating window with drag, resize, always-on-top and tray recovery.
- [ ] Test focus, lock/unlock, fullscreen apps, scaling and monitor removal.
- [x] Verify a solid-background fallback and reset-position action.
- [ ] Decide which properties are reliable enough to expose; record unsupported behavior explicitly.
- [ ] Gate: no stranded offscreen window, no lock-screen lyrics, and a working way to stop/quit.

Evidence: [window probe and native smoke results](evidence/window-probes.md). The smoke run is not a substitute for actual tray interaction, lock/suspend or monitor checks. P0.2 remains partial; P1 continued with the remaining native cases retained as release gates.

### P1.1 — Establish the application and portable contracts

**Files:** `package.json`, `vite.config.ts`, `src/app/App.tsx`, `src/bridge/{types,native,fixture}.ts`, `src-tauri/{Cargo.toml,tauri.conf.json}`, `src-tauri/crates/singalong-core/src/model.rs`, `src-tauri/src/commands/mod.rs`, capability/permission files.
**Consumes:** P0 evidence and architecture.
**Produces:** Main window, explicit runtime capability DTO, generated/checked IPC types and synthetic preview.

- [x] Pin toolchains/dependencies, configure `dist/` output, ignore build artifacts and add the planned check scripts.
- [x] Define Recording, PlayerId, SessionId, PlaybackSnapshot, LyricVersion and error/event DTOs with units/nullability.
- [x] Add bridge tests for null timing, Unicode, large integer encoding, stale revision rejection and initial snapshot/event ordering.
- [x] Implement local main-window bootstrap and an explicitly labelled fixture runtime for browser tests.
- [x] Scope native/custom commands by caller and test that overlay cannot invoke privileged main-only operations.
- [x] Gate: native window and browser fixture preview load independently; production assets contain no remote privileged frontend.

Evidence: [P1.1 foundation](evidence/foundation.md). P0 hardware/account checks remain release gates under the user's direction to continue development autonomously.

### P1.2 — Implement durable storage and settings

**Files:** `src-tauri/src/storage/{mod,repository,settings}.rs`, `src-tauri/migrations/0001_initial.sql`, `src-tauri/crates/singalong-core/src/identity.rs`, `tests/fixtures/identity/`.
**Consumes:** P1 DTOs.
**Produces:** Transactional recording/version repository and typed settings snapshot.

- [x] Translate RecordingIdentity and FolderIdentity test cases, including ambiguous matches and missing-field enrichment.
- [x] Define tables, foreign keys, indexes and schema-version migration handling.
- [x] Implement identity resolution, immutable version insert/select, adjustments, favorites and paginated queries on a database worker.
- [x] Persist settings with sanitized ranges and explicit translation consent; default tracking off for a new install.
- [x] Test rollback on write failure, reopen persistence, migration failure and concurrent identity resolution.
- [x] Gate: no duplicate/ambiguous merging; no destructive database reset on error.

Evidence: [P1.2 storage and identity verification](evidence/storage.md).

### P2.1 — Implement tracked sessions and qualified history

**Files:** `src-tauri/src/platform/{mod,windows,mock}.rs`, later `linux.rs`/`macos.rs`; `src-tauri/src/runtime/tracking.rs`; core `selection.rs`, `qualification.rs`, `clock.rs`.
**Consumes:** Adapter mapping, settings and storage.
**Produces:** Authoritative current snapshot, eligible sessions and once-per-occurrence history writes.

- [x] Adapt PlayerSelector tests to sticky paused ownership and waiting app/stream pins; port CollectionQualifier tests with injectable monotonic and wall clocks.
- [x] Implement scoped adapter start/stop/event cleanup and a serialized tracking coordinator.
- [x] Support enabled-app allowlisting (new apps unchecked), sticky automatic selection, persistent app pins and reliably identified stream pins. Unavailable/disallowed pins wait without fallback; browser ambiguity refuses stream pinning.
- [x] Test 6 seconds play + 14 pause + 4 play → one qualified occurrence; seek-ahead → no early qualification.
- [x] Test app pin across restart, ambiguous stream reconnect, no silent stream-to-app widening, and refresh-before-freeze for unverifiable position.
- [x] Reconcile suspend/resume and reset unobserved intervals; prevent duplicate listeners/plays on rapid toggles.
- [x] Gate: real native snapshots drive the app and synthetic history tests pass.

Evidence: [P2 tracking and lifecycle](evidence/tracking.md). Native integration passed with synthetic Chrome and live Media Player rendering; the complete P0 compatibility matrix is still open.

### P2.2 — Build setup, tray and shutdown

**Files:** `src/features/settings/Players.tsx`, `src/features/now/SessionPicker.tsx`, `src-tauri/src/windows/tray.rs`, `src-tauri/src/runtime/lifecycle.rs`.
**Consumes:** Tracking API and capability report.
**Produces:** Explicit tracking controls independent of reader visibility.

- [x] Show discovered players and explain unavailable timing without claiming untested player support.
- [x] Add tray Show, Start/Stop tracking, main-window Reset position and Quit.
- [ ] Connect Show/Hide lyrics, click-through recovery and overlay Reset position when P4 creates the reader.
- [x] Resume prior tracking on explicit launch, explain first close-to-tray and implement opt-in Windows sign-in startup (off by default, tray launch).
- [ ] Verify actual sign-in registration/login, lock/suspend transitions and native Quit UI. Keep auto-show overlay opt-in and manual hiding authoritative when P4 connects it.
- [ ] Test stop while startup is pending, quit while fetching, second-instance launch and closed main window.
- [ ] Gate: closing a reader does not accidentally stop collection; Quit actually stops all app-owned work.

### P3.1 — Port parsing, matching and provider resolution

**Files:** core `lyrics/{lrc,lyricsfile,matching,preference,queries}.rs`; `src-tauri/src/providers/{http,lrclib,lyrics_ovh,policy}.rs`; `src-tauri/src/runtime/lookup.rs`.
**Consumes:** Recording repository, cancellation and network policy.
**Produces:** Local-first LookupResult with attribution, candidates and retry state.

- [x] Translate Android parser/matcher/query/preference fixtures before implementing equivalent rules.
- [x] Test offsets, repeated LRC tags, malformed words, blank-only responses, qualifiers and inferred filename hints. See [Preview A evidence](evidence/preview-a.md).
- [x] Add bounded HTTP and shared pacing/cooldown ownership; use a local mock server for 404, 429, 503, malformed/oversized responses and cancellation. Runtime wiring remains below.
- [ ] Implement fallback without turning a source failure into a definitive miss; show Possible match for uncertain/inferred lyrics.
- [ ] Fetch immediately for opened readers; save after ten seconds of actual playback in the background, including hidden windows. Add any-connection default and Offline mode cancelling provider work.
- [ ] Test saved selection preservation, simultaneous live/background requests (folder callers join later) and late results after stop/clear.
- [ ] Gate: provider work never overwrites the active song with an old result or silently replaces a personal version.

### P3.2 — Implement the reader and timing controls

**Files:** core `lyrics/{timeline,word_sync,estimated}.rs`; `src-tauri/src/runtime/reader.rs`; `src/features/now/{Reader,ReaderControls,WordLine}.tsx`.
**Consumes:** Current playback, selected version, adjustments and capability flags.
**Produces:** Revisioned reader state and accessible synchronized/manual reading.

- [ ] Port playback clock, blank-marker countdown, delay sign and estimated-timeline fixtures.
- [x] Implement supplied line/word highlighting and separately labelled optional visual estimates.
- [x] Add follow/manual scrolling, resume, version choice, retry, ±offset, reset and align-line controls.
- [x] Add independent text sizes, alignment and word switches plus reduced-motion support. Defer glow and detailed motion settings. Maintain accessible full text.
- [ ] Test missing position/duration, paused seek, same-recording metadata enrichment and song change during lookup.
- [ ] Gate: synthetic word/line outputs match fixtures; native playback remains correctly anchored after seek/pause.

### P4a — Deliver the basic floating reader for Preview A

**Files:** `src/overlay/{main,FloatingReader}.tsx`, `src-tauri/src/windows/overlay.rs`, `src-tauri/capabilities/overlay.json`.
**Consumes:** P0 window evidence and core P3 lookup/reader state.
**Produces:** A usable basic reader driven by real Windows playback and local cache.

- [x] Create a narrow-capability local-content window with show/hide, drag and tray recovery; no duplicate fetch or qualification owner.
- [x] Render live lyrics, source/possible-match labels and manual/unavailable states; use saved lyrics offline.
- [x] Keep automatic display opt-in; explicit hide wins. Hide on lock and stop unnecessary hidden rendering.
- [ ] Exercise real playback, stop/restart, cached reopen and missing position; record evidence before calling Preview A usable.

### P4 — Implement floating reader parity

**Files:** `src/overlay/{main,FloatingReader}.tsx`, `src-tauri/src/windows/{overlay,geometry}.rs`, core `overlay_state.rs`, `src-tauri/capabilities/overlay.json`.
**Consumes:** P4a window, shared reader state, per-context follow controls and P0 evidence.
**Produces:** Recoverable compact/expanded/minimal overlay.

- [x] Port OverlayState and geometry cases: 15-second pause, 8-second unavailable grace, retry reset and manual-state persistence.
- [x] Add logical-unit monitor-aware geometry, edge handle, minimum readable sizes and Reset position.
- [x] Implement separate floating sizes/alignment/opacity and supplied-word-only highlighting.
- [x] Add optional click-through with always-available tray recovery, normal interaction default and default capture visibility. Capture exclusion stays deferred.
- [x] Suppress on lock/suspend, avoid focus stealing and stop hidden animation work.
- [ ] Test main-window saved-song browsing while the overlay stays live; add Return to Now. Test shared version/offset changes without duplicate fetching/qualification.
- [ ] Gate: native checklist passes on every advertised OS/compositor; unsupported controls are absent or explained.

### P5.1 — Implement stable library, manual search and cleanup

**Files:** `src/features/collection/{Collection,SongDetail,SongSearch}.tsx`, `src-tauri/src/commands/library.rs`, `src/features/settings/Cleanup.tsx`.
**Consumes:** Storage, provider search and reader APIs.
**Produces:** Offline reading, search/favorites/sort and lookup recovery without requiring playback.

- [x] Build library search/favorites and deterministic sorting using retained play counts/last-played dates. Defer the separate History screen.
- [x] Implement title/artist search with no active playback, review candidates and save selected lyrics.
- [x] Add Wrong lyrics, alternate versions and corrected title/artist/album query fields; never overwrite original playback metadata.
- [x] Show source, Possible match and timing labels; preserve explicit selected versions including imported personal edits.
- [x] Keep saved-song reading independent of live overlay and expose Return to Now.
- [x] Add separate Clear downloads / Clear history actions. Preserve favorites and personal/imported content; invalidate pending writes before deletion.
- [ ] Gate: offline reopen retains selected versions, no-player search works, and deletion races cannot resurrect cleared content.

### P5.2 — Later: selected-folder lookup

**Files:** `src-tauri/src/files/{audio_metadata,folder_scan}.rs`, `src/features/collection/FolderScan.tsx`.
**Consumes:** Selected-directory token, metadata reader, shared lookup policy and transactional repository.
**Produces:** Cancellable scan job with checked/saved/skipped/not-found/failed counters.

- [ ] Choose a maintained metadata crate after format/license review; publish its supported file formats.
- [ ] Traverse only the selected tree, detect cycles and reject escapes; never upload audio.
- [ ] Port duplicate prevention, empty-version rejection, cooldown/network-wait and cancellation tests.
- [ ] Test unreadable files, access loss, disappearing volumes, cancel before dispatch and a live lookup saving the same recording.
- [ ] Gate: repeated scan adds no duplicate saved lyrics, completed saves survive failure, and cancellation prevents later commits.

### P5.3 — Later: personal editor, TXT/LRC import and History view

**Files:** `src/features/editor/{LyricsEditor,TimingEditor,draft}.tsx`, core `editor.rs`, native `commands/editor.rs`, `src/features/collection/History.tsx`.
**Consumes:** P5.1 storage/search and retained imported versions/history.
**Produces:** Immutable personal versions, line-timing editing and a separate history view.

- [ ] Implement bounded UTF-8 TXT/LRC preview and explicit save without replacing originals.
- [ ] Port text/line-timing edits, changed-line word invalidation and structural validation.
- [ ] Preserve unfinished input; add 100 ms nudges, matching-song stamp/advance and dirty-draft discard handling.
- [ ] Add history filtering using already retained counts/timestamps.
- [ ] Gate: original versions survive all editing paths and old imported personal versions remain usable.

### P6 — Implement Android import and full desktop backups

**Files:** core `backup/{model,android_v1,validation,merge}.rs`, native `files/backup.rs`, `src/features/settings/Backup.tsx`, `tests/fixtures/backups/`.
**Consumes:** Storage, settings, existing Android schemas and synthetic Android-generated archives.
**Produces:** Previewed atomic merge, full desktop export/restore and retained migration data.

- [ ] Capture synthetic fixtures from the pinned Android codec; record input provenance and expected decoded objects.
- [x] Implement exact schema-1 JSON/ZIP import and security/size validation from the compatibility document.
- [ ] Implement preview token, deterministic collision remapping and idempotent merge tests.
- [x] Import supported Android all/preferred and full/links/none archives, preserving omitted-section semantics. Export a full desktop backup; defer selective export controls.
- [x] Preserve current tracking/consent/startup registration and network policy. Retain translations and valid unsupported Android settings as inert data; report restored versus retained/unapplied content. Apply supported settings only if selected, atomically with library data.
- [x] Define a versioned full desktop format and verify desktop-to-desktop round trips, including inert translation/preference data. Synthetic codec/database round trips passed; native file-dialog and installation checks remain separate. Reverse Android export is deferred.
- [ ] Test ZIP64, large/unlimited archives, corrupt hashes/headers, settings errors, disk-full and repeated restore.
- [ ] Gate: Android migration preserves content/edits/timing/provenance without duplicates or silently re-enabling tracking.

### P6.2 — Later: selective and Android-compatible exports

**Files:** core `backup/options.rs`, `src/features/settings/Backup.tsx`, backup fixture tests.
**Consumes:** P6 validated full-backup codec and merge behavior.
**Produces:** Explicit selective controls and separately labelled reverse compatibility when verified.

- [ ] Add all/preferred versions, full/links/none lyrics and independent metadata/history/settings export options.
- [ ] Test selected-version timing/translation attachments and omitted-section restoration.
- [ ] Implement an Android-compatible export target without desktop-only fields; validate with the pinned Android decoder and device restore before advertising support.
- [ ] Gate: partial exports do not lose unrelated restored data or misrepresent links-only entries as downloaded lyrics.

### P7 — Later: implement translation and external handoff

**Files:** `src-tauri/src/providers/{language_detector,mymemory,translation_cache}.rs`, runtime `translation.rs`, `src/features/settings/Translation.tsx`, bridge handoff command.
**Consumes:** Opened version context, explicit consent, shared network policy and persisted quota.
**Produces:** Original-preserving English translation and explicit external Translate result.

- [ ] Evaluate and pin a local detector; document uncertainty and unsupported language behavior.
- [ ] Port repeated-line cache, content-hash validation, 500-byte limit, two-request concurrency and budget tests.
- [ ] Keep partial display separate from complete saved translation; cancel on context/consent changes.
- [ ] Add copy-full-text and bounded external-link handoff with independent failure reporting.
- [ ] Gate: disabled/unopened/unsupported input sends no lyrics; partial failure never hides originals.

### P8 — Later: implement lyric cards

**Files:** `src/features/cards/{model,composition,renderer,CardEditor}.ts[x]`, `src/shared/fonts/`, native `commands/card_export.rs`, `tests/e2e/cards.spec.ts`.
**Consumes:** Immutable lyric/recording snapshot and bounded local image tokens.
**Produces:** Local preview plus PNG save at the four Android output sizes.

- [ ] Port the eight templates, seven font choices, palette/background/metadata/logo controls and 6,000-character selection bound.
- [ ] Carry font notices and original-art provenance; use synthetic visual fixtures.
- [ ] Share measurement/layout between preview and full export; report overflow without clipping words/credits.
- [ ] Test every template/aspect, missing cover, separate photo, art palette, RTL/emoji/combining text and font readiness.
- [ ] Freeze artwork before track change; clean temporary images and cancelled exports.
- [ ] Gate: exported PNG dimensions/content are correct in actual target webviews; optional image clipboard is tested separately.

### P9.1 — Finish product settings, diagnostics and packaging

**Files:** `src/features/settings/{Appearance,About,Diagnostics}.tsx`, `src-tauri/src/diagnostics/mod.rs`, `docs/releases.json`, future `LICENSE`, `NOTICE.md`, `PRIVACY.md`, `.github/workflows/checks.yml`, `.github/workflows/release.yml`.
**Consumes:** Completed feature scope and native evidence.
**Produces:** Installable, honestly documented candidate release.

- [x] Complete essential settings/reset, opt-in sign-in startup, bounded local diagnostics and user-reviewed export.
- [x] Add Check for updates opening the release page; updates are manually installed. No automatic updater in stable.
- [x] Bundle release notes for offline About and share content with the future site.
- [ ] Carry source/font notices; review desktop dependency licenses and provider disclosures.
- [ ] Verify the mandatory Windows player/browser matrix, installer, clean install, update retaining data, uninstall behavior and signing. Test secondary players and document limitations; defer other platforms.
- [ ] Keep updater signing separate from OS signing if an updater is later added; never embed private keys.
- [ ] Gate: release checklist passes; unresolved support limits are published. Publication remains a separate explicitly requested action.

### P9.2 — Build the product/download landing page

**Files:** future `docs/index.html`, `docs/assets/`, `docs/site.css`, `docs/site.js`, site-check tooling.
**Consumes:** Actual shipped feature/support matrix and release metadata.
**Produces:** Solely a static product/download landing page preserving plans.

- [ ] Present the product, accurate supported-player limits and stable download links. No app demo or browser edition.
- [ ] Build outside `docs/` and copy only declared generated assets, or stage a curated Pages artifact.
- [ ] Use repository-relative paths and test project-subpath hosting.
- [ ] Use original/synthetic illustrative content without user lyrics or private data.
- [ ] Test responsive/keyboard/reduced-motion/no-JS content and release-download fallback.
- [ ] Gate: site build leaves root-level `plan/` byte-identical and the deployed artifact excludes plans, agent guidance and implementation evidence; deployment requires a separate request.

## Implementation-only checkpoint

The checked production-source items above are implemented in the current working branch. They are **not** build/test/native verification claims. The user paused testing while the full first-stable source was completed. Mixed implementation/test items and release gates remain unchecked. Source-review corrections and remaining dependencies are tracked in [stable implementation evidence](evidence/stable-implementation.md).

## Task handoff record

For each completed task append a short evidence record under `plan/evidence/`: task ID, commit, files, commands/results, native OS/player versions, remaining limitations and next task. Do not create empty evidence files or claim commands were run because they are listed here.
