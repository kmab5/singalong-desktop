# Singalong Desktop

Desktop port of the [Singalong Android lyrics companion](https://kmab5.github.io/singalong/), planned for Windows first with Tauri 2, Rust, React, TypeScript and Vite.

The native application foundation runs with React/TypeScript, validated Rust IPC and a local SQLite worker. Enabled Windows playback tracking, selection/pinning and qualified history are connected. Lyric lookup, the floating reader and a usable Preview A are still upcoming. See [foundation evidence](plan/evidence/foundation.md), [storage evidence](plan/evidence/storage.md) and the [native probes](src-tauri/README.md).

See [tracking evidence and remaining gates](plan/evidence/tracking.md) for close-to-tray, startup and lifecycle verification.

Use Node 24.22.1 and Rust 1.97.1. Run `npm ci`, then `npm run desktop` to build and open the native shell. `npm run dev:fixture` is a separate, explicitly labelled local testing mode with synthetic content. It is not a public web edition.

Start with the [migration plan index](plan/README.md), including the Android feature audit, desktop architecture, platform integration strategy, data compatibility, phased implementation and verification checklist.

The [first stable-release scope](plan/09-confirmed-release-scope.md) is confirmed and will be delivered through smaller usable previews.

The `docs/` directory is reserved for the future GitHub Pages product/download landing page. Plans live under `plan/`; agent guidance lives in [AGENTS.md](AGENTS.md).

Version tags follow [the tagging policy](TAGGING.md): `vA.B.C-D` for major, feature, fix and build number. The initial planning checkpoint is `v0.0.0-1`.

Made with love by [kmab](https://kmab5.github.io).
