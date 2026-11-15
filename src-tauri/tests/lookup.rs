use singalong_core::{
    lyrics::matching::{self, ProviderSong},
    model::*,
};
use singalong_desktop::{
    platform::Cancellation,
    providers::LookupOutcome,
    runtime::lookup::{Coordinator, Jobs},
    storage::Storage,
};
use std::sync::{Arc, Mutex, atomic::AtomicU64};
#[derive(Default)]
struct State {
    starts: usize,
    next: u64,
    result: Option<(u64, LookupOutcome)>,
    cancels: usize,
}
struct Fake(Arc<Mutex<State>>);
impl Jobs for Fake {
    fn start(&mut self, _: Recording) -> Option<u64> {
        let mut s = self.0.lock().unwrap();
        s.starts += 1;
        s.next += 1;
        Some(s.next)
    }
    fn cancel(&mut self) {
        self.0.lock().unwrap().cancels += 1;
    }
    fn poll(&mut self) -> Option<(u64, LookupOutcome)> {
        self.0.lock().unwrap().result.take()
    }
}
fn session(title: &str) -> PlaybackSnapshot {
    PlaybackSnapshot {
        session_id: "session".into(),
        player_id: PlayerId {
            platform: Platform::Windows,
            native_id: "fixture".into(),
        },
        player_name: "Fixture".into(),
        recording: Some(Recording {
            id: title.into(),
            title: title.into(),
            artists: vec!["Fixture artist".into()],
            album: Some("Fixture album".into()),
            duration_ms: Some(Counter::new(120000)),
            version_label: None,
            file_name: None,
        }),
        status: PlaybackStatus::Playing,
        position_ms: Some(Counter::new(1000)),
        observed_at_ms: Counter::new(0),
        rate: Some(1.0),
        stream_pin_available: false,
    }
}
fn outcome(s: &PlaybackSnapshot) -> LookupOutcome {
    let song:ProviderSong=serde_json::from_value(serde_json::json!({"id":1,"trackName":s.recording.as_ref().unwrap().title,"artistName":"Fixture artist","albumName":"Fixture album","duration":120,"syncedLyrics":"[00:01]Original fixture"})).unwrap();
    LookupOutcome::Found(matching::versions(
        s.recording.as_ref().unwrap(),
        vec![song],
        false,
    ))
}
fn token() -> Cancellation {
    Cancellation {
        epoch: Arc::new(AtomicU64::new(0)),
        expected: 0,
    }
}
#[test]
fn immediate_reader_fetch_waits_for_qualification_to_save_and_shares_requests() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let state = Arc::new(Mutex::new(State::default()));
    let mut c = Coordinator::new(db.clone(), Box::new(Fake(state.clone())));
    let s = session("Original song");
    let t = token();
    c.update(Some(&s), None, false, false, 0, &t);
    assert_eq!(state.lock().unwrap().starts, 0);
    c.update(Some(&s), None, true, false, 0, &t);
    c.update(Some(&s), None, true, false, 100, &t);
    assert_eq!(state.lock().unwrap().starts, 1);
    state.lock().unwrap().result = Some((1, outcome(&s)));
    c.update(Some(&s), None, true, false, 200, &t);
    assert!(matches!(c.state.status, LookupStatus::Ready));
    assert!(db.list(0, 10).unwrap().is_empty());
    assert_eq!(c.state.active_line, Some(0));
    let saved = db.resolve(s.recording.clone().unwrap(), None).unwrap();
    c.update(Some(&s), Some(&saved.id), false, false, 10000, &t);
    assert!(db.selected(saved.id.clone()).unwrap().is_some());
    assert_eq!(state.lock().unwrap().starts, 1);
    let mut reopened = Coordinator::new(db, Box::new(Fake(state.clone())));
    reopened.update(Some(&s), None, true, true, 10001, &t);
    assert!(matches!(reopened.state.status, LookupStatus::Ready));
    assert!(reopened.state.cached);
    assert_eq!(state.lock().unwrap().starts, 1);
}
#[test]
fn stale_offline_and_stopped_results_cannot_replace_or_save() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let state = Arc::new(Mutex::new(State::default()));
    let mut c = Coordinator::new(db.clone(), Box::new(Fake(state.clone())));
    let a = session("One");
    let b = session("Two");
    let t = token();
    c.update(Some(&a), None, true, false, 0, &t);
    c.update(Some(&b), None, true, false, 1, &t);
    state.lock().unwrap().result = Some((1, outcome(&a)));
    c.update(Some(&b), None, true, false, 2, &t);
    assert!(c.state.version.is_none());
    c.update(Some(&b), None, true, true, 3, &t);
    state.lock().unwrap().result = Some((2, outcome(&b)));
    c.update(Some(&b), None, true, true, 4, &t);
    assert!(matches!(c.state.status, LookupStatus::Offline));
    c.update(None, None, true, false, 5, &t);
    assert!(matches!(c.state.status, LookupStatus::Idle));
    assert!(db.list(0, 10).unwrap().is_empty());
}
#[test]
fn a_personal_selection_made_during_lookup_wins_the_save_race() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let state = Arc::new(Mutex::new(State::default()));
    let mut c = Coordinator::new(db.clone(), Box::new(Fake(state.clone())));
    let s = session("Original song");
    let t = token();
    c.update(Some(&s), None, true, false, 0, &t);
    let saved = db.resolve(s.recording.clone().unwrap(), None).unwrap();
    let LookupOutcome::Found(mut versions) = outcome(&s) else {
        unreachable!()
    };
    versions[0].id = "personal".into();
    versions[0].recording_id = saved.id.clone();
    versions[0].origin = LyricOrigin::Personal;
    db.save_versions(saved.id.clone(), versions).unwrap();
    state.lock().unwrap().result = Some((1, outcome(&s)));
    c.update(Some(&s), Some(&saved.id), false, false, 10000, &t);
    assert_eq!(c.state.version.as_ref().unwrap().id, "personal");
    assert_eq!(db.selected(saved.id).unwrap().unwrap().id, "personal");
}

