# Verification, release and website plan

## Evidence levels

Use separate labels for source implemented, compiled, automated tests passed, native behavior observed and release published. A browser test of a mock bridge is not native playback evidence. Android's historical test report is a reference, not a desktop result.

Current evidence: source/documentation inspection only. No Android build or desktop runtime tests were run in this task.

## Stable compatibility gate

Mandatory: Windows 11 native Media Player (record exact app/version), Spotify desktop, and YouTube separately in Chrome, Edge and Firefox. Test Spotify web and Deezer web across those browsers, VLC and KMPlayer as secondary targets with documented exposed-session limitations. Other common apps are best effort. No universal audio-playback claim.

For mandatory targets verify tracking and sync where position is exposed, plus honest fallback where absent. A mandatory-target failure must be reviewed before stable; do not silently downgrade its gate. Record app/stream identity fidelity for pinning. Windows is the only first-stable OS; Linux/macOS checks are later gates.

## Behavior regression matrix

This matrix includes later-feature regression references. Editing, folder scans, translation services and cards are deferred; preserving their imported content in P6 is a stable requirement. Selective exports are later, but importing Android selective archives is required now.

| Area | Minimum cases and expected behavior | Android reference tests |
| --- | --- | --- |
| Selection | Current eligible session retains ownership through pause; replacement after stop/disappearance; app/stream pins wait without fallback; enabled apps only | Adapt PlayerSelectorTest for confirmed desktop differences; TrackingStartRequestsTest |
| Qualification | 6 s play + 14 s pause + 4 s play qualifies once; seek cannot qualify; repeat can start new occurrence | CollectionQualifierTest |
| Identity | Unicode/spacing comparison; original text retained; studio/live and ambiguous records stay distinct | RecordingIdentityTest, FolderIdentityTest |
| Provider | Local selection wins; 429 honors Retry-After; fallback error is not absence; late response cannot replace current song | LyricsRepositoryTest, LrclibClientTest, FallbackLyricsProviderTest, ReaderCoordinatorTest |
| Queries/matching | Filename hints bounded and labelled; edition mismatch rejected; timed preference never overrides explicit selection | SearchQueriesTest, SmartLyricsSearchTest, LyricPreferenceTest |
| Parsing | Multiple LRC tags, offsets, blank intervals, malformed YAML/words, partial word timing | LyricTimingTest, LyricsfileParserTest, WordSyncTest, EmptyLyricsTest |
| Reader | Refresh-before-freeze when position cannot be verified; pause/seek/rate/missing position; positive offset delays display; per-song estimates never persist | EstimatedTimelineTest, WordSyncTest; GlowMotionTest deferred |
| Floating | 15 s pause / 8 s terminal miss; only automatic states recover; wrapped controls and small bounds | OverlayStateTest, OverlayGeometryTest, FloatingWordTrackingTest |
| Library | Favorites/sort/filter, personal-version preservation, delete versus late save, transactional duplicate recheck | LibraryRepositoryTest, LibrarySortTest, FolderScanCoordinatorTest |
| Editing (later) | New version, changed-line timing invalidation, partial timestamps, dirty draft persistence and matching-track stamping | LyricsEditorTest, TimingDraftTest, EditorDraftTest |
| Backup | Android selective imports, full desktop round trip, inert translations/preferences, legacy JSON, ZIP64, corrupt entry/header, repeated merge; selective export later | BackupCodecTest, BackupArchiveTest, BackupRestoreTest, BackupVersionScopeTest |
| Settings | Typed malformed snapshot fails before writes; consent/tracking preserved; Android settings mapped explicitly | PreferenceBackupTest, SettingsBackupTest |
| Translation services (later) | Disabled/unopened sends nothing; repeats deduplicated; quota atomic; partial progress not complete persistence | MyMemoryProviderTest, TranslationRepositoryTest, GoogleTranslateHandoffTest |
| Cards (later) | Eight templates × four aspects; exact dimensions; overflow; RTL/emoji; cover palette independent of background | CardModelsTest, CardTextFlowTest, LyricCardRendererTest, CardExportTest |
| Diagnostics | No exception message, lyrics, paths or request URLs in logs; user-controlled export | CrashReportTest |

Additional stable desktop regressions:

- New apps unchecked; disabled pinned app remains inactive; disappeared pin never tracks another app.
- App pin survives restart; stream ambiguity cannot silently broaden to an app pin; browser-wide scope is labelled.
- Pause in player A plus new playback in B retains A. Refresh success tolerates infrequent events; refresh failure freezes sync with manual reading.
- Remembered tracking on explicit launch; × keeps tray/overlay alive; Quit removes observers/jobs. Opt-in sign-in startup defaults off, launches to tray and respects previous tracking state.
- Auto-show defaults off; manual hide/minimize survives song changes. Automatic pause/miss collapse recovers only automatically collapsed states.
- Click-through can always be disabled from tray; capture remains visible by default; lock hides lyrics.
- Hidden background collection qualifies at ten seconds; open reader lookup is immediate without premature play qualification.
- Offline mode cancels provider requests and reads cache; imported Wi-Fi-only settings are disclosed/inert and do not alter desktop policy.
- Wrong lyrics supports alternate selection/query correction without rewriting metadata; manual search works with no playback.
- Main library browsing does not redirect the live overlay; Return to Now restores live reading.
- Full backups retain imported translations and inactive settings; restore cannot enable tracking, consent, startup registration or OS permissions.
- Check for updates only opens the release page; manual installer upgrade preserves data.

