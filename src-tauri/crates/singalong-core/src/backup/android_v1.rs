use super::{BackupError, Result, control::Context, digest, ensure, json, model::*};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

fn one() -> u32 {
    1
}
fn format() -> String {
    "singalong".into()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Document {
    #[serde(default = "one")]
    schema: u32,
    songs: Vec<BackupSong>,
    #[serde(default)]
    preferences: BTreeMap<String, String>,
    #[serde(default)]
    options: BackupOptions,
}
impl Document {
    pub(super) fn into_data(self, ctx: &Context<'_>) -> Result<BackupData> {
        ensure(self.schema == 1, "unsupported Android schema")?;
        for song in &self.songs {
            ctx.check()?;
            ensure(
                song.plays <= i64::MAX as u64,
                "Android play count exceeds Long range",
            )?;
        }
        let inert_preferences = match self.preferences.get("savedPreferences") {
            Some(raw) => json(raw.as_bytes(), ctx)?,
            None => BTreeMap::new(),
        };
        Ok(BackupData {
            songs: self.songs,
            options: self.options,
            settings: BTreeMap::new(),
            android_preferences: self.preferences,
            inert_preferences,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    #[serde(default = "format")]
    format: String,
    #[serde(default = "one")]
    schema: u32,
    options: BackupOptions,
    entries: Vec<IndexEntry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexEntry {
    path: String,
    bytes: u64,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Details {
    recordings: Vec<Recording>,
    #[serde(default)]
    media_files_included: bool,
    #[serde(default)]
    created_at_epoch_ms: Option<i64>,
    #[serde(default)]
    app_version: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    recording: Recording,
    favorite: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct History {
    recording_id: String,
    plays: u64,
    #[serde(deserialize_with = "required_nullable")]
    last_played: Option<i64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Lyrics {
    recording_id: String,
    versions: Vec<String>,
    references: Vec<LyricReference>,
    #[serde(deserialize_with = "required_nullable")]
    selected: Option<String>,
    adjustments: Vec<Adjustment>,
    translations: Vec<Translation>,
}
fn take<T: serde::de::DeserializeOwned>(
    entries: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    ctx: &Context<'_>,
) -> Result<T> {
    json(
        &entries
            .remove(path)
            .ok_or(BackupError::Invalid("missing archive entry"))?,
        ctx,
    )
}
fn keyed<T>(
    items: Vec<T>,
    key: impl Fn(&T) -> &str,
    ids: &BTreeSet<String>,
    ctx: &Context<'_>,
) -> Result<BTreeMap<String, T>> {
    let mut result = BTreeMap::new();
    for item in items {
        ctx.check()?;
        let id = key(&item).to_owned();
        ensure(
            ids.contains(&id) && result.insert(id, item).is_none(),
            "duplicate or unknown section recording",
        )?;
    }
    ensure(result.len() == ids.len(), "incomplete recording section")?;
    Ok(result)
}
pub(super) fn decode(
    mut entries: BTreeMap<String, Vec<u8>>,
    ctx: &Context<'_>,
) -> Result<BackupData> {
    let index: Index = take(&mut entries, "index.json", ctx)?;
    ensure(
        index.format == "singalong" && index.schema == 1,
        "unsupported Android archive schema",
    )?;
    let mut paths = BTreeSet::new();
    for item in index.entries {
        ctx.check()?;
        ensure(paths.insert(item.path.clone()), "duplicate manifest path")?;
        let bytes = entries
            .get(&item.path)
            .ok_or(BackupError::Invalid("manifest entry missing"))?;
        ensure(
            item.bytes == bytes.len() as u64 && item.sha256 == digest(bytes, ctx)?,
            "archive hash or size mismatch",
        )?;
    }
    ensure(paths.len() == entries.len(), "unindexed archive entries")?;
    let details: Details = take(&mut entries, "details.json", ctx)?;
    ensure(
        !details.media_files_included
            && details.created_at_epoch_ms.unwrap_or(0) >= 0
            && details
                .app_version
                .as_ref()
                .is_none_or(|s| s.encode_utf16().count() <= 100),
        "invalid archive details",
    )?;
    let mut ids = BTreeSet::new();
    for recording in &details.recordings {
        ctx.check()?;
        ensure(
            ids.insert(recording.id.clone()) && *recording == recording.identity_only(),
            "invalid archive identities",
        )?;
    }
    let o = index.options;
    let mut metadata = if o.song_metadata {
        keyed(
            take::<Vec<Metadata>>(&mut entries, "songs.json", ctx)?,
            |v| &v.recording.id,
            &ids,
            ctx,
        )?
    } else {
        BTreeMap::new()
    };
    let mut history = if o.history {
        keyed(
            take::<Vec<History>>(&mut entries, "history.json", ctx)?,
            |v| &v.recording_id,
            &ids,
            ctx,
        )?
    } else {
        BTreeMap::new()
    };
    let mut lyrics = if o.lyrics != LyricsMode::None {
        keyed(
            take::<Vec<Lyrics>>(&mut entries, "lyrics.json", ctx)?,
            |v| &v.recording_id,
            &ids,
            ctx,
        )?
    } else {
        BTreeMap::new()
    };
    let preferences = if o.settings {
        take(&mut entries, "settings.json", ctx)?
    } else {
        BTreeMap::new()
    };
    let mut songs = Vec::new();
    for identity in details.recordings {
        ctx.check()?;
        let meta = metadata.remove(&identity.id);
        ensure(
            meta.as_ref()
                .is_none_or(|m| m.recording.identity_only() == identity),
            "section identity disagreement",
        )?;
        let history = history.remove(&identity.id);
        let lyric = lyrics.remove(&identity.id);
        let mut versions = Vec::new();
        if let Some(lyric) = &lyric {
            ensure(lyric.versions.len() <= 500, "too many version files")?;
            for path in &lyric.versions {
                ctx.check()?;
                ensure(
                    path.strip_prefix("lyrics/")
                        .and_then(|s| s.strip_suffix(".json"))
                        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())),
                    "invalid version path",
                )?;
                versions.push(take(&mut entries, path, ctx)?);
            }
        }
        let (selected, adjustments, translations, lyric_references) = lyric
            .map(|l| (l.selected, l.adjustments, l.translations, l.references))
            .unwrap_or_default();
        songs.push(BackupSong {
            favorite: meta.as_ref().is_some_and(|m| m.favorite),
            recording: meta.map(|m| m.recording).unwrap_or(identity),
            plays: history.as_ref().map_or(0, |h| h.plays),
            last_played: history.and_then(|h| h.last_played),
            versions,
            selected,
            adjustments,
            translations,
            lyric_references,
        });
    }
    ensure(entries.is_empty(), "unexpected archive entries")?;
    Document {
        schema: 1,
        songs,
        preferences,
        options: o,
    }
    .into_data(ctx)
}
