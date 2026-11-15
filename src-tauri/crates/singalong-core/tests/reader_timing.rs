use singalong_core::{
    lyrics::{estimated, word_sync},
    model::{Counter, LyricLine, LyricWord},
};

fn line(text: &str, start: Option<u64>) -> LyricLine {
    LyricLine {
        id: text.into(),
        text: text.into(),
        start_ms: start.map(Counter::new),
        end_ms: None,
        words: vec![],
    }
}

#[test]
fn estimated_timing_weights_utf16_skips_blanks_and_never_changes_originals() {
    let lines = vec![
        line("a", Some(777)),
        line(" \t", None),
        line("😀", None),
        line("b", None),
    ];
    let original = serde_json::to_value(&lines).unwrap();
    assert_eq!(
        estimated::starts(&lines, 4000),
        vec![Some(0), None, Some(1000), Some(3000)]
    );
    assert_eq!(estimated::starts(&lines, 0), vec![None; 4]);
    assert_eq!(estimated::index(&lines, None, 4000, 0), None);
    assert_eq!(estimated::index(&lines, Some(999), 4000, 0), Some(0));
    assert_eq!(estimated::index(&lines, Some(1000), 4000, 0), Some(2));
    assert_eq!(estimated::index(&lines, Some(1000), 4000, 1), Some(0));
    assert_eq!(estimated::index(&lines, Some(0), 4000, 1), None);
    assert_eq!(estimated::index(&lines, Some(0), 4000, -1000), Some(2));
    assert_eq!(serde_json::to_value(&lines).unwrap(), original);
}

#[test]
fn word_emphasis_infers_ends_but_preserves_supplied_overlaps() {
    let mut first = line("a b c", Some(100));
    first.words = vec![
        LyricWord {
            text: "a ".into(),
            start_ms: Counter::new(100),
            end_ms: Some(Counter::new(500)),
        },
        LyricWord {
            text: "b ".into(),
            start_ms: Counter::new(200),
            end_ms: None,
        },
        LyricWord {
            text: "c".into(),
            start_ms: Counter::new(400),
            end_ms: None,
        },
    ];
    let lines = vec![first, line("next", Some(800))];
    let original = serde_json::to_value(&lines).unwrap();
    assert_eq!(
        word_sync::emphasis(&lines, 0, Some(300), None, 0),
        vec![0.5, 0.5, 0.0]
    );
    assert_eq!(
        word_sync::emphasis(&lines, 0, Some(600), None, 0),
        vec![1.0, 1.0, 0.5]
    );
    assert_eq!(
        word_sync::emphasis(&lines, 0, Some(600), None, 300),
        vec![0.5, 0.5, 0.0]
    );
    assert_eq!(word_sync::emphasis(&lines, 0, None, None, 0), vec![0.0; 3]);
    assert!(word_sync::emphasis(&lines, 10, Some(0), None, 0).is_empty());
    assert_eq!(serde_json::to_value(&lines).unwrap(), original);
}

#[test]
fn repeated_word_starts_share_next_boundary_and_duration_clamps_progress() {
    let mut last = line("ab", Some(100));
    last.words = vec![
        LyricWord {
            text: "a".into(),
            start_ms: Counter::new(100),
            end_ms: None,
        },
        LyricWord {
            text: "b".into(),
            start_ms: Counter::new(100),
            end_ms: None,
        },
    ];
    assert_eq!(
        word_sync::emphasis(&[last.clone()], 0, Some(200), Some(300), 0),
        vec![0.5; 2]
    );
    assert_eq!(
        word_sync::emphasis(&[last.clone()], 0, Some(u64::MAX), Some(300), i64::MIN),
        vec![1.0; 2]
    );
    assert_eq!(
        word_sync::emphasis(&[last], 0, Some(0), Some(300), i64::MAX),
        vec![0.0; 2]
    );
}
