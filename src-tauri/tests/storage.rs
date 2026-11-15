use singalong_core::model::*;
use singalong_desktop::storage::{Settings, Storage};

fn song() -> Recording {
    Recording {
        id: "incoming".into(),
        title: "Synthetic song".into(),
        artists: vec!["Fixture artist".into()],
        album: Some("Fixture album".into()),
        duration_ms: Some(Counter::new(120000)),
        version_label: None,
        file_name: None,
    }
}
fn version(recording: &str, id: &str) -> LyricVersion {
    LyricVersion {
        id: id.into(),
        recording_id: recording.into(),
        content_hash: format!("fixture-{id}"),
        origin: LyricOrigin::Personal,
        lines: vec![LyricLine {
            id: "line".into(),
            text: "An original sample line".into(),
            start_ms: None,
            end_ms: None,
            words: vec![],
        }],
        source_name: None,
        source_url: None,
        confidence: MatchConfidence::Confident,
        timing: TimingKind::Unsynced,
        provider_id: None,
        attribution: None,
        retention: "synthetic fixture".into(),
        provider_duration_ms: None,
    }
}
#[test]
fn reopen_preserves_content_selection_adjustments_favorites_and_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite");
    let id;
    {
        let db = Storage::open(path.clone()).unwrap();
        id = db.resolve(song(), None).unwrap().id;
        db.save_versions(id.clone(), vec![version(&id, "one"), version(&id, "two")])
            .unwrap();
        db.select(id.clone(), "two".into()).unwrap();
        db.adjust("two".into(), 250, true).unwrap();
        db.favorite(id.clone(), true).unwrap();
        let settings = Settings {
            text_size: 999,
            opacity: 0.01,
            ..Default::default()
        };
        db.save_settings(settings).unwrap();
    }
    let db = Storage::open(path).unwrap();
    assert_eq!(db.selected(id.clone()).unwrap().unwrap().id, "two");
    assert_eq!(db.adjustment("two".into()).unwrap(), (250, true));
    let rows = db.list(0, 20).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].favorite);
    let settings = db.settings().unwrap();
    assert_eq!(settings.text_size, 64);
    assert_eq!(settings.opacity, 0.2);
    assert!(!settings.tracking_enabled);
    assert!(!settings.translation_consent);
}
#[test]
fn immutable_conflict_rolls_back_entire_batch_and_selection_cannot_cross_recordings() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let id = db.resolve(song(), None).unwrap().id;
    db.save_versions(id.clone(), vec![version(&id, "one")])
        .unwrap();
    let mut conflicting = version(&id, "one");
    conflicting.lines[0].text = "Changed original".into();
    assert!(
        db.save_versions(id.clone(), vec![version(&id, "two"), conflicting])
            .is_err()
    );
    assert_eq!(db.versions(id.clone()).unwrap().len(), 1);
    assert_eq!(
        db.selected(id.clone()).unwrap().unwrap().lines[0].text,
        "An original sample line"
    );
    let mut other = song();
    other.title = "Different song".into();
    let other = db.resolve(other, None).unwrap();
    assert!(db.select(other.id, "one".into()).is_err());
    assert!(db.adjust("one".into(), 600001, false).is_err());
}
#[test]
fn concurrent_resolution_rechecks_identity_inside_the_serial_worker() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let db = db.clone();
            std::thread::spawn(move || db.resolve(song(), None).unwrap().id)
        })
        .collect();
    let ids: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_eq!(db.list(0, 100).unwrap().len(), 1);
}
#[test]
fn ambiguous_album_identity_does_not_merge_distinct_recordings() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let first = db.resolve(song(), None).unwrap();
    let mut other = song();
    other.album = Some("Different album".into());
    let second = db.resolve(other, None).unwrap();
    let mut partial = song();
    partial.album = None;
    let third = db.resolve(partial, None).unwrap();
    assert_ne!(first.id, second.id);
    assert_ne!(third.id, first.id);
    assert_ne!(third.id, second.id);
    assert_eq!(db.resolve(song(), None).unwrap().id, first.id);
    assert_eq!(db.list(0, 100).unwrap().len(), 3);
}
#[test]
fn explicit_editions_survive_previous_identity_and_filename_hints() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let mut studio = song();
    studio.version_label = Some("Studio".into());
    let first = db.resolve(studio, None).unwrap();
    let mut live = song();
    live.version_label = Some("Live".into());
    let second = db.resolve(live.clone(), Some(first.id.clone())).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(db.resolve(live, None).unwrap().id, second.id);
    let mut titled_live = song();
    titled_live.title = "Synthetic song (Live)".into();
    titled_live.file_name = Some("Fixture artist - Synthetic song.mp3".into());
    let third = db.resolve(titled_live, None).unwrap();
    assert_ne!(third.id, first.id);
    assert_ne!(third.id, second.id);
}
#[test]
fn tagged_copy_matches_filename_recording_without_replacing_original_title() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let mut filename = song();
    filename.title = "01 - Fixture artist - Synthetic song.mp3".into();
    filename.artists.clear();
    filename.album = None;
    let first = db.resolve(filename, None).unwrap();
    let tagged = db.resolve(song(), None).unwrap();
    assert_eq!(first.id, tagged.id);
    assert_eq!(tagged.title, "01 - Fixture artist - Synthetic song.mp3");
    assert_eq!(tagged.artists, vec!["Fixture artist"]);
}

#[test]
fn generic_filenames_without_artist_evidence_do_not_merge_across_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).unwrap();
    let mut incoming = song();
    incoming.title = "Track 01.mp3".into();
    incoming.artists.clear();
    incoming.album = None;
    let first = db.resolve(incoming.clone(), None).unwrap();
    let second = db.resolve(incoming.clone(), None).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(
        db.resolve(incoming, Some(first.id.clone())).unwrap().id,
        first.id
    );
}
#[test]
fn unsupported_schema_and_failed_migration_never_reset_existing_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE sentinel(value TEXT); INSERT INTO sentinel VALUES('preserve'); PRAGMA user_version=99;").unwrap();
    }
    assert!(Storage::open(path.clone()).is_err());
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row("SELECT value FROM sentinel", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "preserve"
    );
    let path = dir.path().join("conflict");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE recordings(legacy TEXT); INSERT INTO recordings VALUES('keep');",
        )
        .unwrap();
    }
    assert!(Storage::open(path.clone()).is_err());
    let conn = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        conn.query_row("SELECT legacy FROM recordings", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}
