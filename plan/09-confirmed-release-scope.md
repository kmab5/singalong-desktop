# Confirmed Windows release scope

Confirmed by the user on 2026-11-13 after the grilling session. This is the product contract for the first stable release. It supersedes earlier recommendations where they differ. Confirmation authorizes this documentation update; it does not claim implementation or authorize publication.

## Product, platform and delivery

Build a daily lyrics companion for Windows first, using **Tauri 2 + Rust + React + TypeScript + Vite**. Rust owns tracking, lookup, persistence and shared window state. Keep the core portable for later Linux/macOS work. The first stable release must satisfy the scope below; deliver smaller usable previews rather than silently cutting stable requirements.

Keep Singalong's colors, typography and reading identity, adapting the interface to sidebar navigation, resizable content, keyboard controls and context menus. The floating reader stays familiar. The installed app bundles its UI. GitHub Pages is solely a product/download landing page, with no browser edition or interactive app demo.

## Player compatibility and selection

| Target | First stable release gate |
| --- | --- |
| Windows 11 native Media Player | Mandatory real-player testing; record exact app identity/version |
| Spotify desktop | Mandatory real-player testing |
| YouTube in Chrome, Edge and Firefox | Mandatory testing separately in each browser |
| Spotify web and Deezer web | Test across the named browsers; document exposed-session limitations |
| VLC and KMPlayer | Test and document exposed-session limitations |
| Other common applications | Best effort through exposed media sessions; no universal audio-playback promise |

The mandatory set must pass tracking and synchronization tests for sessions exposing usable position. A discovered limitation in a mandatory target must be surfaced before calling the release stable; it must not be silently reclassified as best effort. Missing position has a supported manual-reading fallback, and missing duration prevents duration-based estimates. The Windows OS floor and package access remain technical validation questions.

Only apps explicitly enabled by the user are eligible. Present detected apps during setup; newly discovered apps remain unchecked. Browser app selection does not imply that every website or tab exposes independently identifiable playback.

Automatic selection retains the current eligible session through pauses, even when another enabled app starts. It can select a replacement when the current session stops or disappears. Disabling an app makes it ineligible. This intentionally differs from Android's newly-playing-player preference. Choose a deterministic tie-breaker for initial/replacement selection during implementation and test it; do not override the confirmed retention rule.

Manual pinning targets either an app or a specific exposed audio stream:

- A pin never falls back to a different target. If unavailable, show **Waiting for pinned player** and an explicit **Return to automatic** action.
- Persist app pins across app launches. A pin to an app may follow different content from that app; label its scope clearly.
- Treat stream pins as temporary unless the adapter can reliably identify that same stream again. If identity is uncertain or several streams could match, remain unavailable and require explicit selection.
- Never broaden a stream pin to an app pin silently. If the browser exposes insufficient stream identity, refuse individual-stream pinning and offer an explicitly labelled browser-app pin.
- A disallowed pinned app remains inactive until explicitly re-enabled or unpinned; a persisted pin is not permission to bypass the allowlist.

An infrequent callback alone does not prove playback is stale. Attempt to refresh state first. When position cannot be verified, freeze synchronization, show **Playback position unavailable**, and retain manual reading. Do not keep advancing lyrics indefinitely on an unverified snapshot.

## Lifecycle and floating reader

Remember tracking state. Explicitly reopening Singalong resumes tracking only if it was enabled before quitting and restores a persistent app pin. First-time setup still requires enabling players.

Closing the main window with × keeps the process in the system tray, with tracking and any visible overlay continuing. Explain this on first close. **Quit Singalong** stops observers/jobs and closes all windows. Include tray controls for show, tracking, floating lyrics, click-through recovery, reset position and quit.

**Start Singalong when I sign in to Windows** is included, off by default. When enabled, launch into the tray and resume only previously enabled tracking. Startup registration is changed only through the explicit preference.

Automatic floating-lyrics display is a separate opt-in. Once shown, the reader follows song changes. An explicit hide/minimize overrides automatic display until the user shows it again; never pop it open because another track starts or startup is enabled.

Stable includes compact, expanded and minimal modes; moving/resizing, recoverable geometry and Android-style edge docking. Preserve Android lifecycle rules:

- Pause for 15 seconds: automatically collapse; resume may restore only an automatically collapsed reader.
- Settled missing-lyrics failure: eight-second grace, then collapse. Do not collapse while a retry is active. Lyrics recovery may restore only an automatically collapsed reader.
- Manual hidden/minimized states never reopen automatically.
- Hide on Windows lock; handle suspend/resume without counting unobserved time as listening.

Normal mouse interaction is default. Include optional click-through with a tray toggle that always lets the user disable it. The overlay appears in screenshots and screen sharing by default. Capture exclusion and global shortcuts are later enhancements, not stable gates.

