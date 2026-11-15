use singalong_core::{
    clock::PlaybackClock, model::*, qualification::Qualifier, selection::Selector,
};
fn sample(id: &str, status: PlaybackStatus) -> PlaybackSnapshot {
    PlaybackSnapshot {
        session_id: id.into(),
        player_id: PlayerId {
            platform: Platform::Windows,
            native_id: id.into(),
        },
        player_name: id.into(),
        recording: Some(Recording {
            id: "song".into(),
            title: "Original fixture".into(),
            artists: vec!["Fixture artist".into()],
            album: None,
            duration_ms: Some(Counter::new(120000)),
            version_label: None,
            file_name: None,
        }),
        status,
        position_ms: Some(Counter::new(0)),
        observed_at_ms: Counter::new(0),
        rate: Some(1.0),
        stream_pin_available: true,
    }
}
#[test]
fn automatic_retains_pause_and_replaces_only_ineligible_or_stopped_owner() {
    let mut selector = Selector::default();
    let mut a = sample("a", PlaybackStatus::Playing);
    let b = sample("b", PlaybackStatus::Playing);
    let enabled = vec![a.player_id.clone(), b.player_id.clone()];
    assert_eq!(
        selector
            .select(&[b.clone(), a.clone()], &enabled, &SelectionMode::Automatic)
            .unwrap()
            .0,
        "a"
    );
    a.status = PlaybackStatus::Paused;
    assert_eq!(
        selector
            .select(&[a.clone(), b.clone()], &enabled, &SelectionMode::Automatic)
            .unwrap()
            .0,
        "a"
    );
    a.status = PlaybackStatus::Stopped;
    assert_eq!(
        selector
            .select(&[a, b.clone()], &enabled, &SelectionMode::Automatic)
            .unwrap()
            .0,
        "b"
    );
    assert!(
        selector
            .select(&[b], &[], &SelectionMode::Automatic)
            .is_none()
    );
}
#[test]
fn pins_wait_without_fallback_or_stream_widening() {
    let mut selector = Selector::default();
    let a = sample("a", PlaybackStatus::Playing);
    let b = sample("b", PlaybackStatus::Playing);
    let enabled = vec![a.player_id.clone(), b.player_id.clone()];
    let app = SelectionMode::AppPin {
        player_id: a.player_id.clone(),
    };
    assert!(
        selector
            .select(std::slice::from_ref(&b), &enabled, &app)
            .is_none()
    );
    assert!(
        selector
            .select(std::slice::from_ref(&a), &[], &app)
            .is_none()
    );
    assert_eq!(
        selector
            .select(std::slice::from_ref(&a), &enabled, &app)
            .unwrap()
            .0,
        "a"
    );
    let stream = SelectionMode::StreamPin {
        player_id: a.player_id.clone(),
        session_id: "old-instance".into(),
    };
    assert!(selector.select(&[a, b], &enabled, &stream).is_none());
}
#[test]
fn only_ten_seconds_of_observed_playback_qualify_once() {
    let mut q = Qualifier::default();
    let mut s = sample("a", PlaybackStatus::Playing);
    assert!(q.observe(Some(&s), 0).is_none());
    s.status = PlaybackStatus::Paused;
    assert!(q.observe(Some(&s), 6000).is_none());
    assert!(q.observe(Some(&s), 20000).is_none());
    s.status = PlaybackStatus::Playing;
    assert!(q.observe(Some(&s), 20000).is_none());
    s.position_ms = Some(Counter::new(90000));
    assert!(q.observe(Some(&s), 23999).is_none());
    assert!(q.observe(Some(&s), 24000).is_some());
    assert!(q.observe(Some(&s), 25000).is_none());
}
#[test]
fn gaps_suspend_and_track_changes_do_not_fabricate_listening() {
    let mut q = Qualifier::default();
    let mut s = sample("a", PlaybackStatus::Playing);
    q.observe(Some(&s), 0);
    assert!(q.observe(Some(&s), 60000).is_none());
    q.observe(Some(&s), 65000);
    q.interrupt();
    assert!(q.observe(Some(&s), 70000).is_none());
    s.recording.as_mut().unwrap().id = "other".into();
    assert!(q.observe(Some(&s), 75000).is_none());
    assert!(q.observe(Some(&s), 85000).is_some());
    q.observe(None, 86000);
    assert!(q.observe(Some(&s), 87000).is_none());
}
#[test]
fn end_to_start_repeat_creates_another_occurrence() {
    let mut q = Qualifier::default();
    let mut s = sample("a", PlaybackStatus::Playing);
    s.position_ms = Some(Counter::new(108000));
    q.observe(Some(&s), 0);
    s.position_ms = Some(Counter::new(119000));
    let first = q.observe(Some(&s), 10000).unwrap();
    s.position_ms = Some(Counter::new(0));
    assert!(q.observe(Some(&s), 11000).is_none());
    s.position_ms = Some(Counter::new(10000));
    assert_ne!(first, q.observe(Some(&s), 21000).unwrap());
}
#[test]
fn clock_keeps_anchor_refreshes_seek_and_freezes_unverifiable_position() {
    let mut clock = PlaybackClock::default();
    assert_eq!(
        clock.refresh(Some((1000, 1)), PlaybackStatus::Playing, Some(1.0), 0, 0),
        Some(1000)
    );
    assert_eq!(
        clock.refresh(
            Some((1000, 1)),
            PlaybackStatus::Playing,
            Some(1.0),
            5000,
            5000
        ),
        Some(6000)
    );
    assert_eq!(
        clock.refresh(Some((9000, 2)), PlaybackStatus::Paused, Some(1.0), 6000, 0),
        Some(9000)
    );
    assert_eq!(
        clock.refresh(
            Some((9000, 2)),
            PlaybackStatus::Paused,
            Some(1.0),
            30000,
            24000
        ),
        Some(9000)
    );
    assert_eq!(
        clock.refresh(None, PlaybackStatus::Playing, Some(1.0), 31000, 0),
        None
    );
    assert_eq!(
        clock.refresh(
            Some((10000, 3)),
            PlaybackStatus::Playing,
            Some(1.0),
            32000,
            16000
        ),
        None
    );
}
