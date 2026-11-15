use singalong_core::model::*;
use singalong_desktop::{platform::Observation, runtime::tracking::Engine, storage::Storage};
fn observed() -> Observation {
    let id = PlayerId {
        platform: Platform::Windows,
        native_id: "fixture-player".into(),
    };
    Observation {
        players: vec![PlayerInfo {
            id: id.clone(),
            name: "Fixture".into(),
            enabled: false,
            available: true,
        }],
        sessions: vec![PlaybackSnapshot {
            session_id: "session".into(),
            player_id: id,
            player_name: "Fixture".into(),
            recording: Some(Recording {
                id: String::new(),
                title: "Synthetic runtime song".into(),
                artists: vec!["Fixture".into()],
                album: None,
                duration_ms: Some(Counter::new(120000)),
                version_label: None,
                file_name: None,
            }),
            status: PlaybackStatus::Playing,
            position_ms: Some(Counter::new(0)),
            observed_at_ms: Counter::new(0),
            rate: Some(1.0),
            stream_pin_available: false,
        }],
    }
}
#[test]
fn disabled_apps_never_collect_and_enabled_play_qualifies_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let mut engine = Engine::new(db.clone()).unwrap();
    engine
        .apply(RuntimeAction::Tracking { enabled: true })
        .unwrap();
    engine.observe(observed(), 0, 1000).unwrap();
    engine.observe(observed(), 10000, 11000).unwrap();
    assert!(db.list(0, 20).unwrap().is_empty());
    engine
        .apply(RuntimeAction::EnablePlayer {
            player_id: observed().players[0].id.clone(),
            enabled: true,
        })
        .unwrap();
    engine.observe(observed(), 11000, 12000).unwrap();
    assert!(db.list(0, 20).unwrap().is_empty());
    engine.observe(observed(), 21000, 22000).unwrap();
    engine.observe(observed(), 22000, 23000).unwrap();
    assert_eq!(db.list(0, 20).unwrap()[0].play_count, 1);
    assert!(
        engine
            .apply(RuntimeAction::Select {
                mode: SelectionMode::StreamPin {
                    player_id: observed().players[0].id.clone(),
                    session_id: "session".into()
                }
            })
            .is_err()
    );
    engine
        .apply(RuntimeAction::Tracking { enabled: false })
        .unwrap();
    engine.observe(observed(), 32000, 33000).unwrap();
    assert!(engine.snapshot.sessions.is_empty());
    assert_eq!(db.list(0, 20).unwrap()[0].play_count, 1);
}
#[test]
fn app_pin_and_allowlist_survive_restart_but_do_not_enable_each_other() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let mut engine = Engine::new(db.clone()).unwrap();
    engine.observe(observed(), 0, 0).unwrap();
    let id = observed().players[0].id.clone();
    engine
        .apply(RuntimeAction::Select {
            mode: SelectionMode::AppPin {
                player_id: id.clone(),
            },
        })
        .unwrap();
    engine
        .apply(RuntimeAction::Tracking { enabled: true })
        .unwrap();
    let mut restored = Engine::new(db).unwrap();
    restored.observe(observed(), 0, 0).unwrap();
    assert_eq!(
        restored.snapshot.selection,
        SelectionMode::AppPin { player_id: id }
    );
    assert!(restored.snapshot.selected_session_id.is_none());
    assert_eq!(
        restored.snapshot.status_message,
        "Waiting for pinned player"
    );
}

#[test]
fn stop_cancels_pending_startup_and_never_publishes_its_result() {
    use singalong_desktop::runtime::Runtime;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::{Duration, Instant};
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let entered = Arc::new(AtomicBool::new(false));
    let signal = entered.clone();
    let runtime = Runtime::spawn(
        db,
        Arc::new(|_| {}),
        Box::new(move |cancel| {
            signal.store(true, Ordering::SeqCst);
            while !cancel.cancelled() {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err("cancelled")
        }),
    )
    .unwrap();
    runtime
        .action(RuntimeAction::Tracking { enabled: true })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !entered.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    runtime
        .action(RuntimeAction::Tracking { enabled: false })
        .unwrap();
    assert!(!runtime.snapshot().tracking_enabled);
    assert!(runtime.snapshot().sessions.is_empty());
    assert_eq!(runtime.snapshot().status_message, "Tracking stopped");
    runtime.shutdown();
}

#[test]
fn stop_restart_with_new_native_session_does_not_recount_the_same_play() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let mut e = Engine::new(db.clone()).unwrap();
    e.observe(observed(), 0, 0).unwrap();
    e.apply(RuntimeAction::EnablePlayer {
        player_id: observed().players[0].id.clone(),
        enabled: true,
    })
    .unwrap();
    e.apply(RuntimeAction::Tracking { enabled: true }).unwrap();
    e.observe(observed(), 0, 0).unwrap();
    e.observe(observed(), 10000, 10000).unwrap();
    e.apply(RuntimeAction::Tracking { enabled: false }).unwrap();
    e.apply(RuntimeAction::Tracking { enabled: true }).unwrap();
    let mut next = observed();
    next.sessions[0].session_id = "new-native-instance".into();
    e.observe(next, 11000, 11000).unwrap();
    let mut next = observed();
    next.sessions[0].session_id = "new-native-instance".into();
    e.observe(next, 21000, 21000).unwrap();
    assert_eq!(db.list(0, 20).unwrap()[0].play_count, 1);
}