Library browsing affects only the main window; the floating reader continues following live playback. Provide **Return to Now**. Keep scroll/follow state scoped by reader context, while live track/version/timing state has one authoritative owner.

## Lyrics, collection and reading

Fetch immediately when an opened reader needs lyrics. Automatically save lyrics after ten seconds of actual observed playback, even when the overlay is hidden. Pauses and seeks must not fabricate listening time. Keep the offline cache and library local.

Preserve Android lookup behavior: local selected lyrics first, provider provenance, strict edition/recording matching, conservative filename hints and unchanged original metadata. Display uncertain or inferred matches automatically with **Possible match**.

Provide **Wrong lyrics** with alternate provider results and editable search title/artist (and available album detail). Search corrections do not rewrite original playback metadata. Also allow manual title/artist search without active playback, reviewing possible matches and saving lyrics offline.

Missing timestamps default to manual reading. Include Android's explicit per-song **Try estimated sync** when position/duration allow it; label estimates and never save them as original timing.

Include earlier/later adjustments, direct offset entry, reset and **Align this line with now**, saved per lyric version with Android's ±600-second range and positive=later semantics. This is timing correction, not the deferred full line-timing editor.

Include supplied-word highlighting with independent switches for main/floating readers; fall back to line highlighting when word timestamps are absent. Essential appearance controls include text sizes, alignment, overlay opacity and reduced-motion support. Preserve independent main/floating settings. Glow and detailed animation customization are deferred.

Network lookup uses any available connection by default. Include an explicit **Offline mode** that cancels/stops provider requests and keeps saved lyrics readable. Wi-Fi-only filtering is deferred. Explain imported Android Wi-Fi-only settings as unsupported; do not silently reinterpret them or change the current desktop network choice during restore.

## Library and backups

Stable includes library search, favorites and sorting. Retain qualified play counts and last-played dates for collection/sorting; defer the separate History screen.

Provide separate **Clear downloads** and **Clear history** actions with explanations. Clear downloads preserves favorites, personal/imported versions and coherent associated data. Clear history leaves lyrics intact. Invalidate in-flight jobs before deletion so they cannot immediately restore deleted content.

Include Android schema-1 ZIP/legacy JSON import and full desktop backup export/restore. Preserve saved lyrics, all versions, selected versions, personal/imported content, timing, provenance, favorites and history. Restore merges safely, preserves existing selections/edits, and does not inflate counts on repetition. Full desktop backups include supported settings; applying settings remains optional and never enables tracking, translation consent, startup registration or OS permissions.

Retain valid saved translations even though translation UI/network services come later. Carry them through subsequent full desktop backups. Validate and retain unsupported Android preference data as inert migration data where representable, report it in the import summary, and never apply it as active desktop configuration. Unsupported data must not be executed or passed through as permissions. Do not claim arbitrary future schema compatibility.

Selective export controls and desktop-to-Android export are later work. The import codec must still understand supported Android selective archives, including links-only/preferred-only and omitted sections. Those references must not count as offline lyrics. Keep the existing archive validation and size-budget design.

## Stable exclusions and release operation

Later: Linux/macOS adapters, full lyric and line-timing editors, standalone TXT/LRC import UI, folder scans, translation services/UI and external translation handoff, lyric cards, separate History view, selective export controls, advanced motion/glow, Wi-Fi filtering, capture exclusion, global shortcuts and automatic updates. Retaining imported data does not require exposing unfinished features.

No audio capture/player, recognition, scraping, paid/key-based provider, account or cloud sync is part of this port.

Use manual update installation. **Check for updates** opens the release page; it does not imply an automatic version check or download. Validate upgrade preservation and signing before stable distribution. Release publication requires its own request.

## Preview sequence and traceability

| Delivery | Task scope | Outcome |
| --- | --- | --- |
| Preview A: first usable preview | P0, P1, P2, core P3 and P4a | Real Windows detection, allowlisting, sticky selection/pins, lookup, local cache and a basic floating reader; explicit unavailable/manual states |
| Preview B: reading experience | Complete P3.2 and P4 | Three reader modes, word timing, estimates/offsets, appearance, collapse/recovery, click-through and separate reader contexts |
| Preview C: library and migration | P5.1 and P6 | Search/correction without playback, favorites/sort, cleanup, Android import and full desktop backup/restore |
| Stable candidate | P9.1 and mandatory compatibility/verification gates | Agreed stable scope, opt-in sign-in startup, manual-update link, packaged upgrade validation |
| Landing page | P9.2 when requested | Product/download page in docs, preserving plans |
| Later features | P5.2, P5.3, P6.2, P7, P8 and later platform adapters | Deferred Android parity and enhancements |

Preview A may use synthetic fixtures while developing but is usable only after real tracking, lyric lookup and caching are exercised. A preview is not permission to cut the agreed stable scope. Task checkboxes remain unchecked until implementation is verified.
