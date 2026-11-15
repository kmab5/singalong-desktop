# Local storage — P1.2

Implemented 2026-11-15. Rust owns SQLite on the `singalong-database` thread. The native shell opens `library.sqlite` under Tauri's private app-data directory; no path, filesystem or SQL command is exposed to webviews. Library UI remains unavailable until P5.1; P2 can now use the repository for collection.

## Implementation

- Pinned `rusqlite` 0.37.0 with bundled SQLite and one bounded worker queue. Callers get typed results; dropping the final storage handle disconnects the queue and joins after queued work drains. Future asynchronous Tauri commands must call the synchronous worker interface from a blocking task, not block the UI event loop.
- Transactional schema migration 0 → 1, foreign keys, WAL and comparison/history/version indexes. Unknown future schemas or failed migrations return errors without deleting/recreating existing data.
- Recordings retain original strings and separate normalized comparison keys. NFKC/case/whitespace normalization follows Android's RecordingIdentity tests. Artist arrays are compared structurally rather than with a potentially colliding concatenation separator.
- A previous session recording can enrich missing metadata. Cross-session matches require unambiguous artist-backed evidence. Conservative filename hints cover the audited FolderIdentity cases without rewriting source titles; generic artist-less filenames stay distinct across sessions. Full provider query decoration/variant handling is still P3 work.
- Immutable version insertion and initial selection are one transaction. Explicit selections survive later candidate saves. Composite foreign keys prevent selecting another recording's version. Per-version offsets are bounded to ±600,000 ms; estimates remain a separate preference, not supplied timestamps.
- Paginated library reads, favorites, safe settings defaults/ranges and reserved tables for history, source references, translations and inert imported preferences. These tables do not implement restore/translation services or authorize them.
- Tracking, startup, automatic overlay and translation consent default off. Settings persistence itself never registers OS startup or activates a service. Restore policy remains P6 work.

## Verification

Eight actual SQLite worker tests passed: reopen durability; atomic batch rollback on immutable-content conflict; foreign-key selection ownership and offset bounds; eight concurrent identity resolutions yielding one recording; distinct/ambiguous albums; filename/tagged identity with original strings preserved; generic filenames kept distinct except with explicit session continuity; unsupported schema and migration conflicts preserving preexisting rows/version. Some scenarios share tests.

Three core identity tests cover Android RecordingIdentity/FolderIdentity examples with synthetic records, including Unicode normalization, live/studio separation, missing-field enrichment, filename hints and ambiguous albums. The generic-filename regression was observed failing before the comparison was tightened. Temporary test databases are not repository artifacts.

All-feature/workspace Rust tests passed with 16 total tests after storage integration. Contract export, TypeScript build and the seven bridge tests also pass. See the foundation and native records for their independent checks. Runtime UI wiring to the storage commands is intentionally absent until the relevant feature phases.

## Handoff

P1.2 creates `src-tauri/src/storage/{mod,repository,settings}.rs`, `migrations/0001_initial.sql`, core `identity.rs`, and real worker/identity tests; updates shell initialization, dependencies and lockfiles. No edits were made in Android or `docs/`. No commit/tag/push/publication is part of this checkpoint. Next dependency is P2.1: native adapter ownership, enabled-app selection/pins, monotonic clocks and qualified history.

Independent review reproduced conflicting version labels, filename hints overriding explicit edition titles, and duplicate exact recordings after ambiguous observations. Regression tests failed before correction and pass now. All identity paths reject conflicting explicit labels; hints cannot replace a conflicting tagged title; a unique exact candidate wins among compatible partial records.

Final all-feature/all-target Clippy passed with warnings denied. The desktop binary rebuilt successfully and was launched after storage integration: native content and Settings navigation worked, then the window closed normally. Opening the main window occurs after successful private database initialization.
