//! Opt-in integration harness. Requires the isolated synthetic Chrome fixture.
//! All history is stored in a temporary database; no song metadata is printed.
use singalong_core::model::*;
use singalong_desktop::{runtime::Runtime, storage::Storage};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn until(runtime: &Runtime, seconds: u64, condition: impl Fn(&RuntimeSnapshot) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while !condition(&runtime.snapshot()) {
        assert!(
            Instant::now() < deadline,
            "Native runtime condition timed out"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
fn main() {
    assert!(
        std::env::args().any(|a| a == "--synthetic-chrome"),
        "Requires explicit synthetic Chrome harness flag"
    );
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("probe.sqlite")).unwrap();
    let runtime = Runtime::native(db.clone(), Arc::new(|_| {})).unwrap();
    runtime.action(RuntimeAction::Discover).unwrap();
    until(&runtime, 8, |s| {
        s.players
            .iter()
            .any(|p| p.id.native_id.to_lowercase().contains("chrome"))
    });
    let player = runtime
        .snapshot()
        .players
        .into_iter()
        .find(|p| p.id.native_id.to_lowercase().contains("chrome"))
        .unwrap();
    assert!(!player.enabled);
    runtime
        .action(RuntimeAction::EnablePlayer {
            player_id: player.id.clone(),
            enabled: true,
        })
        .unwrap();
    runtime
        .action(RuntimeAction::Tracking { enabled: true })
        .unwrap();
    until(&runtime, 8, |s| s.selected_session_id.is_some());
    let snapshot = runtime.snapshot();
    assert_eq!(
        snapshot.sessions.len(),
        1,
        "Use only isolated Chrome playback for this probe"
    );
    assert_eq!(
        snapshot.sessions[0].recording.as_ref().unwrap().title,
        "Singalong synthetic fixture"
    );
    assert!(!snapshot.sessions[0].stream_pin_available);
    runtime
        .action(RuntimeAction::Select {
            mode: SelectionMode::AppPin {
                player_id: player.id.clone(),
            },
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(16);
    while db.list(0, 20).unwrap().is_empty() {
        assert!(Instant::now() < deadline, "No qualified native play");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(db.list(0, 20).unwrap()[0].play_count, 1);
    runtime
        .action(RuntimeAction::Tracking { enabled: false })
        .unwrap();
    assert!(runtime.snapshot().sessions.is_empty());
    runtime
        .action(RuntimeAction::Tracking { enabled: true })
        .unwrap();
    until(&runtime, 8, |s| s.selected_session_id.is_some());
    std::thread::sleep(Duration::from_secs(11));
    assert_eq!(db.list(0, 20).unwrap().len(), 1);
    assert_eq!(db.list(0, 20).unwrap()[0].play_count, 1);
    runtime.shutdown();
    assert!(runtime.snapshot().sessions.is_empty());
    println!(
        "native_runtime_passed: explicit discovery, unchecked app, enabled-only tracking, metadata, app pin, ten-second qualification, rapid restart without recounting, shutdown; temporary database only"
    );
}
