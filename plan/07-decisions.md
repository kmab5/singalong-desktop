# Confirmed decisions and technical gates

Updated 2026-11-13 after user confirmation. The [confirmed release scope](09-confirmed-release-scope.md) is authoritative for product decisions; implementation remains unstarted.

| ID | Decision | Status |
| --- | --- | --- |
| D01 | Tauri 2 + Rust + React + TypeScript + Vite | Confirmed |
| D02 | Root-level plan/ outside hosting; docs reserved for landing page | Confirmed; location revised by user after grilling |
| D03 | Rust owns domain, persistence and shared windows | Confirmed |
| D04 | Desktop-adapted UI preserving Singalong identity | Confirmed |
| D05 | Windows first; Linux/macOS later | Confirmed |
| D06 | Android 1.3.0 pinned commit remains behavior reference | Source baseline; confirmed desktop differences take precedence |
| D07 | Android import and full desktop backup/merge restore in stable | Confirmed; selective exports later |
| D08 | Pages solely a product/download landing page | Confirmed; no app demo/browser edition |
| D09 | Sign-in startup available, off by default, launches into tray | Confirmed |
| D10 | SQLite and atomic desktop restore | Architecture retained |
| D11 | Local language detector replacement | Deferred with translation |
| D12 | Saved-song browsing independent of live floating reader | Confirmed |
| D13 | Enabled apps only; automatic selection sticks through pause | Confirmed desktop difference |
| D14 | App/stream pins wait without fallback; app pins persist | Confirmed; stream reconnect requires reliable identity |
| D15 | Auto-show overlay opt-in; manual hiding wins | Confirmed |
| D16 | Ten-second hidden/background collection; immediate open-reader lookup | Confirmed |
| D17 | Android possible-match, estimates, offsets and supplied word timing | Confirmed stable scope |
| D18 | Any connection plus Offline mode; Wi-Fi-only later | Confirmed desktop difference |
| D19 | Essential appearance, three modes and optional click-through | Confirmed; glow/capture exclusion later |
| D20 | Manual updates; Check for updates opens release page | Confirmed |
| D21 | Preserve imported translations and inert unsupported settings | Confirmed; no automatic consent/permission changes |
| D22 | Fixed first-stable scope delivered through usable previews | Confirmed |

## Remaining technical gates

These are validation/implementation work, not requests to reopen settled product choices:

- Prove Windows session access from the actual package and determine the supported OS floor. Record native Media Player identity/version, browser/website combinations and timing fidelity.
- Validate app versus stream identity. Specify deterministic automatic selection tie-breaks and same-app multi-stream handling without broadening a stream pin or abandoning paused ownership.
- Choose measured stale-position refresh policy; absence of callbacks alone must not freeze a valid session.
- Verify lock/suspend, click-through recovery, startup registration, DPI/monitor behavior and installer upgrades on Windows.
- Define full desktop backup schema and safe inert Android-preference retention. Reverse Android export stays deferred until independently tested.
- Pin dependency/toolchain versions, validate licensing/notices and record signing availability.
- Later, evaluate Linux/macOS adapters, a local language detector and remaining Android feature implementations.

## Evidence and authority

The Android audit and file inventory remain historical source evidence. Confirmed desktop decisions override Android behavior where explicitly different, especially selection, network policy and release scope. Preserve original Android descriptions rather than rewriting history.

No app scaffold, dependency installation, runtime tests, generated website or publication is authorized by this documentation update. Proposed implementation paths remain proposals.

Official API references remain in [architecture](02-desktop-architecture.md), [platform integration](03-platform-integration.md), [data compatibility](04-data-compatibility.md) and [verification](06-verification-release.md). Recheck them during implementation; product confirmation is not proof of API support.