Translate behavioral assertions, not merely test names. Use a common JSON fixture format for pure-domain Kotlin→Rust parity cases. Generate original synthetic lines; never copy downloaded lyric collections into tests or the public site.

## Test layers and commands

P1 establishes actual scripts and locks. Proposed layers:

- Rust core tests for clocks, state machines, matching, parsing, editing and merge rules.
- Rust SQLite integration tests against temporary databases, including transaction failure and migrations.
- Mock HTTP server tests for timeout, pacing, oversized/malformed payloads, cooldown and cancellation.
- Vitest/component tests for accessible controls, dirty drafts and bridge event reconciliation.
- Browser E2E tests for app flows against an explicit fixture bridge.
- Native smoke tests in packaged Tauri/WebView2/WebKit builds for IPC, fonts, file dialogs, clipboard, tray, window behavior and media adapters.

Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, workspace tests/clippy, frontend typecheck/tests/build and relevant E2E checks once configured. CI should never require live providers for correctness. A small opt-in live smoke test can check service availability without interpreting coverage failure as a regression.

## Mandatory native scenarios

Record exact machine/OS, player, application build and date. Do not include listening history in public evidence.

1. Start with no eligible player, select one, play, pause, seek, change track and stop.
2. Run two players and switch manual/automatic selection; close the selected player.
3. Remove network or enable Offline mode during lookup; verify cancellation, cache reading and retry after connection returns. Translation/scan checks follow their later phases.
4. Close main window, use tray/overlay, reopen, quit, then launch again; verify no duplicate observer or history entry.
5. Lock, sleep and resume. No lyrics remain visible on the lock screen; no unobserved sleep becomes listening time.
6. Move overlay between monitors with different DPI, unplug a monitor and recover through Reset position.
7. Validate keyboard-only use, screen reader, 200% scaling, reduced motion, RTL and long metadata.
8. Browse a saved song while playback changes; the floating reader stays live and Return to Now works. Card snapshot checks are deferred.
9. Import a synthetic Android backup containing personal edits and translations, select a version, restart, export a full desktop backup and restore twice. Compare IDs/counts/selection and retained inert content.
10. Simulate full disk/denied file access and cancel export/restore (and scan in its later phase). Existing data remains readable and temporary files are cleaned.

Windows native checks are required for the first preview. Linux checks name the compositor/session/package; macOS checks name supported players and permission state. Mark unsupported combinations explicitly.

## Performance checks

Measure rather than invent baseline numbers. Record idle CPU/memory, visible-overlay activity, event latency, large-library query time, card render time and backup peak memory. Acceptance requirements: no animation loop while hidden, no full-library broadcast on each position update, no blocking SQLite/HTTP/image decode on UI threads, bounded concurrent requests and cancellable long jobs.

For timing, test deterministic pure calculations exactly; record end-to-end native drift and callback latency separately. A visually smooth animation does not prove timestamp accuracy.

## Tagging checkpoints

Follow the repository [tagging ethics](../TAGGING.md): annotated `vA.B.C-D` tags, with major/feature/fix components and a globally increasing build number. Tag useful verified checkpoints liberally, including planning milestones and usable previews. Published tags are immutable; tags do not imply native verification or binary publication. The first planning checkpoint is `v0.0.0-1`.

## Releases

Use platform-specific CI builds for advertised targets. Start with a Windows installer candidate; choose Linux/macOS formats only after native and package-level checks. Do not copy Android APK ABI or certificate workflows.

Before public release: verify version/release-note agreement, repeatable install/update retaining app data, capability/permission behavior, dependency/source/font notices and privacy disclosures. Keep binaries separate from diagnostics and user data. Decide OS signing/notarization availability explicitly. First stable uses manual installation and a Check for updates link to the release page. Automatic updates are deferred; their later signing must be independent from installer code signing.

Reuse the Android product's provenance/license notices when porting covered material and document the desktop dependency set. This is a repository planning requirement, not a legal review. No signing files or licensing files were copied by this task.

## Pages

Keep authored plans in root-level `plan/`, outside the `docs/` publishing tree. Publish only `docs/` or a curated landing-page artifact; never include `plan/`, agent guidance or implementation evidence in site output. Both are supported by [GitHub Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site).

The site must reflect actual support, have stable download fallbacks, use self-hosted licensed assets and avoid analytics unless separately requested. The Pages site is solely a product/download landing page; no public demo/browser edition. Local fixture adapters remain testing tools. Validate project subpath hosting, responsive layout, keyboard/reduced motion, release metadata failure and plan-directory preservation.
