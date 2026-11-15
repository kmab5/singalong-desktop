use singalong_core::model::*;

#[test]
fn revisions_survive_javascript_precision_limits_and_reject_malformed_values() {
    let counter = Counter::new(u64::MAX);
    assert_eq!(
        serde_json::to_string(&counter).unwrap(),
        "\"18446744073709551615\""
    );
    for value in [
        "\"-1\"",
        "\"01\"",
        "\"18446744073709551616\"",
        "123",
        "\"1e3\"",
    ] {
        assert!(serde_json::from_str::<Counter>(value).is_err(), "{value}");
    }
}

#[test]
fn unknown_timing_and_unicode_survive_the_wire_without_inventing_zeroes() {
    let mut snapshot = RuntimeSnapshot::foundation();
    snapshot.sessions.push(PlaybackSnapshot {
        session_id: "ephemeral-1".into(),
        player_id: PlayerId {
            platform: Platform::Windows,
            native_id: "synthetic".into(),
        },
        player_name: "播放器 🎵".into(),
        recording: None,
        status: PlaybackStatus::Paused,
        position_ms: None,
        observed_at_ms: Counter::new(9007199254740993),
        rate: None,
    });
    let value = serde_json::to_value(&snapshot).unwrap();
    assert!(value["sessions"][0]["positionMs"].is_null());
    assert_eq!(value["sessions"][0]["playerName"], "播放器 🎵");
    assert_eq!(value["sessions"][0]["observedAtMs"], "9007199254740993");
}
