# Singalong desktop migration plans

Prepared 2026-11-13. Updated 2026-11-15. Status: **P1 foundation and storage implemented; native media/window probes exercised; remaining player, hardware and actual lock/suspend gates stay open. P2.1 tracking is implemented; P2.2 has reader-dependent and native lifecycle gates remaining. P3 lookup/cache and P4a reader source are implemented; verification remains incomplete. All stable-scope coding is authorized, with testing paused at the user’s request.**

Current evidence: [foundation](evidence/foundation.md), [storage](evidence/storage.md), [media sessions](evidence/platform-probes.md), [window lifecycle](evidence/window-probes.md). Reproduction commands: [probe runbook](../src-tauri/README.md).

Android baseline: `kmab5/singalong`, commit `453068332bb58e3d642d294ef861ba6ccd337952`, version 1.3.0. The inspected Android working tree was clean. Desktop initially contained only a README and Git metadata.

Read the [confirmed first stable-release scope](09-confirmed-release-scope.md) first. It records the user-approved decisions and supersedes earlier recommendations.

## Reading order

1. [Android product and source audit](01-android-audit.md): what exists, its rules, source ownership and parity map.
2. [Desktop architecture](02-desktop-architecture.md): boundaries, data flow, windows, frontend and IPC.
3. [Platform integration](03-platform-integration.md): Windows, Linux and macOS feasibility and fallbacks.
4. [Data and provider compatibility](04-data-compatibility.md): storage, Android backups, lookup, translation and editing.
5. [Implementation plan](05-implementation-plan.md): dependency-ordered tasks, proposed paths and acceptance criteria.
6. [Verification and release](06-verification-release.md): regression fixtures, native matrix, packaging and Pages.
7. [Decisions and open questions](07-decisions.md): explicit recommendations, alternatives and evidence needed.
8. [Android file inventory](08-source-inventory.md): tracked production, test, resource and tooling files for future agents.

Agent workflow rules are in [AGENTS.md](../AGENTS.md). Hosting rules are in [docs/README.md](../docs/README.md).

## Confirmed scope and delivery

Windows first with Tauri 2, Rust, React, TypeScript and Vite. Build a dependable daily lyrics companion through usable previews, keeping the agreed first stable scope fixed. See the confirmed scope for selection/pinning, tray/startup, reader behavior, offline library, compatibility targets and migration rules.

The first preview delivers real playback detection, player selection/pinning, lyric lookup, local caching and a basic floating reader. Later previews complete three reader modes, library tools and Android/full-desktop backups before stable. Full editing, folder scans, translation, cards, selective exports and other platforms follow stable.

GitHub Pages is solely the product/download landing page. It is separate from the bundled Tauri frontend; there is no public browser app or demo in scope.

The source audit did not run Android builds/device tests or validate desktop players. User approval confirms the plan, not implementation or runtime support. Start implementation only when requested, following [the revised task sequence](05-implementation-plan.md).

Current implementation: P1.1/P1.2 complete; P2.1 implemented and verified; P2.2 partial pending reader-dependent controls and native lifecycle checks. See [P2 evidence](evidence/tracking.md).

Current coding handoff: [stable implementation progress](evidence/stable-implementation.md). This separates source completion from deferred verification.
