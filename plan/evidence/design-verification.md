# Design review and automated verification

Date: 2026-11-15. Scope: P3.2/P4 reader presentation, P5.1 library/search presentation, P6 backup regression coverage, P9.1 settings presentation and build checks. The user resumed automated verification while excluding live testing. This record supersedes the earlier testing pause; it does not close native compatibility or stable-release gates.

## Interface changes

Preserved the established dark/violet Windows system-font identity. Added durable interface rules in DESIGN.md and consolidated the stylesheet into consistent colors, typography, spacing, control sizes and focus states. Reviewed against Impeccable craft guidance and the [Web Interface Guidelines](https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md).

Playback source selection is a named disclosure so it no longer pushes lyrics off the initial main view. Version/timing tools follow the lyrics. Settings have Playback, Appearance, Library data, Backups and About sections. Appearance edits are local drafts with explicit Save and Reset, including reset of an unsaved draft. Added a keyboard skip link, accessible docking names, search input names and appropriate inline confirmation semantics.

The compact reader fits its native 440 × 360 default. Screenshot review exposed an existing scroll-coordinate error: the active line could be outside the clipped lyric viewport. Corrected the relative-coordinate calculation and observe reader resizing to keep the active line centred while following. Manual scrolling still suspends following.

Synthetic screenshot review covered Now, Appearance, narrow search/backups, and compact/expanded/minimal floating layouts. Default floating sizes were 440 × 360, 560 × 720 and 320 × 110. Main checks used 1120 × 760 and 620 × 480. A regression explicitly requires the active line to fit inside the lyric viewport. No real song, provider request, native app, media session, live database or file dialog was used.

Local screenshots are generated under ignored artifacts/: design-now.png, design-settings.png, design-search-narrow.png, design-backups-narrow.png, and design-floating-{compact,expanded,minimal}.png. They contain only original synthetic text. No screenshot or fixture runtime is deployed to Pages.

## Results

Toolchain: Node 24.22.1, Rust 1.97.1, installed Chrome 157.0.8103.51 in headless mode; Tauri 2.11.5. Each final command below exited 0.

| Check | Result |
| --- | --- |
| npm run build | TypeScript check and production Vite bundle passed |
| npm run build:fixture | Separate local fixture bundle passed |
| npm run contract:check | Generated IPC contract/types/validator drift check passed |
| npm test -- --run | 8 frontend boundary tests passed, Vitest 4.1.12 |
| npm run test:e2e | 8 headless Chrome tests passed; final run 13.5 seconds |
| axe-core 4.14.0, embedded in design browser tests | No WCAG 2 A/AA or 2.1 AA rule violations on the ten tested screen states: Now, five settings sections, library search, three floating modes |
| cargo test --locked --manifest-path src-tauri/Cargo.toml --workspace --features desktop -j 1 | 73 tests passed, none failed or ignored |
| cargo clippy --locked --manifest-path src-tauri/Cargo.toml --workspace --all-targets --features desktop -j 1 -- -D warnings | Passed; mechanical lint cleanup and worker context grouping reviewed |
| cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check | Passed |
| cargo build --locked --manifest-path src-tauri/Cargo.toml --features desktop --bin singalong-desktop -j 1 | Windows debug executable built; not launched |
| npm run check:native-manifest -- -Executable src-tauri/target/debug/singalong-desktop.exe | Embedded Common Controls v6 dependency verified |
| npm audit --audit-level=moderate | Zero reported npm vulnerabilities after Vitest patch update |
| Impeccable mechanical detector | No findings on reviewed UI targets |
| git diff --check | Passed |

Added fifteen Rust tests: six codec tests, six storage merge tests and three estimated/word-timing tests. They cover desktop round trips and inert data, integrity/schema/resource rejection, idempotence, conflicting immutable versions, preview and transaction rollback, pre-cancellation, null selection preservation, imported-copy cleanup protection and timing boundaries. These do not cover every backup edge case or cancellation interleaving.

The first parallel desktop-feature test build exhausted compiler memory. Retrying with one build job passed without changing behavior to accommodate it. Cold browser loading twice exceeded the previous 30-second timeout while native compilation pressured memory. The final browser suite ran after Cargo finished, with a documented 90-second ceiling, and passed in 13.5 seconds. No assertions were disabled. Lint issues were fixed without blanket warning suppression.

CI configuration now checks the desktop executable manifest rather than an unbuilt probe, installs the Chrome channel used by Playwright, and includes desktop-feature tests and strict Clippy. The workflow itself was not remotely dispatched. Production dist contains only index.html and bundled JS/CSS; plan/ and docs/ are excluded and docs/ was not modified.

## Remaining gates

Source implemented, executable built, automated tests passed, and native manually verified remain separate claims. Automated accessibility scans are not a complete accessibility certification; transparency contrast over arbitrary desktop backgrounds and screen-reader use still need native review.

No live playback/seek, tray lifecycle, lock/suspend, startup registration, click-through, multi-monitor/DPI, native backup dialog, real Android-generated archive restore, installer/upgrade/uninstall, signing or distribution check ran. No installer was built or published. Large/ZIP64 archives, disk-full behavior, mid-operation cancellation and the broader compatibility matrix retain their outstanding plan gates. A release dependency-license/security review is also separate from the npm audit.

Next dependency: an explicitly resumed native verification session, followed by packaging/install/upgrade checks and release review. Do not call Preview A/B/C or stable fully verified from this record alone.

Local checkpoint ddab1fb contains the Rust regression additions and lint cleanup. The following UI checkpoint contains DESIGN.md, app/reader/settings/style changes, fixture route, browser/accessibility regressions, patched test tooling, CI corrections and this evidence. No push, release publication or Pages deployment accompanied these checks.
