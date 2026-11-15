use singalong_core::backup::*;
use std::{cell::Cell, io::Cursor};

fn fixture() -> BackupData {
    serde_json::from_value(serde_json::json!({
        "songs": [{
            "recording": {"id":"song", "title":"Synthetic sunrise", "artists":["Fixture"], "album":null, "durationMs":4000},
            "plays":9007199254740993u64, "lastPlayed":1234, "favorite":true,
            "versions":[{"id":"verse", "recordingId":"song", "contentHash":"opaque-original-hash", "origin":"PERSONAL",
                "lines":[{"id":"line", "text":"Synthetic light", "startMs":100, "endMs":3000,
                    "words":[{"text":"Synthetic ", "startMs":100, "endMs":800}, {"text":"light", "startMs":900}]}],
                "sourceName":null, "sourceUrl":null, "confidence":"CONFIDENT"}],
            "selected":"verse", "adjustments":[{"versionId":"verse", "delayMs":-250, "allowUncertainSync":true}],
            "translations":[{"versionId":"verse", "contentHash":"opaque-original-hash", "provider":"fixture", "sourceLanguage":"en", "lines":{"line":"Synthetic translation"}, "attribution":"fixture"}]
        }], "options":{}, "settings":{}, "androidPreferences":{},
        "inertPreferences":{"fixture":{"type":"long","value":9007199254740993i64}}
    })).unwrap()
}

#[test]
fn desktop_roundtrip_is_lossless_and_deterministic() {
    let data = fixture();
    let bytes = encode_bytes(&data, BackupBudget::default()).unwrap();
    let decoded = decode_bytes(&bytes, BackupBudget::default()).unwrap();
    assert_eq!(decoded.source, BackupSource::DesktopV1);
    assert_eq!(decoded.data, data);
    assert_eq!(
        (
            decoded.summary.recordings,
            decoded.summary.versions,
            decoded.summary.translations
        ),
        (1, 1, 1)
    );
    assert_eq!(
        encode_bytes(&decoded.data, BackupBudget::default()).unwrap(),
        bytes
    );
    let mut prefixed = Cursor::new([b"ignored".as_slice(), bytes.as_slice()].concat());
    prefixed.set_position(7);
    assert_eq!(
        decode_reader(prefixed, BackupBudget::default())
            .unwrap()
            .data,
        data
    );
}

#[test]
fn tampering_and_future_schema_are_rejected() {
    let bytes = encode_bytes(&fixture(), BackupBudget::default()).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mut modified = original.clone();
    modified["data"]["songs"][0]["recording"]["title"] = "Changed title".into();
    assert!(matches!(
        decode_bytes(
            &serde_json::to_vec(&modified).unwrap(),
            BackupBudget::default()
        ),
        Err(BackupError::Invalid("desktop backup hash mismatch"))
    ));
    let mut future = original;
    future["schema"] = 2.into();
    assert!(matches!(
        decode_bytes(
            &serde_json::to_vec(&future).unwrap(),
            BackupBudget::default()
        ),
        Err(BackupError::Invalid("unsupported desktop backup version"))
    ));
}

#[test]
fn exact_byte_budget_accepts_archive_and_one_byte_less_rejects_it() {
    let data = fixture();
    let bytes = encode_bytes(&data, BackupBudget::unlimited()).unwrap();
    let exact = BackupBudget {
        max_bytes: Some(bytes.len() as u64),
    };
    assert_eq!(encode_bytes(&data, exact).unwrap(), bytes);
    assert_eq!(decode_bytes(&bytes, exact).unwrap().data, data);
    let short = BackupBudget {
        max_bytes: Some(bytes.len() as u64 - 1),
    };
    assert!(matches!(
        encode_bytes(&data, short),
        Err(BackupError::BudgetExceeded)
    ));
    assert!(matches!(
        decode_bytes(&bytes, short),
        Err(BackupError::BudgetExceeded)
    ));
}

#[test]
fn parser_limits_apply_even_with_unlimited_bytes_before_writing() {
    let data = fixture();
    let bytes = encode_bytes(&data, BackupBudget::unlimited()).unwrap();
    let control = BackupControl {
        parse_limits: ParseLimits {
            max_nodes: 1,
            max_string_bytes: 1,
        },
        ..Default::default()
    };
    let mut output = Vec::new();
    assert!(matches!(
        encode_writer_with_control(&mut output, &data, BackupBudget::unlimited(), &control),
        Err(BackupError::ParseLimitExceeded)
    ));
    assert!(output.is_empty());
    assert!(matches!(
        decode_bytes_with_control(&bytes, BackupBudget::unlimited(), &control),
        Err(BackupError::ParseLimitExceeded)
    ));
}

#[test]
fn cancellation_is_latched_after_work_has_started() {
    let data = fixture();
    let bytes = encode_bytes(&data, BackupBudget::default()).unwrap();
    for encode in [false, true] {
        let calls = Cell::new(0);
        // A one-shot cancellation must not turn into a JSON or I/O error later.
        let cancel = || {
            calls.set(calls.get() + 1);
            calls.get() == 10
        };
        let control = BackupControl {
            is_cancelled: &cancel,
            ..Default::default()
        };
        let result = if encode {
            encode_bytes_with_control(&data, BackupBudget::default(), &control).map(|_| ())
        } else {
            decode_bytes_with_control(&bytes, BackupBudget::default(), &control).map(|_| ())
        };
        assert!(matches!(result, Err(BackupError::Cancelled)));
        assert!(calls.get() >= 10);
    }
}

#[test]
fn invalid_selection_and_partial_desktop_export_are_rejected() {
    let mut data = fixture();
    data.songs[0].selected = Some("missing".into());
    assert!(validate(&data).is_err());
    data.songs[0].selected = None;
    validate(&data).unwrap();
    data.options.history = false;
    data.songs[0].plays = 0;
    data.songs[0].last_played = None;
    validate(&data).unwrap();
    assert!(matches!(
        encode_bytes(&data, BackupBudget::default()),
        Err(BackupError::Invalid(
            "desktop export requires full snapshot"
        ))
    ));
}
