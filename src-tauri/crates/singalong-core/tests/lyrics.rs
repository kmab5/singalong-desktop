use singalong_core::{
    lyrics::{lrc, lyricsfile, matching, queries, timeline},
    model::*,
};
fn recording(title: &str, artist: &str) -> Recording {
    Recording {
        id: "recording".into(),
        title: title.into(),
        artists: vec![artist.into()],
        album: Some("Fixture album".into()),
        duration_ms: Some(Counter::new(120000)),
        version_label: None,
        file_name: None,
    }
}
fn song() -> matching::ProviderSong {
    serde_json::from_str(r#"{"id":1,"trackName":"Fixture song","artistName":"Fixture artist","albumName":"Fixture album","duration":120,"syncedLyrics":"[00:01]An original line\n[00:03]Another original line"}"#).unwrap()
}
#[test]
fn lrc_preserves_repeated_tags_offsets_unicode_and_blank_intervals() {
    let rows = lrc::parse(
        Some("[offset:-500]\n[00:01.00][00:03.00]世界\n[00:04]\n[00:08]Next original line"),
        None,
    );
    assert_eq!(
        rows.iter()
            .map(|r| r.start_ms.as_ref().unwrap().value())
            .collect::<Vec<_>>(),
        vec![500, 2500, 3500, 7500]
    );
    assert_eq!(rows[1].text, "世界");
    assert_eq!(
        timeline::at(&rows, Some(3500), 0).countdown_seconds,
        Some(4)
    );
    assert_eq!(timeline::at(&rows, Some(3000), 0).countdown_seconds, None);
    assert_eq!(timeline::at(&rows, Some(2600), 250).index, Some(0));
    assert_eq!(timeline::at(&rows, None, 0).index, None);
}
#[test]
fn malformed_lrc_falls_back_and_blank_only_lyrics_are_empty() {
    assert_eq!(
        lrc::parse(Some("[00:99]bad"), Some("Original\n世界"))[1].text,
        "世界"
    );
    assert!(lrc::parse(Some("[00:01]  "), Some(" \n ")).is_empty());
    let lines = lrc::parse(Some("[00:01]one\n[00:01]two"), None);
    assert_eq!(timeline::at(&lines, Some(1000), 0).index, Some(1));
    assert!(lrc::parse(Some(&"x".repeat(2_000_001)), None).is_empty());
}
const YAML: &str = "version: '1.0'\nmetadata: {title: Fixture, artist: Fixture}\nlines:\n  - text: 'Hello world'\n    start_ms: 1000\n    end_ms: 6000\n    words:\n      - {text: 'Hello ', start_ms: 1000, end_ms: 1500}\n      - {text: world, start_ms: 4000, end_ms: 6000}\n";
#[test]
fn lyricsfile_keeps_valid_words_but_discards_malformed_word_timing() {
    let rows = lyricsfile::parse(YAML).unwrap();
    assert_eq!(rows[0].words.len(), 2);
    assert_eq!(rows[0].words[0].text, "Hello ");
    for bad in [
        YAML.replace("text: world", "text: wrong"),
        YAML.replace("end_ms: 1500", "end_ms: 500"),
        YAML.replace("start_ms: 4000", "start_ms: 4.5"),
    ] {
        let rows = lyricsfile::parse(&bad).unwrap();
        assert_eq!(rows[0].text, "Hello world");
        assert!(rows[0].words.is_empty());
    }
}
#[test]
fn lyricsfile_rejects_unsafe_unsupported_and_ambiguous_documents() {
    for bad in [
        YAML.replace("'1.0'", "'2.0'"),
        YAML.replace("version: '1.0'", "version: '1.0'\nversion: '1.0'"),
        YAML.replace("artist: Fixture", "artist: Fixture, offset_ms: 100"),
        "a: &a [x]\nb: *a".into(),
        "!!java.net.URL ['https://example.invalid']".into(),
        format!("{}x{}", "[".repeat(100), "]".repeat(100)),
        "x".repeat(2_000_001),
    ] {
        assert!(lyricsfile::parse(&bad).is_none());
    }
}
#[test]
fn queries_use_bounded_hints_without_removing_editions_or_rewriting_metadata() {
    let original = recording(
        "03 - Fixture_artist - Fixture_song (Live Remix) [Official Video].mp3",
        "<unknown>",
    );
    let hints = queries::plan(&original);
    assert!(hints.len() <= 3);
    assert_eq!(hints[0].recording.title, "Fixture song (Live Remix)");
    assert_eq!(hints[0].recording.artists, vec!["Fixture artist"]);
    assert!(hints[0].inferred);
    assert_eq!(original.artists, vec!["<unknown>"]);
    assert!(queries::plan(&recording("Artist - Album - Title.mp3", "<unknown>")).is_empty());
    assert_eq!(
        queries::plan(&recording("99 Problems.mp3", "Artist"))[0]
            .recording
            .title,
        "99 Problems"
    );
    let clean = recording("Fixture song (Live)", "Fixture artist");
    assert!(!queries::plan(&clean)[0].inferred);
}
#[test]
fn matching_preserves_editions_rejects_wrong_artist_and_ranks_timed_content() {
    let input = recording("Fixture song", "Fixture artist");
    let candidate = song();
    assert!(matches!(
        matching::confidence(&input, &candidate),
        Some(MatchConfidence::Confident)
    ));
    let versions = matching::versions(&input, vec![candidate.clone()], false);
    assert_eq!(versions.len(), 1);
    assert!(matches!(versions[0].timing, TimingKind::Supplied));
    let inferred = matching::versions(&input, vec![candidate.clone()], true);
    assert!(matches!(inferred[0].confidence, MatchConfidence::Uncertain));
    for bad in [
        matching::ProviderSong {
            track_name: "Fixture song (Live)".into(),
            ..candidate.clone()
        },
        matching::ProviderSong {
            artist_name: "Other".into(),
            ..candidate.clone()
        },
        matching::ProviderSong {
            duration: 250.0,
            ..candidate.clone()
        },
    ] {
        assert!(matching::confidence(&input, &bad).is_none());
    }
    let blank = matching::ProviderSong {
        synced_lyrics: Some("[00:01] ".into()),
        plain_lyrics: Some(" ".into()),
        ..candidate
    };
    assert!(matching::versions(&input, vec![blank], false).is_empty());
}

#[test]
fn candidate_preference_is_deterministic_and_does_not_relax_matching() {
    let input = recording("Fixture song", "Fixture artist");
    let mut candidates = vec![];
    for (id, duration, timed) in [
        (1, 120.0, false),
        (2, 126.0, true),
        (3, 120.4, true),
        (4, 0.0, true),
    ] {
        let mut value = song();
        value.id = id;
        value.duration = duration;
        if !timed {
            value.synced_lyrics = None;
            value.plain_lyrics = Some("Original plain fixture".into());
        }
        candidates.push(value);
    }
    let ids = |items| {
        matching::versions(&input, items, false)
            .into_iter()
            .map(|v| v.provider_id.unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(candidates.clone()), ["3", "2", "4", "1"]);
    candidates.reverse();
    assert_eq!(ids(candidates), ["3", "2", "4", "1"]);
}

#[test]
fn hints_cannot_remove_tagged_editions_and_duplicate_responses_keep_identity() {
    let mut input = recording("Fixture song (Live)", "Fixture artist");
    input.file_name = Some("Fixture artist - Fixture song.mp3".into());
    assert!(
        queries::plan(&input)
            .iter()
            .all(|q| q.recording.title.contains("Live"))
    );
    let input = recording("Fixture song", "Fixture artist");
    let first = matching::versions(&input, vec![song(), song()], false);
    assert_eq!(first.len(), 1);
    assert_eq!(
        first[0].id,
        matching::versions(&input, vec![song()], false)[0].id
    );
    let mut changed = song();
    changed.synced_lyrics = Some("[00:01]Different original fixture".into());
    assert_ne!(
        first[0].content_hash,
        matching::versions(&input, vec![changed], false)[0].content_hash
    );
}

#[test]
fn invalid_draft_falls_back_and_word_timing_changes_content_identity() {
    let input = recording("Fixture song", "Fixture artist");
    let mut candidate = song();
    candidate.lyricsfile = Some("unsupported".into());
    assert_eq!(
        matching::versions(&input, vec![candidate.clone()], false)[0].lines[0].text,
        "An original line"
    );
    candidate.lyricsfile = Some(YAML.into());
    let first = matching::versions(&input, vec![candidate.clone()], false);
    candidate.lyricsfile = Some(YAML.replace("start_ms: 4000", "start_ms: 4100"));
    assert_ne!(
        first[0].content_hash,
        matching::versions(&input, vec![candidate], false)[0].content_hash
    );
    for bad in [
        YAML.replace("version: '1.0'", "version: 1.0"),
        format!("{YAML}\n---\nx: y"),
        YAML.replace("start_ms: 1000", "start_ms: 9223372036854775808"),
    ] {
        assert!(lyricsfile::parse(&bad).is_none());
    }
}

#[test]
fn repeated_tags_cannot_amplify_provider_text_past_the_parse_budget() {
    let payload = format!("{}{}", "[00:01]".repeat(100), "x".repeat(30_000));
    assert!(lrc::parse(Some(&payload), None).is_empty());
}
#[test]
fn filename_hints_preserve_complete_editions_and_can_fill_only_missing_artist() {
    for (title, file) in [
        (
            "Fixture song (2011 Remastered)",
            "Artist - Fixture song.mp3",
        ),
        (
            "Fixture song (Live at Wembley)",
            "Artist - Fixture song (Live at Rome).mp3",
        ),
    ] {
        let mut input = recording(title, "<unknown>");
        input.file_name = Some(file.into());
        assert!(queries::plan(&input).is_empty());
    }
    let mut input = recording("Fixture song (Live at Wembley)", "<unknown>");
    input.file_name = Some("Artist - Fixture song (Live at Wembley).mp3".into());
    assert_eq!(queries::plan(&input)[0].recording.artists, vec!["Artist"]);
}
#[test]
fn separate_version_labels_require_the_complete_matching_edition() {
    for label in ["Live", "2011 Remastered", "Live at Wembley"] {
        let mut input = recording("Fixture song", "Fixture artist");
        input.version_label = Some(label.into());
        let mut candidate = song();
        candidate.track_name = format!("Fixture song ({label})");
        assert!(matching::confidence(&input, &candidate).is_some());
        assert!(matching::confidence(&input, &song()).is_none());
        candidate.track_name = "Fixture song (Live at Rome)".into();
        assert!(matching::confidence(&input, &candidate).is_none());
    }
}
