# Tagging ethics

Use annotated Git tags liberally for meaningful, verified checkpoints: planning milestones, usable previews, features, fixes and release candidates. A tag records an exact repository state; it does not by itself claim a stable application, successful runtime testing or a published binary release.

## Format

Every version tag uses `vA.B.C-D`:

| Part | Meaning | Rule |
| --- | --- | --- |
| A | Major release | Increment for a major product release or breaking change; reset B and C to zero |
| B | Feature release | Increment for a feature release within the current major; reset C to zero |
| C | Fix | Increment for fixes within the current feature release |
| D | Build number | Positive, repository-wide increasing checkpoint/build number; increment for every new version tag, including documentation-only checkpoints |

A, B and C are nonnegative integers. D starts at 1 and never resets when A, B or C changes. Use no leading zeroes. For a checkpoint without a major/feature/fix release, keep A.B.C and increment D. Where changes span categories, use the highest applicable category.

Examples of a sequence:

- `v0.0.0-1`: confirmed planning baseline; no desktop executable yet.
- `v0.0.0-2`: another verified planning/build checkpoint.
- `v0.1.0-3`: feature release.
- `v0.1.1-4`: fix release.
- `v1.0.0-5`: major release.

These examples explain the convention, not a commitment to future release numbers. Build number means versioned repository checkpoint, not proof that an application binary was built. This is the project's tag convention; future package/updater version mapping must preserve it without relying on lexicographic tag order.

## Tag responsibly

- Tag completed, useful checkpoints frequently. Do not tag every intermediate edit or claim unperformed verification.
- Check local and remote tags before choosing a number. D must exceed every existing build number; do not reuse numbers.
- Run checks appropriate to the change and review the exact staged files before committing. Documentation-only milestones need documentation checks, not a fabricated app build result.
- Create an annotated tag on the reviewed commit. Its message identifies the milestone, verification performed and important limits such as planning-only or preview status.
- Published tags are immutable. Never force-move, overwrite or delete a published tag to repair a release; make a new commit and increment the version/build as appropriate.
- When push is authorized, push the intended branch and explicit tag together, atomically where supported. Do not push unrelated local tags or force-push over remote work.
- Tags and binary/website publication are separate actions. Do not create a GitHub Release, upload installers or deploy Pages merely because a tag exists.

## Initial checkpoint

`v0.0.0-1` marks the Android audit, confirmed Windows scope, preview roadmap, root-level plan directory, agent guidance and this tagging policy. It is a planning-only checkpoint. No desktop implementation, executable build or player compatibility result is claimed.
