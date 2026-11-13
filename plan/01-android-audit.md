# Android product and source audit

Baseline: 2026-11-13, Android 1.3.0 at `453068332bb58e3d642d294ef861ba6ccd337952`.

Source links below are pinned to that commit. Paths in code refer to `app/src/main/java/com/kmab/singalong/` unless otherwise stated. The [file inventory](08-source-inventory.md) complements this behavioral audit; inventory inclusion is not a claim that every drawing statement was individually verified.

## Product and navigation

Singalong follows music already playing in selected apps. It reads session metadata and position; it neither plays songs nor records audio. Its four top-level destinations are Now, Library, History and Settings. A saved-song reader, version/import/editor workspace, timing editor, folder scan sheet and lyric-card workspace branch off these destinations.

Now shows current recording, player, lyric source, lookup state and reader. Options select an active session or automatic selection, choose lyric versions, adjust synchronization, edit/import, and open translation. Saved-song reading works without a current playback session. The library contains recordings with saved lyric text; history contains recordings with qualified plays, even without lyrics. Both support search and favorites. Sorting includes title, artist, album, recently played and most played, with deterministic ties and missing values last.

The visual identity uses kmab/Singalong marks, dark purple surfaces, glass-like panels and local fonts. Android layouts are touch-oriented. Desktop should preserve identity and reading hierarchy while adapting navigation, keyboard access, resizing and pointer interactions.

