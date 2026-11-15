use std::collections::{BTreeMap, BTreeSet};
use base64::{ engine::{GeneralPurpose, GeneralPurposeConfig, DecodePaddingMode}, alphabet};
use super::{model::*, ensure, Result, BackupControl, control::{Context, CheckedReader}};
use std::io::Read;

fn len(s: &str) -> usize { s.encode_utf16().take(100_001).count() }
fn id(s: &str, max: usize) -> bool { len(s) <= max && !s.trim().is_empty() }
fn optional(s: &Option<String>, max: usize) -> bool { s.as_ref().is_none_or(|v| len(v) <= max) }
fn blank(s: &str, ctx: &Context<'_>) -> Result<bool> {
    for (i, c) in s.chars().enumerate() {
        if i % 1024 == 0 { ctx.check()?; }
        if !c.is_whitespace() { return Ok(false); }
    }
    ctx.check()?; Ok(true)
}
fn same_string(a: &str, b: &str, ctx: &Context<'_>) -> Result<bool> {
    if a.len() != b.len() { return Ok(false); }
    for (a, b) in a.as_bytes().chunks(8192).zip(b.as_bytes().chunks(8192)) {
        ctx.check()?; if a != b { return Ok(false); }
    }
    Ok(true)
}
fn same_preference(a: &PreferenceValue, b: &PreferenceValue, ctx: &Context<'_>) -> Result<bool> {
    match (a, b) {
        (PreferenceValue::String(a), PreferenceValue::String(b)) | (PreferenceValue::Bytes(a), PreferenceValue::Bytes(b)) => same_string(a, b, ctx),
        (PreferenceValue::Strings(a), PreferenceValue::Strings(b)) => {
            if a.len() != b.len() { return Ok(false); }
            for (a, b) in a.iter().zip(b) { ctx.check()?; if !same_string(a, b, ctx)? { return Ok(false); } }
            Ok(true)
        }
        // All other same-kind variants are scalar comparisons; unequal kinds
        // short-circuit without walking strings/arrays.
        _ => Ok(a == b),
    }
}

