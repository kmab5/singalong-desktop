//! Comparison keys and conservative filename hints never replace source strings.
use crate::model::Recording;
use unicode_normalization::UnicodeNormalization;

pub fn normalize(value: &str) -> String {
    value
        .nfkc()
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn artists(value: &Recording) -> Vec<String> {
    value
        .artists
        .iter()
        .map(|artist| normalize(artist))
        .collect()
}
fn album(value: &Recording) -> String {
    normalize(value.album.as_deref().unwrap_or_default())
}
fn metadata_compatible(a: &Recording, b: &Recording) -> bool {
    (album(a).is_empty() || album(b).is_empty() || album(a) == album(b))
        && (version(a).is_empty() || version(b).is_empty() || version(a) == version(b))
        && match (&a.duration_ms, &b.duration_ms) {
            (Some(a), Some(b)) => a.value().abs_diff(b.value()) <= 2000,
            _ => true,
        }
}
fn version(value: &Recording) -> String {
    normalize(value.version_label.as_deref().unwrap_or_default())
}
pub fn exact(a: &Recording, b: &Recording) -> bool {
    normalize(&a.title) == normalize(&b.title)
        && artists(a) == artists(b)
        && album(a) == album(b)
        && version(a) == version(b)
        && a.duration_ms == b.duration_ms
}
pub fn compatible(a: &Recording, b: &Recording) -> bool {
    normalize(&a.title) == normalize(&b.title)
        && artists(a) == artists(b)
        && metadata_compatible(a, b)
}
pub fn same_observation(a: &Recording, b: &Recording) -> bool {
    normalize(&a.title) == normalize(&b.title)
        && (a.artists.is_empty() || b.artists.is_empty() || artists(a) == artists(b))
        && metadata_compatible(a, b)
}
pub fn enrich_missing(existing: &Recording, incoming: &Recording) -> Recording {
    let mut value = existing.clone();
    if value.artists.is_empty() {
        value.artists = incoming.artists.clone();
    }
    if album(&value).is_empty() {
        value.album = incoming.album.clone();
    }
    if value.duration_ms.is_none() {
        value.duration_ms = incoming.duration_ms.clone();
    }
    if value.version_label.is_none() {
        value.version_label = incoming.version_label.clone();
    }
    if value.file_name.is_none() {
        value.file_name = incoming.file_name.clone();
    }
    value
}

fn forms(value: &Recording) -> Vec<Recording> {
    let mut result = vec![value.clone()];
    for name in [Some(value.title.as_str()), value.file_name.as_deref()]
        .into_iter()
        .flatten()
    {
        let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let Some((stem, extension)) = name.rsplit_once('.') else {
            continue;
        };
        if ![
            "mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "wma", "aiff", "alac", "mp4", "webm",
        ]
        .contains(&extension.to_lowercase().as_str())
        {
            continue;
        }
        let mut parts: Vec<_> = stem.split(" - ").collect();
        if parts.first().is_some_and(|part| {
            !part.is_empty() && part.len() <= 3 && part.chars().all(|c| c.is_ascii_digit())
        }) {
            parts.remove(0);
        }
        if parts.len() == 2
            && (normalize(&value.title) == normalize(name)
                || normalize(&value.title) == normalize(parts[1]))
            && (value.artists.is_empty()
                || normalize(&value.artists.join(", ")) == normalize(parts[0]))
        {
            let mut hint = value.clone();
            hint.title = parts[1].to_string();
            if hint.artists.is_empty() {
                hint.artists = vec![parts[0].to_string()];
            }
            result.push(hint);
        }
    }
    result
}
pub fn folder_same(a: &Recording, b: &Recording) -> bool {
    forms(a).iter().any(|left| {
        !left.artists.is_empty() && forms(b).iter().any(|right| compatible(left, right))
    })
}
pub fn folder_match<'a>(existing: &'a [Recording], incoming: &Recording) -> Option<&'a Recording> {
    let matches: Vec<_> = existing
        .iter()
        .filter(|value| folder_same(value, incoming))
        .collect();
    if matches.len() == 1 {
        return Some(matches[0]);
    }
    let exact: Vec<_> = matches
        .into_iter()
        .filter(|value| exact(value, incoming))
        .collect();
    if exact.len() == 1 {
        Some(exact[0])
    } else {
        None
    }
}
