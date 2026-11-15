use singalong_core::backup as b;
use singalong_desktop::{platform::Cancellation, storage::Storage};
use std::sync::{Arc, atomic::AtomicU64};

fn token() -> Cancellation {
    Cancellation {
        epoch: Arc::new(AtomicU64::new(0)),
        expected: 0,
    }
}
fn fixture() -> b::BackupData {
    serde_json::from_value(serde_json::json!({
        "songs":[{"recording":{"id":"song", "title":"Synthetic sunrise", "artists":["Fixture"], "album":null, "durationMs":4000},
            "plays":7, "lastPlayed":1234, "favorite":true,
            "versions":[{"id":"verse", "recordingId":"song", "contentHash":"opaque-hash", "origin":"PROVIDER",
                "lines":[{"id":"line", "text":"Synthetic light", "startMs":100, "words":[{"text":"Synthetic light", "startMs":100,"endMs":500}]}],
                "sourceName":"Fixture provider", "sourceUrl":null, "confidence":"CONFIDENT"}],
            "selected":"verse", "adjustments":[{"versionId":"verse", "delayMs":-250, "allowUncertainSync":true}],
            "translations":[{"versionId":"verse", "contentHash":"opaque-hash", "provider":"fixture", "sourceLanguage":"en", "lines":{"line":"Synthetic translation"}, "attribution":"fixture"}]}],
        "options":{}, "settings":{"textSize":{"type":"int","value":36},"trackingEnabled":{"type":"boolean","value":true},"translationConsent":{"type":"boolean","value":true}},
        "androidPreferences":{}, "inertPreferences":{"fixture":{"type":"string","value":"retained"}}
    })).unwrap()
}
fn snapshot(db: &Storage) -> b::BackupData {
    db.backup_snapshot(token()).unwrap()
}
fn merge(
    db: &Storage,
    data: b::BackupData,
    preview: bool,
) -> singalong_desktop::storage::MergeReport {
    db.backup_merge(Arc::new(data), true, preview, token())
        .unwrap()
}

#[test]
fn preview_rolls_back_every_section_and_predicts_committed_merge() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("preview.sqlite")).unwrap();
    let before = snapshot(&db);
    let preview = merge(&db, fixture(), true);
    assert_eq!(preview.recordings_added, 1);
    assert_eq!(preview.versions_added, 1);
    assert_eq!(preview.translations_added, 1);
    assert_eq!(snapshot(&db), before);
    let committed = merge(&db, fixture(), false);
    assert_eq!(preview.summary(), committed.summary());
    assert_eq!(db.settings().unwrap().text_size, 36);
    assert!(!db.settings().unwrap().tracking_enabled);
    assert!(!db.settings().unwrap().translation_consent);
}

#[test]
fn merge_is_idempotent_and_export_survives_codec_and_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("merge.sqlite");
    let db = Storage::open(path.clone()).unwrap();
    merge(&db, fixture(), false);
    let first = snapshot(&db);
    let repeat = merge(&db, fixture(), false);
    assert_eq!(
        (
            repeat.recordings_added,
            repeat.versions_added,
            repeat.translations_added
        ),
        (0, 0, 0)
    );
    assert_eq!(
        (
            repeat.recordings_matched,
            repeat.versions_skipped,
            repeat.translations_skipped
        ),
        (1, 1, 1)
    );
    assert_eq!(snapshot(&db), first);
    assert_eq!(first.songs[0].plays, 7);
    assert_eq!(first.songs[0].versions, fixture().songs[0].versions);
    assert_eq!(first.songs[0].translations, fixture().songs[0].translations);
    let bytes = b::encode_bytes(&first, b::BackupBudget::default()).unwrap();
    assert_eq!(
        b::decode_bytes(&bytes, b::BackupBudget::default())
            .unwrap()
            .data,
        first
    );
    drop(db);
    let reopened = Storage::open(path).unwrap();
    assert_eq!(snapshot(&reopened), first);
}

