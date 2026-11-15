# P9.1 packaging source

Installer source targets a per-user Windows NSIS package with WebView2 bootstrap support. The app identifier is unchanged. License, source notices, privacy text and offline release notes are explicitly bundled. No plan/ or docs/ files are packaged. The UI bundles the same release-notes source used for distribution.

scripts/package-windows.ps1 requires an explicit exact Tauri CLI version; no CLI was installed or installer built during the testing pause. Candidate workflow execution is manual, has read-only repository permission and uploads a short-lived workflow artifact; it never creates a GitHub release or deploys Pages. Selecting/pinning the CLI used for the candidate, dependency license inventory, signing configuration and installer/upgrade/uninstall checks remain release gates. No secrets or signing identities are present in source.

The checks workflow defines later Rust/frontend/browser checks; none were triggered by this implementation work. A future passing CI run still cannot establish native player compatibility or installer behavior. Check for updates only opens the fixed GitHub releases page.

No publication has been authorized by this source implementation checkpoint.