#[test]
fn personal_selection_wins_before_qualification_and_expired_timing_is_manual() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let state = Arc::new(Mutex::new(State::default()));
    let mut c = Coordinator::new(db.clone(), Box::new(Fake(state.clone())));
    let mut s = session("Original song");
    let t = token();
    c.update(Some(&s), None, true, false, 0, &t);
    let saved = db.resolve(s.recording.clone().unwrap(), None).unwrap();
    let LookupOutcome::Found(mut versions) = outcome(&s) else {
        unreachable!()
    };
    versions[0].id = "personal".into();
    versions[0].recording_id = saved.id.clone();
    versions[0].origin = LyricOrigin::Personal;
    db.save_versions(saved.id.clone(), versions).unwrap();
    state.lock().unwrap().result = Some((1, outcome(&s)));
    c.update(Some(&s), None, true, false, 100, &t);
    assert_eq!(c.state.version.as_ref().unwrap().id, "personal");
    assert_eq!(db.versions(saved.id).unwrap().len(), 1);
    c.update(Some(&s), None, true, false, 3001, &t);
    assert!(!c.state.position_available);
    assert!(c.state.active_line.is_none());
    s.observed_at_ms = Counter::new(3001);
    s.status = PlaybackStatus::Paused;
    s.position_ms = Some(Counter::new(500));
    c.update(Some(&s), None, true, false, 3001, &t);
    assert!(c.state.position_available);
    assert!(c.state.active_line.is_none());
    s.position_ms = Some(Counter::new(1000));
    s.observed_at_ms = Counter::new(3100);
    c.update(Some(&s), None, true, false, 3100, &t);
    assert_eq!(c.state.active_line, Some(0));
}

struct Immediate {
    id: u64,
    result: Option<(u64, LookupOutcome)>,
}
impl Jobs for Immediate {
    fn start(&mut self, r: Recording) -> Option<u64> {
        self.id += 1;
        let mut s = session(&r.title);
        s.recording = Some(r.clone());
        let LookupOutcome::Found(mut versions) = outcome(&s) else {
            unreachable!()
        };
        versions[0].lines[0].text = r.title;
        self.result = Some((self.id, LookupOutcome::Found(versions)));
        Some(self.id)
    }
    fn cancel(&mut self) {
        self.result = None;
    }
    fn poll(&mut self) -> Option<(u64, LookupOutcome)> {
        self.result.take()
    }
}
struct ChangingAdapter(Arc<AtomicU64>);
impl singalong_desktop::platform::Adapter for ChangingAdapter {
    fn changed(&self) -> bool {
        true
    }
    fn refresh(
        &mut self,
        _: &[PlayerId],
        now: u64,
        _: &Cancellation,
    ) -> Result<singalong_desktop::platform::Observation, &'static str> {
        let mut s = session(if self.0.load(std::sync::atomic::Ordering::SeqCst) == 0 {
            "First"
        } else {
            "Second"
        });
        s.observed_at_ms = Counter::new(now);
        Ok(singalong_desktop::platform::Observation {
            players: vec![PlayerInfo {
                id: s.player_id.clone(),
                name: "Fixture".into(),
                enabled: true,
                available: true,
            }],
            sessions: vec![s],
        })
    }
}
#[test]
fn every_emitted_snapshot_keeps_track_and_lyrics_together_across_change_and_stop() {
    use std::{
        sync::atomic::Ordering,
        thread,
        time::{Duration, Instant},
    };
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let mut settings = db.settings().unwrap();
    settings.tracking_enabled = true;
    settings.enabled_players = vec![session("First").player_id];
    db.save_settings(settings).unwrap();
    let stage = Arc::new(AtomicU64::new(0));
    let adapter = stage.clone();
    let snapshots = Arc::new(Mutex::new(Vec::<RuntimeSnapshot>::new()));
    let output = snapshots.clone();
    let runtime = singalong_desktop::runtime::Runtime::spawn_with_jobs(
        db,
        Arc::new(move |s| output.lock().unwrap().push(s)),
        Box::new(move |_| Ok(Box::new(ChangingAdapter(adapter.clone())))),
        Box::new(Immediate {
            id: 0,
            result: None,
        }),
    )
    .unwrap();
    let wait = |title: &str| {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if runtime
                .snapshot()
                .reader
                .version
                .as_ref()
                .is_some_and(|v| v.lines[0].text == title)
            {
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        }
    };
    wait("First");
    stage.store(1, Ordering::SeqCst);
    wait("Second");
    runtime
        .action(RuntimeAction::Tracking { enabled: false })
        .unwrap();
    runtime.shutdown();
    for snapshot in snapshots.lock().unwrap().iter() {
        if let Some(version) = &snapshot.reader.version {
            let selected = snapshot
                .sessions
                .iter()
                .find(|s| Some(&s.session_id) == snapshot.selected_session_id.as_ref())
                .expect("lyrics without a selected session");
            assert_eq!(
                version.lines[0].text,
                selected.recording.as_ref().unwrap().title
            );
            assert!(snapshot.tracking_enabled);
        }
    }
}
