# Preview A implementation record

In progress. Scope: P3.1 plus the basic synchronized/manual reader needed by P4a. Full P3.2/P4 controls remain Preview B, as already planned.

## Design and checkpoints

1. Port bounded LRC/lyricsfile parsing, conservative query hints, strict edition matching, provenance and candidate preference; use original synthetic fixtures adapted from the audited Android tests.
2. One provider worker owns HTTP pacing/cooldowns and cancellable requests to fixed LRCLIB/lyrics.ovh endpoints. The tracking coordinator continues processing controls while HTTP is in flight. Provider errors remain distinct from confirmed misses.
3. Rust owns a shared lookup result for the active recording. Open main/floating readers trigger immediate lookup; background work starts at qualification. Cache writes require qualification, current generation and current recording; explicit stored selections win. Offline cancels network but retains local reading.
4. Add one local floating window with restrictive capabilities, show/hide/tray recovery and opt-in auto-show. Manual hide/minimize suppresses auto-show. Both readers consume the same authoritative lookup/timeline state.
5. Verify synthetic HTTP/cancellation/storage races, browser reader flows and native playback/cache/floating behavior, then record remaining release gates and commit/tag the checkpoint.

No provider payloads or personal listening details belong in this record. `docs/` remains unchanged.

## Core parsing checkpoint — 2026-11-15

Implemented P3.1's parser/query/matcher/preference fixtures and the basic line timeline. Twelve synthetic lyric regression tests cover LRC offsets/repeated tags/blank intervals, unknown positions, safe lyricsfile words and legacy fallback, deterministic candidate ranking, metadata preservation, full edition matching and stable content identity. Tests first failed for the missing module; three review regressions also failed before their fixes.

Read-only review found repeated-tag text amplification and incomplete edition checks. Expanded LRC content now has a cumulative 2 MB cap, filename hints cannot replace a tagged title, and separate version labels participate in exact title matching. YAML parsing rejects duplicate keys, aliases, anchors, custom tags, multiple documents and excess depth/node count before model construction. Source license/provenance is recorded in `SOURCE-NOTICES.md` and `licenses/`.

Desktop-generated content hashes include all serialized timing fields. Imported Android IDs/hashes must be retained during P6 migration, rather than regenerated with the desktop representation.

No provider network, cache orchestration or floating-reader behavior is established by these core tests. Those remain the next dependencies for Preview A.

Verification: `cargo test --manifest-path src-tauri/Cargo.toml --offline --workspace --lib --tests` passed all 41 tests (12 new lyric tests); core all-target Clippy with warnings denied passed; Cargo formatting and `git diff --check` passed. This checkpoint builds/tests Rust libraries, not a newly verified desktop executable.
