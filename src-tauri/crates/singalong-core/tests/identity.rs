use singalong_core::{
    identity::*,
    model::{Counter, Recording},
};

fn song() -> Recording {
    Recording {
        id: "studio".into(),
        title: "A Song".into(),
        artists: vec!["Artist".into()],
        album: Some("Album".into()),
        duration_ms: Some(Counter::new(180_000)),
        version_label: None,
        file_name: None,
    }
}
#[test]
fn comparison_normalizes_without_changing_display_strings_or_editions() {
    let studio = song();
    let mut changed = studio.clone();
    changed.title = "  Ａ   SONG ".into();
    changed.artists = vec!["ARTIST".into()];
    assert!(compatible(&studio, &changed));
    assert_eq!(studio.title, "A Song");
    changed.title = "A Song (Live)".into();
    assert!(!compatible(&studio, &changed));
    changed = studio.clone();
    changed.duration_ms = Some(Counter::new(205000));
    assert!(!compatible(&studio, &changed));
    changed = studio.clone();
    changed.album = None;
    assert!(compatible(&studio, &changed));
    changed.artists = vec!["Other".into()];
    assert!(!compatible(&studio, &changed));
}
#[test]
fn missing_artist_enrichment_is_only_allowed_with_session_continuity() {
    let full = song();
    let mut partial = full.clone();
    partial.artists.clear();
    assert!(!compatible(&partial, &full));
    assert!(same_observation(&partial, &full));
    assert_eq!(enrich_missing(&partial, &full).artists, vec!["Artist"]);
}
#[test]
fn filename_hints_do_not_rewrite_metadata_or_arbitrarily_pick_an_album() {
    let tagged = song();
    let mut file = tagged.clone();
    file.title = "01 - Artist - A Song.mp3".into();
    file.artists.clear();
    file.album = None;
    assert_eq!(
        folder_match(std::slice::from_ref(&tagged), &file)
            .unwrap()
            .id,
        "studio"
    );
    assert_eq!(file.title, "01 - Artist - A Song.mp3");
    assert!(file.artists.is_empty());
    let mut partial = tagged.clone();
    partial.artists.clear();
    partial.file_name = Some("Artist - A Song.flac".into());
    assert!(folder_same(&partial, &tagged));
    let mut other = tagged.clone();
    other.id = "other".into();
    other.album = Some("Other album".into());
    assert!(folder_match(&[tagged.clone(), other.clone()], &file).is_none());
    assert_eq!(
        folder_match(&[other, tagged.clone()], &tagged).unwrap().id,
        "studio"
    );
    let mut live = tagged.clone();
    live.title += " (Live)";
    assert!(!folder_same(&tagged, &live));
}
