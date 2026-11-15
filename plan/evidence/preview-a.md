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

## Provider worker checkpoint — 2026-11-15

Added fixed LRCLIB and lyrics.ovh endpoints, 10-second timeouts, 2 MB streaming/body limits, disabled redirects and client retries, client identification, sequential 750 ms request pacing and provider-specific monotonic cooldowns. Retry-After accepts seconds and HTTP dates. Primary errors can use successful fallback, but failed sources cannot establish a definitive miss. A bounded single worker cancels obsolete jobs; one replaceable result slot prevents stale unread results blocking a newer song. It performs no database writes.

Nine original local-server tests cover primary/fallback provenance, failures versus misses, 404/429/503, malformed JSON, declared and chunked oversized bodies, stalled bodies, cancellation during requests/pacing, cooldown reuse, worker cancellation and stale completed results. A thirteen-test core lyric suite also caps aggregate candidate expansion. The mock server explicitly resets accepted Windows sockets to blocking mode and reads complete headers; ten consecutive provider-suite runs passed after correcting that harness issue.

The provider API behavior was checked against [LRCLIB's official documentation](https://lrclib.net/docs.txt) and [lyrics.ovh's API source](https://github.com/NTag/lyrics.ovh/blob/master/apiary.apib). No accounts, keys, scraping or write endpoints were added. No live provider response or UI integration is claimed by this checkpoint; shared runtime caching/reader integration is next.

Verification at this checkpoint: all 51 Rust library/integration tests passed; workspace library/test Clippy with warnings denied and formatting passed. Provider review found no important implementation defect; its additional cancellation/body-limit tests were added. Desktop UI/native provider behavior remains unverified.

## Shared runtime and basic reader — implementation checkpoint

Added read-only recording matching, immediate open-reader lookup, qualified-only cache writes, saved-selection precedence, Offline cancellation, shared main/floating state, supplied-line following, manual-scroll/resume and timing freshness fallback. Every published snapshot reconciles reader identity before emission. Terminal lookup outcomes are bounded to 256 recent entries, with miss/error lifetimes and explicit retry; provider cooldowns remain authoritative. Hidden windows do not receive rendering updates; showing a window requests fresh shared state.

The local-only overlay has native title-bar dragging/resizing, main/tray show/hide/recovery, opt-in auto-show and manual suppression. System hiding is separate from manual hiding, and stale queued auto-show snapshots are rejected. Lock/suspend recovery fixes are source-reviewed but have not been exercised.

Before the user paused testing, native Windows Media Player playback produced real provider lyrics with Possible match/Unsynced/Saved offline labels; the floating reader displayed the same active recording and closed to hidden. Main reader browser tests and coordinator regressions were run under the earlier testing authorization. Native stop/restart, offline cached reopen, automatic-display recovery, Quit and the wider compatibility matrix remain incomplete. The last native test left Offline mode enabled; no user preferences were changed after Computer Use was stopped.

On 2026-11-15 the user requested completing the entire stable implementation before more testing. Subsequent source changes have not been executed or verified. No preview/stable release or installer publication is claimed.
