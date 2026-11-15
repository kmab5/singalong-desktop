# Stable implementation progress

User authorization: finish the entire confirmed first-stable scope, then test. Testing is paused from 2026-11-15; do not run tests/probes or native/browser automation during coding. Existing passing evidence applies only to the recorded earlier revisions. Source completion below is not verification or release readiness.

| Area | Coding state | Verification |
| --- | --- | --- |
| P1/P2 foundation, storage, tracking/tray | Implemented; reader controls now connected | Earlier evidence retained; native lifecycle/compatibility gates open |
| P3.1 lookup/cache | Implemented, shared worker/coordinator and offline cache | Earlier parser/provider/coordinator evidence; newest fixes untested |
| P4a basic floating reader | Implemented, narrow local-only window and shared lyrics | Live display/hide observed earlier; lifecycle fixes untested |
| P3.2 full reading controls | In progress next | Deferred |
| P4 floating modes/geometry/appearance | Pending | Deferred |
| P5.1 library/search/correction/cleanup | Pending | Deferred |
| P6 Android/full desktop backups | Codec implementation in progress | Deferred |
| P9.1 settings/diagnostics/manual updates/packaging | Pending | Deferred; signing/distribution remain external gates |

Deferred features and the future landing page remain outside this work. No pushing, installer publication or Pages deployment is authorized by this implementation request. Checkpoints are local commits; unverified coding checkpoints are not tagged as verified releases.

Source handoff: current changes cover Rust runtime/storage/window coordination, generated IPC, shared reader UI and settings. Immediate next dependencies are reader context commands/settings, library commands and the neutral validated backup model. The development app may still be running an earlier build; no automation will touch it while testing is paused.