#[test]
fn cancelled_merge_and_snapshot_leave_existing_database_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("cancel.sqlite")).unwrap();
    merge(&db, fixture(), false);
    let before = snapshot(&db);
    let mut incoming = fixture();
    incoming.songs[0].recording.id = "new".into();
    incoming.songs[0].recording.title = "Another synthetic song".into();
    incoming.songs[0].versions[0].recording_id = "new".into();
    let cancelled = Cancellation {
        expected: 1,
        ..token()
    };
    let error = db
        .backup_merge(Arc::new(incoming), true, false, cancelled.clone())
        .unwrap_err();
    assert_eq!(error.to_string(), "backup cancelled");
    assert_eq!(
        db.backup_snapshot(cancelled).unwrap_err().to_string(),
        "backup cancelled"
    );
    assert_eq!(snapshot(&db), before);
}

#[test]
fn null_selection_stays_null_and_imported_provider_copy_survives_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("cleanup.sqlite")).unwrap();
    let mut data = fixture();
    data.songs[0].selected = None;
    data.songs[0].favorite = false;
    merge(&db, data.clone(), false);
    // Reader fallback must not manufacture a persisted explicit selection.
    assert_eq!(db.selected("song".into()).unwrap().unwrap().id, "verse");
    let before = snapshot(&db);
    assert_eq!(before.songs[0].selected, None);
    let mut disposable = db.version("verse".into()).unwrap();
    disposable.id = "ordinary-download".into();
    disposable.content_hash = "ordinary-download-hash".into();
    db.save_versions("song".into(), vec![disposable]).unwrap();
    db.clear_downloads().unwrap();
    assert_eq!(snapshot(&db), before);
    assert_eq!(db.versions("song".into()).unwrap().len(), 1);
    db.select("song".into(), "verse".into()).unwrap();
    merge(&db, data, false);
    assert_eq!(db.selected("song".into()).unwrap().unwrap().id, "verse");
}

#[test]
fn failure_after_recording_insertion_rolls_back_entire_merge() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rollback.sqlite");
    let db = Storage::open(path.clone()).unwrap();
    let before = snapshot(&db);
    // Only this disposable test database is opened directly. Force a deterministic
    // mid-transaction failure after recording resolution has already inserted rows.
    let connection = rusqlite::Connection::open(path).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_fixture BEFORE INSERT ON translations BEGIN SELECT RAISE(ABORT, 'synthetic write failure'); END;").unwrap();
    let error = db
        .backup_merge(Arc::new(fixture()), true, false, token())
        .unwrap_err();
    assert!(matches!(
        error,
        singalong_desktop::storage::StorageError::Sql(_)
    ));
    assert_eq!(snapshot(&db), before);
    connection
        .execute_batch("DROP TRIGGER reject_fixture;")
        .unwrap();
    assert_eq!(merge(&db, fixture(), false).recordings_added, 1);
}

#[test]
fn conflicting_immutable_content_is_preserved_and_repeat_import_does_not_duplicate() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("conflict.sqlite")).unwrap();
    merge(&db, fixture(), false);
    let mut conflicting = fixture();
    conflicting.songs[0].versions[0].lines[0].text = "Different synthetic verse".into();
    conflicting.songs[0].versions[0].lines[0].words[0].text = "Different synthetic verse".into();
    let report = merge(&db, conflicting.clone(), false);
    assert_eq!(report.versions_added, 1);
    assert!(report.conflicts > 0);
    assert_eq!(
        db.selected("song".into()).unwrap().unwrap().lines[0].text,
        "Synthetic light"
    );
    let before = snapshot(&db);
    assert_eq!(before.songs[0].versions.len(), 2);
    let repeated = merge(&db, conflicting, false);
    assert_eq!(repeated.versions_added, 0);
    assert_eq!(snapshot(&db), before);
}