pub fn validate_preferences(values: &BTreeMap<String, PreferenceValue>) -> Result<()> {
    validate_preferences_with_control(values, &BackupControl::default())
}
pub fn validate_preferences_with_control(values: &BTreeMap<String, PreferenceValue>, control: &BackupControl<'_>) -> Result<()> {
    preferences_inner(values, &Context::new(control))
}
fn preferences_inner(values: &BTreeMap<String, PreferenceValue>, ctx: &Context<'_>) -> Result<()> {
    ctx.check()?;
    for (key, value) in values {
        ctx.check()?;
        ensure(!key.is_empty(), "invalid preference name")?;
        match value {
            PreferenceValue::Float(v) => ensure(v.is_finite(), "nonfinite preference")?,
            PreferenceValue::Double(v) => ensure(v.is_finite(), "nonfinite preference")?,
            PreferenceValue::Bytes(v) => {
                let engine = GeneralPurpose::new(&alphabet::STANDARD, GeneralPurposeConfig::new()
                    .with_decode_padding_mode(DecodePaddingMode::Indifferent).with_decode_allow_trailing_bits(true));
                let mut reader = base64::read::DecoderReader::new(CheckedReader::new(v.as_bytes(), ctx), &engine);
                let mut buffer = [0u8; 8192];
                loop {
                    ctx.check()?;
                    let result = reader.read(&mut buffer);
                    ctx.check()?;
                    let n = result.map_err(|_| super::BackupError::Invalid("invalid base64 preference"))?;
                    if n == 0 { break; }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Validation never normalizes IDs, hashes, text, timing, or provenance.
pub fn validate(data: &BackupData) -> Result<()> {
    validate_with_control(data, &BackupControl::default())
}
pub fn validate_with_control(data: &BackupData, control: &BackupControl<'_>) -> Result<()> {
    validate_inner(data, &Context::new(control))
}
pub(super) fn validate_inner(data: &BackupData, ctx: &Context<'_>) -> Result<()> {
    ctx.check()?;
    let o = &data.options;
    ensure(o.song_metadata || o.history || o.settings || o.lyrics != LyricsMode::None, "empty section selection")?;
    ensure(o.settings || (data.settings.is_empty() && data.android_preferences.is_empty() && data.inert_preferences.is_empty()), "settings in omitted section")?;
    ensure(o.song_metadata || o.history || o.lyrics != LyricsMode::None || data.songs.is_empty(), "songs in settings-only backup")?;
    preferences_inner(&data.settings, ctx)?;
    preferences_inner(&data.inert_preferences, ctx)?;
    for key in data.android_preferences.keys() { ctx.check()?; ensure(id(key, 200), "invalid Android preference name")?; }
    if let Some(raw) = data.android_preferences.get("savedPreferences") {
        let typed: BTreeMap<String, PreferenceValue> = super::json(raw.as_bytes(), ctx)?;
        preferences_inner(&typed, ctx)?;
        // Keep both representations coherent; raw bytes remain available for round trips.
        for (k, v) in &typed {
            ctx.check()?;
            let retained = data.inert_preferences.get(k).ok_or(super::BackupError::Invalid("missing inert preference"))?;
            ensure(same_preference(v, retained, ctx)?, "inconsistent inert preference snapshot")?;
        }
    }
    let mut recordings = BTreeSet::new();
    let mut versions = BTreeSet::new();
    let mut references = BTreeSet::new();
    let mut provenance = BTreeMap::new();
    for song in &data.songs {
        ctx.check()?;
        let r = &song.recording;
        ensure(recordings.insert(&r.id) && id(&r.id, 200) && id(&r.title, 4096), "invalid or duplicate recording")?;
        ensure(r.artists.len() <= 100 && r.artists.iter().all(|a| len(a) <= 4096)
            && optional(&r.album, 4096) && optional(&r.file_name, 4096)
            && r.duration_ms.unwrap_or(0) >= 0, "invalid recording metadata")?;
        ensure(song.last_played.unwrap_or(0) >= 0, "negative history timestamp")?;
        ensure(o.history || (song.plays == 0 && song.last_played.is_none()), "history in omitted section")?;
        ensure(o.song_metadata || (!song.favorite && *r == r.identity_only()), "metadata in omitted section")?;
        ensure(song.versions.len() <= 500 && song.lyric_references.len() <= 500, "too many versions or references")?;
        ensure(o.lyrics == LyricsMode::Full || (song.versions.is_empty() && song.selected.is_none() && song.adjustments.is_empty() && song.translations.is_empty()), "full lyrics in omitted section")?;
        ensure(o.lyrics != LyricsMode::None || song.lyric_references.is_empty(), "references in omitted section")?;
        if o.lyrics != LyricsMode::None && o.version_scope == VersionScope::Preferred {
            let ids: BTreeSet<_> = song.versions.iter().map(|v| &v.id).chain(song.lyric_references.iter().map(|v| &v.id)).collect();
            ensure(ids.len() <= 1, "multiple preferred versions")?;
        }
        let local: BTreeMap<_, _> = song.versions.iter().map(|v| (&v.id, v)).collect();
        for v in &song.versions {
            ctx.check()?;
            ensure(versions.insert(&v.id) && id(&v.id, 512) && v.recording_id == r.id && v.provider_duration_ms.unwrap_or(0) >= 0, "invalid or duplicate version")?;
            ensure(!v.lines.is_empty() && v.lines.len() <= 20_000, "invalid line count")?;
            let mut line_ids = BTreeSet::new();
            let mut text_bytes = 0usize;
            let mut previous = 0;
            for line in &v.lines {
                ctx.check()?;
                ensure(line_ids.insert(&line.id) && len(&line.id) <= 200 && len(&line.text) <= 100_000, "invalid lyric line")?;
                text_bytes = text_bytes.checked_add(line.text.len()).ok_or(super::BackupError::Invalid("lyric size overflow"))?;
                ensure(text_bytes <= 2 * 1024 * 1024, "lyric text exceeds 2 MiB")?;
                if let Some(start) = line.start_ms {
                    ensure(start >= previous, "unordered or negative line timing")?;
                    previous = start;
                }
                if let Some(end) = line.end_ms {
                    ensure(line.start_ms.is_some_and(|s| end >= s), "invalid line end")?;
                }
                if !line.words.is_empty() {
                    ensure(line.start_ms.is_some() && line.words.len() <= 10_000, "invalid word count or untimed words")?;
                    let mut remaining = line.text.as_str();
                    for word in &line.words {
                        ctx.check()?;
                        ensure(!word.text.is_empty() && word.start_ms >= 0 && word.end_ms.is_none_or(|e| e >= word.start_ms), "invalid word timing")?;
                        remaining = remaining.strip_prefix(&word.text).ok_or(super::BackupError::Invalid("word concatenation differs from line"))?;
                    }
                    ensure(remaining.is_empty(), "incomplete word concatenation")?;
                }
            }
            provenance.insert(&v.id, v);
        }
        for reference in &song.lyric_references {
            ctx.check()?;
            ensure(references.insert(&reference.id) && id(&reference.id, 512) && reference.recording_id == r.id, "invalid or duplicate reference")?;
            ensure(len(&reference.content_hash) <= 4096 && reference.provider_duration_ms.unwrap_or(0) >= 0
                && optional(&reference.source_name, 16_384) && optional(&reference.source_url, 16_384)
                && optional(&reference.provider_id, 16_384) && optional(&reference.attribution, 16_384)
                && len(&reference.retention) <= 16_384, "invalid reference provenance")?;
        }
        ensure(song.selected.as_ref().is_none_or(|s| local.contains_key(s)), "dangling selected version")?;
        let mut adjustments = BTreeSet::new();
        for a in &song.adjustments {
            ctx.check()?;
            ensure(adjustments.insert(&a.version_id) && local.contains_key(&a.version_id) && (-600_000..=600_000).contains(&a.delay_ms), "invalid adjustment")?;
        }
        let mut translations = BTreeSet::new();
        for t in &song.translations {
            ctx.check()?;
            ensure(translations.insert(&t.version_id), "duplicate translation")?;
            let v = local.get(&t.version_id).ok_or(super::BackupError::Invalid("dangling translation"))?;
            let mut expected = BTreeSet::new();
            for line in &v.lines { ctx.check()?; if !blank(&line.text, ctx)? { expected.insert(&line.id); } }
            ensure(t.lines.len() == expected.len(), "incomplete translation")?;
            for (key, text) in &t.lines {
                ctx.check()?;
                ensure(expected.contains(key) && len(text) <= 100_000 && !blank(text, ctx)?, "invalid translation line")?;
            }
            ensure(same_string(&t.content_hash, &v.content_hash, ctx)? && !blank(&t.provider, ctx)? && !blank(&t.source_language, ctx)?
                && !blank(&t.attribution, ctx)?, "invalid or incomplete translation")?;
        }
    }
    for reference in data.songs.iter().flat_map(|s| &s.lyric_references) {
        ctx.check()?;
        if let Some(v) = provenance.get(&reference.id) {
            // Reference strings have already passed the small per-field limits;
            // compare without allocating a second provenance object.
            ensure(v.recording_id == reference.recording_id && v.content_hash == reference.content_hash
                && v.origin == reference.origin && v.source_name == reference.source_name
                && v.source_url == reference.source_url && v.confidence == reference.confidence
                && v.provider_id == reference.provider_id && v.attribution == reference.attribution
                && v.retention == reference.retention && v.provider_duration_ms == reference.provider_duration_ms,
                "conflicting reference provenance")?;
        }
    }
    ctx.check()
}

pub(super) fn summary(data: &BackupData, ctx: &Context<'_>) -> Result<BackupSummary> {
    let mut omitted = Vec::new();
    if !data.options.song_metadata { omitted.push("songMetadata".into()); }
    if !data.options.history { omitted.push("history".into()); }
    if !data.options.settings { omitted.push("settings".into()); }
    if data.options.lyrics != LyricsMode::Full { omitted.push("lyricText".into()); }
    if data.options.lyrics == LyricsMode::None { omitted.push("lyricReferences".into()); }
    let mut versions = 0; let mut references = 0; let mut translations = 0;
    for song in &data.songs { ctx.check()?; versions += song.versions.len() as u64; references += song.lyric_references.len() as u64; translations += song.translations.len() as u64; }
    Ok(BackupSummary {
        recordings: data.songs.len() as u64,
        versions,
        references,
        translations,
        inert_preferences: data.inert_preferences.len() as u64,
        unapplied_settings: (data.settings.len() + data.android_preferences.len()) as u64,
        omitted_sections: omitted, preferred_only: data.options.version_scope == VersionScope::Preferred,
    })
}
