# Landing page

This directory is reserved solely for the future GitHub Pages product/download landing page. There is no site scaffold or deployment workflow yet.

## Publishing boundary

- Publish only this directory or a curated landing-page build artifact.
- Planning documents and implementation evidence live in the root-level `plan/` directory, outside this publishing tree. Do not copy them or agent guidance into site output.
- Future `index.html`, `assets/`, `site.css` and `site.js` serve the product/download page. No public app demo or standalone browser edition is in scope.
- Application source lives outside this directory. The packaged Tauri frontend builds to `dist/`, never directly into `docs/`.
- Stage generated landing-page output separately and copy only declared assets. Do not empty authored directories or publish the repository root.

The installed Tauri app bundles its own frontend and does not depend on the landing page. No privileged remote Pages content will be loaded into the app.
