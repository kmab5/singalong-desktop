use super::{lrc, lyricsfile, preference};
use crate::{identity::normalize, model::*};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::LazyLock};

static QUALIFIERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(live|remix|acoustic|instrumental|karaoke|demo|radio edit|extended)\b").unwrap()
});
pub fn qualifiers(value: &str) -> BTreeSet<String> {
    QUALIFIERS
        .find_iter(&normalize(value))
        .map(|m| m.as_str().into())
        .collect()
}
pub(crate) fn comparison(value: &str) -> String {
    normalize(value)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
fn source() -> String {
    "LRCLIB".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSong {
    pub id: i64,
    pub track_name: String,
    pub artist_name: String,
    #[serde(default)]
    pub album_name: String,
    #[serde(default)]
    pub duration: f64,
    #[serde(default)]
    pub instrumental: bool,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub lyricsfile: Option<String>,
    #[serde(default = "source")]
    pub source: String,
    pub source_url: Option<String>,
}

pub fn confidence(recording: &Recording, song: &ProviderSong) -> Option<MatchConfidence> {
    let artist = comparison(&recording.artists.join(", "));
    let title = search_title(recording);
    if comparison(&title) != comparison(&song.track_name)
        || artist.is_empty()
        || artist != comparison(&song.artist_name)
        || qualifiers(&title) != qualifiers(&song.track_name)
    {
        return None;
    }
    let duration = song.duration * 1000.0;
    if !song.duration.is_finite() || song.duration < 0.0 || duration > i64::MAX as f64 {
        return None;
    }
    let difference = recording
        .duration_ms
        .as_ref()
        .filter(|d| d.value() > 0)
        .filter(|_| duration > 0.0)
        .map(|d| (d.value() as f64 - duration).abs());
    if difference.is_some_and(|d| {
        d > (10_000.0f64).max(recording.duration_ms.as_ref().unwrap().value() as f64 * 0.1)
    }) {
        return None;
    }
    let album = recording
        .album
        .as_ref()
        .is_some_and(|a| !normalize(a).is_empty() && normalize(a) == normalize(&song.album_name));
    Some(if difference.is_some_and(|d| d <= 2000.0) && album {
        MatchConfidence::Confident
    } else {
        MatchConfidence::Uncertain
    })
}

/// Include separately tagged edition evidence in searches without modifying metadata.
pub fn search_title(recording: &Recording) -> String {
    let title = comparison(&recording.title);
    if let Some(label) = recording
        .version_label
        .as_ref()
        .filter(|s| !s.trim().is_empty())
    {
        let edition = comparison(label);
        if title != edition && !title.ends_with(&format!(" {edition}")) {
            return format!("{} ({label})", recording.title);
        }
    }
    recording.title.clone()
}

pub fn versions(
    recording: &Recording,
    songs: Vec<ProviderSong>,
    inferred: bool,
) -> Vec<LyricVersion> {
    let mut versions = vec![];
    for song in songs {
        let Some(mut confidence) = confidence(recording, &song) else {
            continue;
        };
        if song.instrumental {
            continue;
        }
        if inferred {
            confidence = MatchConfidence::Uncertain;
        }
        let lines = song
            .lyricsfile
            .as_deref()
            .and_then(lyricsfile::parse)
            .filter(|rows| rows.iter().any(|r| !r.text.trim().is_empty()))
            .unwrap_or_else(|| {
                lrc::parse(song.synced_lyrics.as_deref(), song.plain_lyrics.as_deref())
            });
        if !lines.iter().any(|r| !r.text.trim().is_empty()) {
            continue;
        }
        let content =
            serde_json::to_vec(&lines).expect("lyric lines contain only serializable values");
        let hash = format!("{:x}", Sha256::digest(content));
        let id = format!(
            "{}:{}:{}:{}",
            recording.id,
            song.source.to_lowercase(),
            song.id,
            hash
        );
        if versions.iter().any(|v: &LyricVersion| v.id == id) {
            continue;
        }
        let timing = if lines.iter().any(|r| r.start_ms.is_some()) {
            TimingKind::Supplied
        } else {
            TimingKind::Unsynced
        };
        let attribution = format!(
            "{} — {} · {} · {}{}",
            song.track_name,
            song.artist_name,
            song.album_name,
            song.source,
            if inferred { " · Inferred search" } else { "" }
        );
        let source_url = song.source_url.or_else(|| {
            (song.source == "LRCLIB").then(|| format!("https://lrclib.net/api/get/{}", song.id))
        });
        versions.push(LyricVersion {
            id,
            recording_id: recording.id.clone(),
            content_hash: hash,
            origin: LyricOrigin::Provider,
            lines,
            source_name: Some(song.source),
            source_url,
            confidence,
            timing,
            provider_id: Some(song.id.to_string()),
            attribution: Some(attribution),
            retention: "cached".into(),
            provider_duration_ms: (song.duration > 0.0)
                .then(|| Counter::new((song.duration * 1000.0) as u64)),
        });
    }
    preference::sort(
        &mut versions,
        recording.duration_ms.as_ref().map(Counter::value),
    );
    versions
}
