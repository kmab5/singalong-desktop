use crate::{identity::normalize, model::Recording};
use regex::Regex;
use std::sync::LazyLock;

static EXT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\.(mp3|flac|m4a|aac|ogg|opus|wav|wma|aiff|alac|mp4|webm)$").unwrap()
});
static PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[0-9]{1,3}\s*[-.]\s*|0[0-9]{1,2}\s+)").unwrap());
static SEP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+[-–—|]\s+").unwrap());
static DECORATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[\[(](?:official(?: music)? (?:video|audio)|official|lyrics?|lyric video|audio|video|hd|hq|4k)[\])]").unwrap()
});

#[derive(Debug, Clone)]
pub struct Query {
    pub recording: Recording,
    pub inferred: bool,
}
fn known(s: &str) -> bool {
    !matches!(
        normalize(s).as_str(),
        "" | "unknown" | "<unknown>" | "[unknown]" | "unknown artist" | "unknown album"
    )
}
fn spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn plan(original: &Recording) -> Vec<Query> {
    if original.title.len() > 4096
        || original.artists.len() > 64
        || original.artists.iter().any(|a| a.len() > 4096)
        || original.file_name.as_ref().is_some_and(|s| s.len() > 4096)
    {
        return vec![];
    }
    let raw = if EXT.is_match(original.title.trim()) {
        original
            .title
            .trim()
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
    } else {
        &original.title
    };
    let clean = spaces(&DECORATION.replace_all(&EXT.replace(raw, "").replace('_', " "), ""));
    let stripped = PREFIX.replace(&clean, "");
    let title = if stripped.is_empty() {
        clean.as_str()
    } else {
        stripped.as_ref()
    };
    let artists: Vec<_> = original
        .artists
        .iter()
        .filter(|s| known(s))
        .map(|s| spaces(&s.replace('_', " ")))
        .collect();
    let artist = artists.join(", ");
    let parts: Vec<_> = SEP.split(title).collect();
    let mut derived = original.clone();
    derived.title = title.into();
    derived.artists = artists;
    derived.album = original.album.as_ref().filter(|a| known(a)).cloned();
    if parts.len() == 4
        && !parts[2].is_empty()
        && parts[2].len() <= 3
        && parts[2].bytes().all(|b| b.is_ascii_digit())
        && (derived.artists.is_empty() || normalize(&artist) == normalize(parts[0]))
    {
        if derived.artists.is_empty() {
            derived.artists = vec![parts[0].into()];
        }
        derived.title = parts[3].into();
        if derived.album.is_none() {
            derived.album = Some(parts[1].into());
        }
    } else if parts.len() >= 2 {
        if !derived.artists.is_empty() && normalize(&artist) == normalize(parts[0]) {
            derived.title = title[SEP.find(title).unwrap().end()..].into();
        } else if !derived.artists.is_empty()
            && normalize(&artist) == normalize(parts.last().unwrap())
        {
            derived.title = title[..SEP.find_iter(title).last().unwrap().start()].into();
        } else if derived.artists.is_empty() && parts.len() == 2 && known(parts[0]) {
            derived.artists = vec![parts[0].into()];
            derived.title = parts[1].into();
        }
    }
    let tagged_title = derived.title.clone();
    let mut result = vec![];
    for r in [derived, original.clone()] {
        if r.title.trim().is_empty() || r.artists.is_empty() || !r.artists.iter().all(|a| known(a))
        {
            continue;
        }
        let inferred =
            r.title != original.title || r.artists != original.artists || r.album != original.album;
        push(
            &mut result,
            Query {
                recording: r,
                inferred,
            },
        );
    }
    if let Some(name) = original
        .file_name
        .as_ref()
        .filter(|s| !s.trim().is_empty() && *s != &original.title)
    {
        let mut from_file = original.clone();
        from_file.title = name.clone();
        from_file.file_name = None;
        for mut q in plan(&from_file) {
            // A filename may fill missing metadata, never remove a tagged edition.
            if !EXT.is_match(original.title.trim())
                && known(&original.title)
                && super::matching::comparison(&tagged_title)
                    != super::matching::comparison(&q.recording.title)
            {
                continue;
            }
            q.inferred = true;
            q.recording.file_name = Some(name.clone());
            push(&mut result, q);
        }
    }
    result.truncate(3);
    result
}
fn push(result: &mut Vec<Query>, query: Query) {
    if !result.iter().any(|q| {
        q.recording.title == query.recording.title
            && q.recording.artists == query.recording.artists
            && q.recording.album == query.recording.album
    }) {
        result.push(query);
    }
}