Evidence: [FoundationScreen](https://github.com/kmab5/singalong/blob/453068332bb58e3d642d294ef861ba6ccd337952/app/src/main/java/com/kmab/singalong/ui/FoundationScreen.kt), `ui/CollectionScreen.kt`, `ui/SettingsScreen.kt`, `ui/theme/`.

## Runtime and tracking

`AppContainer` wires a shared coroutine scope, DataStore settings, Room library, playback observer, tracking coordinator, providers, reader, translation and overlay state. It also owns card capture and folder scans. The desktop port should preserve shared ownership without recreating this large wiring class as a monolith.

Android uses NotificationListenerService permission to access MediaSessionManager. It does not read or store notification text. A foreground service supports explicit tracking and the overlay. A Quick Settings tile is a start/stop shortcut; simply adding or viewing the tile does not start tracking. App opening can resume a persisted tracking preference; service recreation alone cannot authorize a fresh session. Overlay permission is separate from music access.

Only selected packages are eligible. A newly observed PLAYING transition wins automatic selection; otherwise the previously selected non-stopped session is retained, then a playing session, then the first eligible session. A manual session override wins until it disappears or becomes disallowed.

Recording identity preserves original display metadata and compares NFKC-normalized, case-folded, whitespace-normalized fields. Missing fields may be enriched during an uninterrupted observation. Conflicting artists, albums or durations must not collapse studio/live/remix recordings. An ambiguous set of compatible records is not arbitrarily merged.

A play qualifies after **10,000 ms of observed playing time**, not media-position progress. Pauses do not add listening time; seeking ahead cannot instantly qualify. Each occurrence is counted once. A near-end-to-start repeat can start another occurrence. History uses wall-clock timestamps; qualification uses elapsed time.

Evidence: [TrackingCoordinator](https://github.com/kmab5/singalong/blob/453068332bb58e3d642d294ef861ba6ccd337952/app/src/main/java/com/kmab/singalong/tracking/TrackingCoordinator.kt), `tracking/{AndroidPlaybackObserver,PlayerSelector,CollectionQualifier,TrackingGate,TrackingStartRequests,TrackingService,TrackingTileService}.kt`, `library/RecordingIdentity.kt`.

## Lookup, provenance and synchronization

Local selected lyrics win before network lookup. LRCLIB is primary; lyrics.ovh is a plain-text fallback. A failed source does not prove absence. The app distinguishes loading, offline, rate limited, unavailable, malformed response, network error, missing lyrics and a confident instrumental match. Retries are bounded, honor cooldowns and stop when demand or policy changes.

Search uses at most three deduplicated queries, including conservative filename hints. Hints never overwrite original metadata. Matching compares normalized title and artists, preserves edition qualifiers, rejects large duration disagreement and requires both close duration and matching album for confidence. Inferred/manual and fallback matches remain uncertain. Candidate ranking prefers timed lyrics, then duration proximity, confidence, usable word timing and stable ID; explicit user selection is preserved.

LRC parsing supports multiple timestamp tags, offsets, ordered timing and plain fallback. Blank timed markers describe instrumental intervals. A long gap alone does not establish an instrumental break. A bounded safe parser also accepts the supported Lyricsfile 1.0 draft subset, falling back to LRC/plain when unsupported. Word segments retain original whitespace and absolute track timestamps; malformed words are discarded without discarding valid line timing.

Playback position advances from a snapshot using a monotonic clock and speed only while playing, clamps to duration and freezes when paused. Missing position means manual reading. The visible reader updates lyric moments on a 200 ms loop. UI word animation can interpolate separately. Scrolling suspends following, and the user can resume it.

Offsets are per lyric version, clamped to **±600,000 ms**. Positive offsets display later. Controls include step adjustments, direct seconds input, reset and aligning a chosen line to now. Untimed lyrics can opt into a clearly labelled estimated timeline weighted by non-whitespace character count and track duration. Estimates never become original timestamps.

Now appearance has separate plain/timed sizes, alignment, spacing, inactive-line dimming, tap focus, word tracking, and none/fade/focus/glow modes. Now's glow can use visual word estimates if supplied timing is absent. Floating word tracking is different: supplied timestamps only, no glow, disabled during manual browsing; guards reject more than 128 word segments or 4,096 characters for that renderer.

Evidence: `lyrics/{ReaderCoordinator,LyricsRepository,LyricSearch,SearchQueries,LyricsMatcher,LyricPreference,LyricTiming,LyricsfileParser,WordSync,WordSweep,EstimatedTimeline,FloatingWordTracking,GlowMotion}.kt`. Tests explicitly cover stale results, malformed timing, empty lyrics, offline cache and threading.

## Floating reader

Modes include compact, expanded and minimal presentation, plus hidden, manually minimized and automatically minimized lifecycle states. The panel can move, resize and dock to an edge handle; tapping reopens and dragging the handle moves it without opening. Minimal mode reveals accessible controls on interaction. Geometry respects available screen bounds and provides enough height for wrapped controls.

A pause lasting **15 seconds** can auto-minimize. A terminal missing-lyrics state without an active retry gets an **8-second** grace period. Automatically minimized states may reopen when playback or lyrics recover; manual close/minimize must not be undone automatically. Screen lock/off suppresses display and unnecessary reader work. Permission revocation removes the window and reports recovery instructions.

Floating typography is independent from Now: current/other-line sizes, alignment, supplied-word highlighting, opacity and minimal height. Translation size is separately configurable.

Evidence: `overlay/{OverlayState,OverlayGeometry,OverlayControlsLayout,OverlayWindowHost,FloatingLyricsPanel,PreviewLyricsPanel}.kt`, `ui/FloatingLyricText.kt`, overlay and floating UI tests.

## Offline library, import and personal editing

Room schema **4** stores recordings, unique play occurrences, lyric JSON versions, provenance-only references, selections, adjustments and translations. Migrations add favorites/translations, filenames and lyric references.

Users can add a song, search with corrected query fields, preview UTF-8 TXT/LRC imports and save a new personal version. Import is bounded to **2 MiB** and **20,000 lines**. Editing does not overwrite the provider original. Preserving line timing requires stable line structure; changing a line's text drops its word timing. Retiming also drops obsolete end/word timing on changed lines. Multiline timed segments need an untimed copy before structural editing.

Timing drafts accept partial timing, validate nondecreasing timestamps without reordering lyrics, nudge by 100 ms, clear times and stamp the current matching song's playback position before advancing. Drafts survive hiding timing controls; leaving dirty edits requires a discard decision.

Folder lookup reads only a selected tree, extracts local audio metadata and filename hints, then searches serially. It supports progress, cancellation, network waiting and cooldowns. It skips existing saved matches and rechecks within a transaction before saving. Unreadable files count as failures, misses do not create empty lyric entries, and audio bytes are never uploaded.

Clear history retains lyrics/favorites. Clear downloads preserves favorites by default and retains imports/personal versions; related selections/adjustments/translations remain coherent. Late translation/adjustment writes check version existence. Desktop should additionally apply generation invalidation to all in-flight download work when clearing downloads.

Evidence: [LibraryRepository](https://github.com/kmab5/singalong/blob/453068332bb58e3d642d294ef861ba6ccd337952/app/src/main/java/com/kmab/singalong/library/LibraryRepository.kt), `library/{LibraryDatabase,AudioFolderSource,FolderIdentity,FolderScanCoordinator,LibrarySort}.kt`, `editor/`, `ui/{LibraryTools,EditorDraft,TimingEditor}.kt`.

## Translation and external handoff

English translation is optional and requires explicit consent. ML Kit identifies language locally; MyMemory receives original lyric lines. English/unknown language skips translation. Repeated text shares cached segments; at most two segments are requested concurrently. Lines over **500 UTF-8 bytes** are unsupported by the current adapter, rather than silently split. A local **5,000-code-point daily budget** reserves usage before sending. Cached partial lines remain useful after failure. Only complete validated translations are stored as full version translations; segment cache stores partial progress.

Translation is for opened lyrics, not bulk background library collection. Records bind to version ID, content hash and original line IDs. Disabling translation cancels work and leaves originals intact.

Open in Google Translate is separate: copy full original text, attempt a bounded prefilled URL, otherwise open a paste target. Report copying and launching independently. It does not import translations back into the library.

Evidence: `translation/`, `settings/SettingsStore.kt`, translation tests and Android's `docs/PROVIDERS.md`.

## Lyric cards

A card freezes the chosen recording/version/line and matching artwork so later playback cannot change the draft. Users select original lines (up to 6,000 characters), change artwork or a separate background photo and export locally rendered PNGs.

Eight templates: Afterglow, Signal, Margin, Spotlight, Sleeve, Postcard, Duet and Gallery. Seven fonts: Inter, Lora, Oswald, Caveat, Space Grotesk, Playfair Display and PT Mono. Four output sizes: 1080×1080, 1080×1350, 1080×1920 and 1600×900. Controls include curated/local-art palettes, seven background treatments, alignment, size/weight/spacing, placement, cover scale/corners, optional title/artist/album, logo position/scale/style and app name.

Artwork-derived colors come from the cover independently of background photo or cover visibility. Renderer measures text, handles mixed directions and reports overflow instead of silently truncating selected words or credits. Save/share uses a local PNG, with bounded images and temporary-file cleanup. Audio filenames/folder paths are excluded from card content.

Evidence: `cards/`, `ui/{LyricCardSheet,CardEditorControls,CardEditorWorkspace}.kt`, renderer/text-flow/export tests.

## Backup and settings

Selective ZIP export includes full lyrics, links/metadata only or no lyrics; all versions or preferred-only; optional metadata, history and settings. Preferred selection uses the user's choice before recommendation and carries corresponding timing and translation. Minimal recording identity accompanies lyrics/history when rich metadata is omitted. Audio/artwork are excluded.

The ZIP contains an index, details, selected section JSON files and individual lyric-version files. Hashes, sizes, CRCs, safe paths, local headers and central-directory consistency are validated before merge. Legacy schema-1 JSON is accepted. The default archive cap is **50 MiB uncompressed**, user-adjustable or unlimited; record-level validation still applies. Archives are not encrypted.

Restore is a merge: no clearing omitted sections, no repeated count inflation, no replacement of existing selections/adjustments, no ambiguous merging of partial recordings. Links-only entries do not count as downloaded lyrics. Settings application is optional/off by default, validated before writes, and preserves current tracking and translation-consent choices. Android uses separate Room/DataStore transactions and reports a later settings-write failure explicitly.

Persisted settings include allowed players, tracking/translation choices, network policy, appearance, geometry, library sorting, notification-prompt state and archive size. Android permissions and temporary drafts are not portable preferences.

Evidence: `backup/`, `settings/{SettingsBackup,PreferenceBackup,SettingsStore}.kt`, `ui/BackupControls.kt`, [Android backup guide](https://github.com/kmab5/singalong/blob/453068332bb58e3d642d294ef861ba6ccd337952/docs/BACKUPS.md).

## Supporting systems and evidence limits

Crash diagnostics keep one bounded local report with exception types/code locations, excluding exception messages, lyrics, metadata and URLs. Next launch offers review/export, not automatic upload. Feedback opens the kmab form for user completion. About and the static website share versioned release-note JSON.

Android release tooling validates versions, signing continuity, APK architectures and release-note assets. Public releases contain universal/ABI APKs; diagnostics stay in CI artifacts. These are Android mechanisms to replace, not copy verbatim.

Android documents 204 passing unit tests for its 1.3.0 baseline, plus build/lint/tooling results. Instrumented test compilation is explicitly distinguished from execution. This audit did not rerun them or verify all players. Source licensing is GPL-3.0-only with an ML Kit-specific additional permission; third-party assets/content retain their terms.

## Desktop delivery mapping

The audit above describes Android behavior, including its original player selection and Wi-Fi settings. The [confirmed desktop scope](09-confirmed-release-scope.md) overrides those behaviors where explicitly different; do not change this historical audit to match the new product.

| Capability | First stable / later | Tasks |
| --- | --- | --- |
| Selected-player tracking, sticky automatic selection and waiting pins | Stable; desktop selection differs from Android | P0–P2 |
| Qualified collection and recording identity | Stable | P1–P3 |
| Providers, matching, line/word timing, estimates and offsets | Stable | P3 |
| Basic floating reader | First usable preview | P4a |
| Compact/expanded/minimal reader, collapse and click-through | Stable | P4 |
| Library search, favorites, sorting, manual lookup/correction | Stable | P5.1 |
| Play counts/last-played | Stable data; separate History screen later | P2 / P5.3 |
| TXT/LRC, personal edits, line-timing editor | Later; preserve imported versions now | P5.3 |
| Folder lookup | Later | P5.2 |
| Android import and full desktop backup/restore | Stable, including inert translations/settings | P6 |
| Selective and reverse Android exports | Later | P6.2 |
| Translation services and external handoff | Later | P7 |
| Cards and PNG export | Later | P8 |
| Essential appearance, tray, opt-in startup, manual updates | Stable | P2/P4/P9.1 |
| Product/download landing page only | Separate website task | P9.2 |
