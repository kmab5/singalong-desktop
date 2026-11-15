# Working on Singalong Desktop

## Current scope

P1/P2 foundation, storage and Windows tracking are implemented. P3 lookup/cache and the basic P4a floating reader are implemented in source; prior native checks observed real provider lyrics and floating display, but Preview A verification is incomplete. The user has now authorized coding the entire confirmed first-stable scope through Preview A/B/C and P9.1, with all testing postponed until coding is finished. Do not run tests, probes, browser/native automation or launch the app during this implementation-only phase. Source review, formatting and contract generation are allowed. Record unverified changes honestly and keep release gates open. See [implementation progress](plan/evidence/stable-implementation.md), [Preview A evidence](plan/evidence/preview-a.md), and [confirmed scope](plan/09-confirmed-release-scope.md).

The Android reference is the sibling repository `../singalong`, audited at commit `453068332bb58e3d642d294ef861ba6ccd337952` (1.3.0). See the source map and pinned links in the audit. Treat it as read-only unless work there is explicitly requested. Its historical release authorizations do not authorize desktop publication.

## Structure and ownership

- `plan/`: audit, architecture, implementation sequence, compatibility and verification records. Keep this root-level directory outside all landing-page build/deployment artifacts.
- `docs/`: reserved for the future GitHub Pages frontend; never set a build tool to empty this directory.
- Proposed app code belongs in `src/` and `src-tauri/`; generated application assets belong in `dist/`.
- Keep public documentation safe for publication. No user lyrics, backups, audio, databases, listening logs, local paths from users, keys or credentials.
- Keep durable agent instructions here. Record architectural decisions and evidence in plans rather than accumulating session transcripts.

## Implementation rules

- Implement only the phase requested. Update its checklist and evidence as work finishes.
- Before coding, inspect the current repository and the relevant Android files/tests. The plans describe a proposed layout, not existing APIs.
- Use Tauri 2 and Rust. React + TypeScript + Vite and Windows-first delivery are confirmed choices.
- Rust owns playback selection, qualified plays, providers, persistence, restore and shared window state. UI owns presentation and temporary editing interactions.
- Generate or validate IPC types; never let main and floating windows independently collect plays or fetch the same song.
- Use monotonic time for playback and listening intervals; wall time for history and quota dates. Re-anchor on seek, resume and adapter changes.
- Preserve original metadata, source provenance, distinct recordings, explicit lyric selections and personal edits.
- Supplied timing, estimated timing, uncertain matches and unavailable features must be distinguishable.
- Cancellation, shutdown, permission loss and deletion must prevent stale UI updates and late resurrection of removed content.
- Validate commands in Rust, including caller window. Tauri plugin capabilities alone do not automatically restrict custom commands.
- Keep filesystem access limited to private app data and explicitly selected files/folders. Do not add arbitrary URL fetch or shell-execution commands.
- Use synthetic lyrics/artwork in tests, demos and the future website. Carry source/font licenses when reusing assets.
- No microphone capture, audio recognition, scraping, paid/key-based provider, account system or cloud sync is included in this port.
- Do not add startup registration or global shortcuts silently. Tracking remembered across an explicit app launch is distinct from starting at OS login.

## Confirmed product rules

- Implement the agreed stable scope through Preview A/B/C and stable-candidate gates. Deferred features are not stable blockers; see the scope/task mapping.
- Only enabled apps are eligible. Automatic selection retains the current session through pause until stop/disappearance; do not blindly port Android PlayerSelector behavior.
- Pins wait without fallback. Persist app pins; reconnect stream pins only with reliable identity. Never silently widen a stream pin to an app/browser pin.
- Closing the main window keeps tracking in the tray. Quit stops everything. Restore prior tracking on explicit launch; sign-in startup is a separate opt-in, off by default, launching into the tray.
- Automatic floating display is opt-in. Manual hide/minimize wins over auto-show and automatic collapse recovery. Include click-through with a tray escape.
- Collect after ten seconds of actual playback even while hidden; fetch immediately for an open reader. Keep library browsing independent of live floating lyrics.
- Default lookup to any connection and offer Offline mode. Wi-Fi-only is deferred; disclose unsupported Android preferences without applying them.
- Stable includes Android backup import and full desktop backup/merge restore. Retain imported translations/inert preferences without enabling deferred services, consent, tracking or startup registration.
- Full editing, TXT/LRC import UI, folder scans, translation UI/services, cards, separate History screen and selective export controls are later work.
- Pages is solely a product/download landing page. Local fixture runtimes are development/testing tools, not a public demo.
- Manual updates only for stable; Check for updates opens the release page.

## Tagging ethics

Follow [TAGGING.md](TAGGING.md). Tag meaningful verified checkpoints liberally using annotated `vA.B.C-D` tags: A major, B feature, C fix, D a repository-wide increasing build/checkpoint number. Increment D for every new tag; never reset it or move a published tag. Documentation-only tags must say no executable was built. Initial planning checkpoint: `v0.0.0-1`.

A request to commit/tag/push authorizes those Git operations for the requested milestone; it does not authorize installer releases or Pages deployment. Push only the intended branch and explicit tag, without force.

The user requested regular commits at meaningful development junctions. Commit verified milestones and keep task evidence current; do not accumulate an entire phase's unrelated changes before committing. This standing instruction authorizes local checkpoint commits and the tagging policy above, not remote pushes or publication.

## Verification and handoff

Run the Cargo probes in [the probe runbook](src-tauri/README.md) and the foundation checks in [foundation evidence](plan/evidence/foundation.md). Run `npm run build` before compiling the desktop feature, since its bundled assets come from `dist/`. `npm run dev:fixture` is an explicit local test runtime; `npm run desktop` starts the native app. Browser tests do not establish native window or media-session compatibility. Record OS/player/build versions and observed results for native checks.

For every handoff state: files changed, completed task IDs, commands/results, unverified behavior and next dependency. Keep source implemented, build passed, tests passed and manually verified as separate claims. Do not commit, tag, push or publish as an incidental part of a documentation task.
