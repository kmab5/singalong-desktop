//! Worker-owned backup snapshots and merge transactions. No paths or IPC DTOs.
use super::{Result, Settings, Storage, StorageError};
use crate::platform::Cancellation;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use singalong_core::{backup as b, identity, model as m};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    sync::Arc,
};

const IMPORTED: &str = "backup.imported-version/";
const INERT: &str = "backup.inert/";
const DESKTOP: &str = "backup.desktop/";
const ANDROID: &str = "backup.android.primary";

#[derive(Debug, Clone, Default)]
pub struct MergeReport {
    pub recordings_added: u64,
    pub recordings_matched: u64,
    pub versions_added: u64,
    pub versions_skipped: u64,
    pub references_added: u64,
    pub references_skipped: u64,
    pub translations_added: u64,
    pub translations_skipped: u64,
    pub conflicts: u64,
    pub preferences_retained: u64,
    pub settings_applied: u64,
}
impl MergeReport {
    pub fn summary(&self) -> Vec<String> {
        vec![
            format!(
                "Recordings: {} added, {} matched",
                self.recordings_added, self.recordings_matched
            ),
            format!(
                "Lyric versions: {} added, {} already present",
                self.versions_added, self.versions_skipped
            ),
            format!(
                "References: {} added, {} already present",
                self.references_added, self.references_skipped
            ),
            format!(
                "Translations: {} added, {} skipped",
                self.translations_added, self.translations_skipped
            ),
            format!("Conflicts preserved: {}", self.conflicts),
            format!(
                "Preference records retained without activation: {}",
                self.preferences_retained
            ),
            format!("Supported settings changed: {}", self.settings_applied),
        ]
    }
}

fn check(cancel: &Cancellation) -> Result<()> {
    if cancel.cancelled() {
        Err(StorageError::Invalid("backup cancelled"))
    } else {
        Ok(())
    }
}
fn invalid(message: &'static str) -> StorageError {
    StorageError::Invalid(message)
}
fn sql_count(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| invalid("backup counter exceeds SQLite signed 64-bit range"))
}
fn signed(value: &m::Counter) -> Result<i64> {
    sql_count(value.value())
}
fn counter(value: i64) -> Result<m::Counter> {
    Ok(m::Counter::new(
        u64::try_from(value).map_err(|_| invalid("negative backup time"))?,
    ))
}
fn opt_signed(value: &Option<m::Counter>) -> Result<Option<i64>> {
    value.as_ref().map(signed).transpose()
}
fn opt_counter(value: Option<i64>) -> Result<Option<m::Counter>> {
    value.map(counter).transpose()
}
fn recording_out(r: &m::Recording) -> Result<b::Recording> {
    Ok(b::Recording {
        id: r.id.clone(),
        title: r.title.clone(),
        artists: r.artists.clone(),
        album: r.album.clone(),
        duration_ms: opt_signed(&r.duration_ms)?,
        version_label: r.version_label.clone(),
        file_name: r.file_name.clone(),
    })
}
fn recording_in(r: &b::Recording) -> Result<m::Recording> {
    Ok(m::Recording {
        id: r.id.clone(),
        title: r.title.clone(),
        artists: r.artists.clone(),
        album: r.album.clone(),
        duration_ms: opt_counter(r.duration_ms)?,
        version_label: r.version_label.clone(),
        file_name: r.file_name.clone(),
    })
}
fn version_out(v: &m::LyricVersion, cancel: &Cancellation) -> Result<b::LyricVersion> {
    let mut lines = Vec::new();
    for line in &v.lines {
        check(cancel)?;
        let mut words = Vec::new();
        for word in &line.words {
            check(cancel)?;
            words.push(b::LyricWord {
                text: word.text.clone(),
                start_ms: signed(&word.start_ms)?,
                end_ms: opt_signed(&word.end_ms)?,
            });
        }
        lines.push(b::LyricLine {
            id: line.id.clone(),
            text: line.text.clone(),
            start_ms: opt_signed(&line.start_ms)?,
            end_ms: opt_signed(&line.end_ms)?,
            words,
        });
    }
    Ok(b::LyricVersion {
        id: v.id.clone(),
        recording_id: v.recording_id.clone(),
        content_hash: v.content_hash.clone(),
        origin: match v.origin {
            m::LyricOrigin::Provider => b::LyricOrigin::Provider,
            m::LyricOrigin::Import => b::LyricOrigin::Import,
            m::LyricOrigin::Personal => b::LyricOrigin::Personal,
        },
        lines,
        source_name: v.source_name.clone(),
        source_url: v.source_url.clone(),
        confidence: match v.confidence {
            m::MatchConfidence::Confident => b::MatchConfidence::Confident,
            m::MatchConfidence::Uncertain => b::MatchConfidence::Uncertain,
        },
        provider_id: v.provider_id.clone(),
        attribution: v.attribution.clone(),
        retention: v.retention.clone(),
        provider_duration_ms: opt_signed(&v.provider_duration_ms)?,
    })
}
fn version_in(v: &b::LyricVersion, cancel: &Cancellation) -> Result<m::LyricVersion> {
    let mut lines = Vec::new();
    for line in &v.lines {
        check(cancel)?;
        let mut words = Vec::new();
        for word in &line.words {
            check(cancel)?;
            words.push(m::LyricWord {
                text: word.text.clone(),
                start_ms: counter(word.start_ms)?,
                end_ms: opt_counter(word.end_ms)?,
            });
        }
        lines.push(m::LyricLine {
            id: line.id.clone(),
            text: line.text.clone(),
            start_ms: opt_counter(line.start_ms)?,
            end_ms: opt_counter(line.end_ms)?,
            words,
        });
    }
    Ok(m::LyricVersion {
        id: v.id.clone(),
        recording_id: v.recording_id.clone(),
        content_hash: v.content_hash.clone(),
        origin: match v.origin {
            b::LyricOrigin::Provider => m::LyricOrigin::Provider,
            b::LyricOrigin::Import => m::LyricOrigin::Import,
            b::LyricOrigin::Personal => m::LyricOrigin::Personal,
        },
        timing: if v.lines.iter().any(|l| l.start_ms.is_some()) {
            m::TimingKind::Supplied
        } else {
            m::TimingKind::Unsynced
        },
        lines,
        source_name: v.source_name.clone(),
        source_url: v.source_url.clone(),
        confidence: match v.confidence {
            b::MatchConfidence::Confident => m::MatchConfidence::Confident,
            b::MatchConfidence::Uncertain => m::MatchConfidence::Uncertain,
        },
        provider_id: v.provider_id.clone(),
        attribution: v.attribution.clone(),
        retention: v.retention.clone(),
        provider_duration_ms: opt_counter(v.provider_duration_ms)?,
    })
}
fn hash<T: Serialize>(value: &T, cancel: &Cancellation) -> Result<String> {
    struct Sink<'a>(Sha256, &'a Cancellation);
    impl Write for Sink<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            for chunk in bytes.chunks(8192) {
                if self.1.cancelled() {
                    return Err(std::io::Error::other("backup cancelled"));
                }
                self.0.update(chunk);
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut sink = Sink(Sha256::new(), cancel);
    let result = serde_json::to_writer(&mut sink, value);
    check(cancel)?;
    result?;
    Ok(format!("{:x}", sink.0.finalize()))
}
fn validate(data: &b::BackupData, cancel: &Cancellation) -> Result<()> {
    let cancelled = || cancel.cancelled();
    // This boundary receives the immutable, already-budgeted codec result (or
    // trusted DB snapshot). Revalidate record invariants without imposing a new
    // hidden default parser cap on an import decoded with explicitly larger caps.
    let control = b::BackupControl {
        is_cancelled: &cancelled,
        parse_limits: b::ParseLimits {
            max_nodes: u64::MAX,
            max_string_bytes: u64::MAX,
        },
    };
    let result = b::validate_with_control(data, &control);
    check(cancel)?;
    result.map_err(|_| invalid("invalid backup records"))
}
impl Storage {
    pub fn backup_snapshot(&self, cancel: Cancellation) -> Result<b::BackupData> {
        self.call(move |db| {
            check(&cancel)?;
            let tx = db.connection.transaction()?;
            let snapshot = snapshot(&tx, &cancel)?;
            check(&cancel)?;
            tx.rollback()?;
            Ok(snapshot)
        })
    }
    pub fn backup_merge(
        &self,
        data: Arc<b::BackupData>,
        apply_settings: bool,
        preview: bool,
        cancel: Cancellation,
    ) -> Result<MergeReport> {
        self.call(move |db| {
            check(&cancel)?;
            validate(&data, &cancel)?;
            let tx = db
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let report = merge(&tx, &data, apply_settings, &cancel)?;
            check(&cancel)?;
            if preview {
                tx.rollback()?;
            } else {
                tx.commit()?;
            }
            Ok(report)
        })
    }
}

fn inert_get(db: &Connection, key: &str) -> Result<Option<String>> {
    Ok(db
        .query_row(
            "SELECT value FROM inert_preferences WHERE key=?",
            [key],
            |r| r.get(0),
        )
        .optional()?)
}
fn inert_insert(db: &Connection, key: &str, value: &str) -> Result<bool> {
    Ok(db.execute(
        "INSERT OR IGNORE INTO inert_preferences(key,value) VALUES(?1,?2)",
        params![key, value],
    )? != 0)
}
/// Retain a conflicting original as opaque typed data; exported keys never grant
/// authority and are never interpreted as SQL/internal keys on a subsequent import.
fn retain_conflict<T: Serialize>(
    db: &Connection,
    category: &str,
    name: &str,
    value: &T,
    report: &mut MergeReport,
    cancel: &Cancellation,
) -> Result<()> {
    check(cancel)?;
    let record = (category, name, value);
    let value = b::PreferenceValue::String(serde_json::to_string(&record)?);
    let encoded = serde_json::to_string(&value)?;
    let mut salt = 0u64;
    loop {
        check(cancel)?;
        let id = hash(&(&record, salt), cancel)?;
        let key = format!("{INERT}desktop.retained.{id}");
        match inert_get(db, &key)? {
            None => {
                inert_insert(db, &key, &encoded)?;
                report.preferences_retained += 1;
                break;
            }
            Some(old) if old == encoded => break,
            Some(_) => {
                salt = salt
                    .checked_add(1)
                    .ok_or(invalid("backup collision space exhausted"))?;
            }
        }
    }
    Ok(())
}
fn retain_preference(
    db: &Connection,
    prefix: &str,
    name: &str,
    value: &b::PreferenceValue,
    report: &mut MergeReport,
    cancel: &Cancellation,
) -> Result<()> {
    check(cancel)?;
    let key = format!("{prefix}{name}");
    let encoded = serde_json::to_string(value)?;
    match inert_get(db, &key)? {
        None => {
            inert_insert(db, &key, &encoded)?;
            report.preferences_retained += 1;
        }
        Some(old) if serde_json::from_str::<b::PreferenceValue>(&old)? != *value => {
            report.conflicts += 1;
            retain_conflict(db, prefix, name, value, report, cancel)?;
        }
        _ => {}
    }
    Ok(())
}

fn settings_read(db: &Connection) -> Result<Settings> {
    let raw: Option<String> = db
        .query_row("SELECT content FROM settings WHERE id=1", [], |r| r.get(0))
        .optional()?;
    raw.map(|s| serde_json::from_str::<Settings>(&s))
        .transpose()?
        .unwrap_or_default()
        .sanitized()
}
fn settings_export(settings: &Settings) -> Result<BTreeMap<String, b::PreferenceValue>> {
    let serde_json::Value::Object(fields) = serde_json::to_value(settings)? else {
        return Err(invalid("invalid settings snapshot"));
    };
    fields
        .into_iter()
        .map(|(key, v)| {
            let value = match v {
                serde_json::Value::Bool(v) => b::PreferenceValue::Boolean(v),
                serde_json::Value::String(v) => b::PreferenceValue::String(v),
                serde_json::Value::Number(v) if v.as_i64().is_some() => {
                    b::PreferenceValue::Long(v.as_i64().ok_or(invalid("invalid setting integer"))?)
                }
                serde_json::Value::Number(v) if v.as_u64().is_some() => {
                    return Err(invalid(
                        "setting integer exceeds signed 64-bit preference range",
                    ));
                }
                serde_json::Value::Number(v) => {
                    b::PreferenceValue::Double(v.as_f64().ok_or(invalid("invalid setting number"))?)
                }
                v => b::PreferenceValue::String(serde_json::to_string(&v)?),
            };
            Ok((key, value))
        })
        .collect()
}

fn snapshot(db: &Connection, cancel: &Cancellation) -> Result<b::BackupData> {
    let mut data = b::BackupData {
        songs: Vec::new(),
        options: b::BackupOptions::default(),
        settings: BTreeMap::new(),
        android_preferences: BTreeMap::new(),
        inert_preferences: BTreeMap::new(),
    };
    let mut query = db
        .prepare("SELECT content,favorite,play_count,last_played_ms FROM recordings ORDER BY id")?;
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        check(cancel)?;
        let recording = recording_out(&serde_json::from_str::<m::Recording>(
            &row.get::<_, String>(0)?,
        )?)?;
        let id = &recording.id;
        let mut song = b::BackupSong {
            recording: recording.clone(),
            favorite: row.get(1)?,
            plays: row.get(2)?,
            last_played: row.get(3)?,
            versions: Vec::new(),
            selected: None,
            adjustments: Vec::new(),
            translations: Vec::new(),
            lyric_references: Vec::new(),
        };
        let mut versions =
            db.prepare("SELECT content FROM lyric_versions WHERE recording_id=? ORDER BY id")?;
        let mut versions = versions.query([id])?;
        while let Some(v) = versions.next()? {
            check(cancel)?;
            let v = version_out(
                &serde_json::from_str::<m::LyricVersion>(&v.get::<_, String>(0)?)?,
                cancel,
            )?;
            let adjustment = db
                .query_row(
                    "SELECT delay_ms,estimated_sync FROM adjustments WHERE version_id=?",
                    [&v.id],
                    |r| {
                        Ok(b::Adjustment {
                            version_id: v.id.clone(),
                            delay_ms: r.get(0)?,
                            allow_uncertain_sync: r.get(1)?,
                        })
                    },
                )
                .optional()?;
            if let Some(a) = adjustment {
                song.adjustments.push(a);
            }
            let translation: Option<String> = db
                .query_row(
                    "SELECT content FROM translations WHERE version_id=?",
                    [&v.id],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(t) = translation {
                song.translations.push(serde_json::from_str(&t)?);
            }
            song.versions.push(v);
        }
        song.selected = db
            .query_row(
                "SELECT version_id FROM selections WHERE recording_id=?",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        let mut refs =
            db.prepare("SELECT content FROM lyric_references WHERE recording_id=? ORDER BY id")?;
        let mut refs = refs.query([id])?;
        while let Some(r) = refs.next()? {
            check(cancel)?;
            song.lyric_references
                .push(serde_json::from_str(&r.get::<_, String>(0)?)?);
        }
        data.songs.push(song);
    }
    if let Some(raw) = inert_get(db, ANDROID)? {
        data.android_preferences = serde_json::from_str(&raw)?;
    }
    let mut prefs = db.prepare("SELECT key,value FROM inert_preferences ORDER BY key")?;
    let mut prefs = prefs.query([])?;
    while let Some(row) = prefs.next()? {
        check(cancel)?;
        let key: String = row.get(0)?;
        let value: String = row.get(1)?;
        if let Some(key) = key.strip_prefix(INERT) {
            data.inert_preferences
                .insert(key.into(), serde_json::from_str(&value)?);
        } else if let Some(key) = key.strip_prefix(DESKTOP) {
            data.settings
                .insert(key.into(), serde_json::from_str(&value)?);
        } else if key != ANDROID
            && !key.starts_with(IMPORTED)
            && !key.starts_with("backup.recording-map/")
        {
            // Older/unrecognized storage rows remain opaque and are not activated.
            let name = format!("desktop.unmapped.{}", hash(&(&key, &value), cancel)?);
            data.inert_preferences
                .entry(name)
                .or_insert(b::PreferenceValue::String(serde_json::to_string(&(
                    key, value,
                ))?));
        }
        // Imported protection/mapping rows are implementation state, not preferences.
    }
    data.settings.extend(settings_export(&settings_read(db)?)?);
    validate(&data, cancel)?;
    Ok(data)
}

fn full_metadata(r: &m::Recording) -> bool {
    !r.artists.is_empty()
        && r.artists.iter().all(|a| !a.trim().is_empty())
        && r.album.as_ref().is_some_and(|a| !a.trim().is_empty())
        && r.duration_ms.as_ref().is_some_and(|d| d.value() > 0)
}
fn recording_get(db: &Connection, id: &str) -> Result<Option<m::Recording>> {
    let raw: Option<String> = db
        .query_row("SELECT content FROM recordings WHERE id=?", [id], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(raw.map(|s| serde_json::from_str(&s)).transpose()?)
}
fn recording_map_key(r: &b::Recording, cancel: &Cancellation) -> Result<String> {
    Ok(format!("backup.recording-map/{}", hash(r, cancel)?))
}
// Reservations express source ownership even when no corresponding DB row
// exists yet. Compatibility never grants another source its reserved ID.
fn owns_reserved_id(candidate: &str, source: &str, reserved: &BTreeSet<&str>) -> bool {
    candidate == source || !reserved.contains(candidate)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordingOriginal<'a> {
    source_id: &'a str,
    target_id: &'a str,
    recording: &'a b::Recording,
}
fn retain_recording_original(
    db: &Connection,
    incoming: &b::Recording,
    target: &str,
    report: &mut MergeReport,
    cancel: &Cancellation,
) -> Result<()> {
    retain_conflict(
        db,
        "recording-original",
        &incoming.id,
        &RecordingOriginal {
            source_id: &incoming.id,
            target_id: target,
            recording: incoming,
        },
        report,
        cancel,
    )
}
fn recordings_resolve(
    db: &Connection,
    data: &b::BackupData,
    report: &mut MergeReport,
    cancel: &Cancellation,
) -> Result<BTreeMap<String, String>> {
    let mut existing = BTreeMap::new();
    let mut q = db.prepare("SELECT id,content FROM recordings ORDER BY id")?;
    let mut rows = q.query([])?;
    while let Some(row) = rows.next()? {
        check(cancel)?;
        existing.insert(
            row.get::<_, String>(0)?,
            serde_json::from_str::<m::Recording>(&row.get::<_, String>(1)?)?,
        );
    }
    let mut reserved = BTreeSet::new();
    for song in &data.songs {
        check(cancel)?;
        reserved.insert(song.recording.id.as_str());
    }
    let mut claimed = BTreeSet::new();
    let mut resolved = BTreeMap::new();
    // All incoming IDs are already reserved, including absent/incompatible IDs.
    // Reserve resolved targets before considering fuzzy matches or allocations.
    for song in &data.songs {
        check(cancel)?;
        let incoming = recording_in(&song.recording)?;
        let known = if existing
            .get(&incoming.id)
            .is_some_and(|r| identity::same_observation(r, &incoming))
        {
            Some(incoming.id.clone())
        } else {
            inert_get(db, &recording_map_key(&song.recording, cancel)?)?.filter(|id| {
                existing
                    .get(id)
                    .is_some_and(|r| identity::same_observation(r, &incoming))
            })
        };
        if let Some(id) = known {
            if !claimed.contains(&id) && owns_reserved_id(&id, &incoming.id, &reserved) {
                claimed.insert(id.clone());
                resolved.insert(incoming.id, id);
            }
        }
    }
    for song in &data.songs {
        check(cancel)?;
        let incoming = recording_in(&song.recording)?;
        let mut target = resolved.get(&incoming.id).cloned();
        if target.is_none()
            && !existing.contains_key(&incoming.id)
            && data.options.song_metadata
            && full_metadata(&incoming)
        {
            let mut candidates = Vec::new();
            for (id, r) in &existing {
                check(cancel)?;
                if !claimed.contains(id)
                    && !reserved.contains(id.as_str())
                    && full_metadata(r)
                    && identity::compatible(r, &incoming)
                {
                    candidates.push(id.clone());
                    if candidates.len() > 1 {
                        break;
                    }
                }
            }
            if candidates.len() == 1 {
                target = candidates.pop();
            }
        }
        let id = if let Some(target) = target {
            target
        } else {
            let mut candidate = incoming.id.clone();
            let mut salt = 0u64;
            loop {
                check(cancel)?;
                if owns_reserved_id(&candidate, &incoming.id, &reserved)
                    && !claimed.contains(&candidate)
                {
                    match recording_get(db, &candidate)? {
                        Some(old) if identity::same_observation(&old, &incoming) => {
                            break candidate;
                        }
                        None => break candidate,
                        Some(_) => {}
                    }
                }
                candidate = format!(
                    "backup-recording-{}",
                    hash(&(&song.recording, salt), cancel)?
                );
                salt = salt
                    .checked_add(1)
                    .ok_or(invalid("backup collision space exhausted"))?;
            }
        };
        if !owns_reserved_id(&id, &incoming.id, &reserved) {
            return Err(invalid("backup recording ownership conflict"));
        }
        let old = recording_get(db, &id)?;
        if old.is_some() {
            report.recordings_matched += 1;
        } else {
            report.recordings_added += 1;
        }
        if id != incoming.id && existing.contains_key(&incoming.id) {
            report.conflicts += 1;
        }
        let mut stored = if let Some(old) = old {
            if data.options.song_metadata {
                identity::enrich_missing(&old, &incoming)
            } else {
                old
            }
        } else {
            incoming.clone()
        };
        stored.id = id.clone();
        // Keep source spellings/filename/duration even when normalized matching
        // or missing-field enrichment chooses existing active metadata. A remap
        // also preserves the complete original plus its explicit relationship.
        if recording_out(&stored)? != song.recording {
            retain_recording_original(db, &song.recording, &id, report, cancel)?;
        }
        let count = if data.options.history {
            sql_count(song.plays)?
        } else {
            0
        };
        let last = if data.options.history {
            song.last_played
        } else {
            None
        };
        let favorite = data.options.song_metadata && song.favorite;
        db.execute("INSERT INTO recordings(id,title_key,content,favorite,play_count,last_played_ms) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET title_key=excluded.title_key,content=excluded.content,favorite=MAX(recordings.favorite,excluded.favorite),play_count=MAX(recordings.play_count,excluded.play_count),last_played_ms=CASE WHEN excluded.last_played_ms IS NULL THEN recordings.last_played_ms WHEN recordings.last_played_ms IS NULL THEN excluded.last_played_ms ELSE MAX(recordings.last_played_ms,excluded.last_played_ms) END",
            params![id, identity::normalize(&stored.title), serde_json::to_string(&stored)?, favorite, count, last])?;
        let map_key = recording_map_key(&song.recording, cancel)?;
        if let Some(previous) = inert_get(db, &map_key)? {
            if previous != id {
                // A stale memo may point at another incoming source's stable ID.
                // Preserve its historical relationship, but never reuse that ID
                // or leave an unusable memo that can change ownership next time.
                retain_recording_original(db, &song.recording, &previous, report, cancel)?;
            }
        }
        db.execute(
            "INSERT INTO inert_preferences(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![map_key, id],
        )?;
        claimed.insert(id.clone());
        resolved.insert(incoming.id, id);
    }
    Ok(resolved)
}
fn version_get(
    db: &Connection,
    id: &str,
    cancel: &Cancellation,
) -> Result<Option<b::LyricVersion>> {
    let raw: Option<String> = db
        .query_row("SELECT content FROM lyric_versions WHERE id=?", [id], |r| {
            r.get(0)
        })
        .optional()?;
    raw.map(|s| version_out(&serde_json::from_str(&s)?, cancel))
        .transpose()
}
fn reference_get(db: &Connection, id: &str) -> Result<Option<b::LyricReference>> {
    let raw: Option<String> = db
        .query_row(
            "SELECT content FROM lyric_references WHERE id=?",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(raw.map(|s| serde_json::from_str(&s)).transpose()?)
}
fn version_target(
    db: &Connection,
    reference: &b::LyricReference,
    version: Option<&b::LyricVersion>,
    reserved: &BTreeSet<&str>,
    cancel: &Cancellation,
) -> Result<String> {
    let mut candidate = reference.id.clone();
    let mut salt = 0u64;
    let mut content_conflict = false;
    loop {
        check(cancel)?;
        if owns_reserved_id(&candidate, &reference.id, reserved) {
            let mut expected_ref = reference.clone();
            expected_ref.id = candidate.clone();
            let ref_ok = reference_get(db, &candidate)?.is_none_or(|old| old == expected_ref);
            let version_ok = if let Some(old) = version_get(db, &candidate, cancel)? {
                let same_provenance = b::LyricReference::from(&old) == expected_ref;
                if let Some(full) = version {
                    let mut expected = full.clone();
                    expected.id = candidate.clone();
                    let same_content = old == expected;
                    // Only an eligible, same-provenance full version with
                    // different content justifies leaving the shared sequence.
                    // Reserved IDs and unrelated occupants never do so.
                    if ref_ok && same_provenance && !same_content && !content_conflict {
                        content_conflict = true;
                        salt = 0;
                    }
                    same_content
                } else {
                    same_provenance
                }
            } else {
                true
            };
            if ref_ok && version_ok {
                return Ok(candidate);
            }
        }
        // Links and full imports share every provenance-only salt, not just the
        // first remap. Preserve the original first-remap hash for existing data.
        candidate = if content_conflict {
            let full = version.ok_or(invalid("missing conflicting backup version"))?;
            format!(
                "backup-version-{}",
                hash(&("full-content", reference, full, salt), cancel)?
            )
        } else if salt == 0 {
            format!("backup-version-{}", hash(reference, cancel)?)
        } else {
            format!(
                "backup-version-{}",
                hash(&("provenance", reference, salt), cancel)?
            )
        };
        salt = salt
            .checked_add(1)
            .ok_or(invalid("backup collision space exhausted"))?;
    }
}

fn merge(
    db: &Connection,
    data: &b::BackupData,
    apply_settings: bool,
    cancel: &Cancellation,
) -> Result<MergeReport> {
    let mut report = MergeReport::default();
    let mut reserved_versions = BTreeSet::new();
    // Reject unsupported counts before any row changes, even during preview.
    for song in &data.songs {
        check(cancel)?;
        sql_count(song.plays)?;
        for id in song
            .versions
            .iter()
            .map(|v| v.id.as_str())
            .chain(song.lyric_references.iter().map(|r| r.id.as_str()))
        {
            check(cancel)?;
            reserved_versions.insert(id);
        }
    }
    let recordings = recordings_resolve(db, data, &mut report, cancel)?;
    for song in &data.songs {
        check(cancel)?;
        let recording_id = recordings
            .get(&song.recording.id)
            .ok_or(invalid("missing backup recording mapping"))?;
        let mut version_ids = BTreeMap::new();
        for original in &song.versions {
            check(cancel)?;
            let mut version = original.clone();
            version.recording_id = recording_id.clone();
            let target = version_target(
                db,
                &b::LyricReference::from(&version),
                Some(&version),
                &reserved_versions,
                cancel,
            )?;
            if target != original.id {
                report.conflicts += 1;
            }
            version.id = target.clone();
            if version_get(db, &target, cancel)?.is_some() {
                report.versions_skipped += 1;
            } else {
                db.execute(
                    "INSERT INTO lyric_versions(id,recording_id,content) VALUES(?1,?2,?3)",
                    params![
                        target,
                        recording_id,
                        serde_json::to_string(&version_in(&version, cancel)?)?
                    ],
                )?;
                report.versions_added += 1;
            }
            // Protect imported provider copies while retaining their source origin.
            inert_insert(db, &format!("{IMPORTED}{target}"), "true")?;
            version_ids.insert(original.id.clone(), target);
        }
        for original in &song.lyric_references {
            check(cancel)?;
            let mut reference = original.clone();
            reference.recording_id = recording_id.clone();
            let target = if let Some(target) = version_ids.get(&original.id) {
                target.clone()
            } else {
                version_target(db, &reference, None, &reserved_versions, cancel)?
            };
            if target != original.id && !version_ids.contains_key(&original.id) {
                report.conflicts += 1;
            }
            reference.id = target.clone();
            if reference_get(db, &target)?.is_some() {
                report.references_skipped += 1;
            } else {
                db.execute(
                    "INSERT INTO lyric_references(id,recording_id,content) VALUES(?1,?2,?3)",
                    params![target, recording_id, serde_json::to_string(&reference)?],
                )?;
                report.references_added += 1;
            }
            version_ids.insert(original.id.clone(), target);
        }
        if let Some(selected) = &song.selected {
            let selected = version_ids
                .get(selected)
                .ok_or(invalid("missing selected version mapping"))?;
            let existing: Option<String> = db
                .query_row(
                    "SELECT version_id FROM selections WHERE recording_id=?",
                    [recording_id],
                    |r| r.get(0),
                )
                .optional()?;
            if existing.as_ref().is_some_and(|id| id != selected) {
                report.conflicts += 1;
                retain_conflict(db, "selection", recording_id, selected, &mut report, cancel)?;
            } else if existing.is_none() {
                db.execute(
                    "INSERT INTO selections(recording_id,version_id) VALUES(?1,?2)",
                    params![recording_id, selected],
                )?;
            }
        }
        for original in &song.adjustments {
            check(cancel)?;
            let version_id = version_ids
                .get(&original.version_id)
                .ok_or(invalid("missing adjustment mapping"))?;
            let a = b::Adjustment {
                version_id: version_id.clone(),
                ..original.clone()
            };
            let existing: Option<(i64, bool)> = db
                .query_row(
                    "SELECT delay_ms,estimated_sync FROM adjustments WHERE version_id=?",
                    [version_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            if let Some(old) = existing {
                if old != (a.delay_ms, a.allow_uncertain_sync) {
                    report.conflicts += 1;
                    retain_conflict(db, "adjustment", version_id, &a, &mut report, cancel)?;
                }
            } else {
                db.execute(
                    "INSERT INTO adjustments(version_id,delay_ms,estimated_sync) VALUES(?1,?2,?3)",
                    params![version_id, a.delay_ms, a.allow_uncertain_sync],
                )?;
            }
        }
        for original in &song.translations {
            check(cancel)?;
            let version_id = version_ids
                .get(&original.version_id)
                .ok_or(invalid("missing translation mapping"))?;
            // Line IDs are scoped to each immutable version and intentionally do
            // not change when the globally unique version/recording ID changes.
            let translated = b::Translation {
                version_id: version_id.clone(),
                ..original.clone()
            };
            let existing: Option<String> = db
                .query_row(
                    "SELECT content FROM translations WHERE version_id=?",
                    [version_id],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(old) = existing {
                report.translations_skipped += 1;
                if serde_json::from_str::<b::Translation>(&old)? != translated {
                    report.conflicts += 1;
                    retain_conflict(
                        db,
                        "translation",
                        version_id,
                        &translated,
                        &mut report,
                        cancel,
                    )?;
                }
            } else {
                db.execute(
                    "INSERT INTO translations(version_id,content) VALUES(?1,?2)",
                    params![version_id, serde_json::to_string(&translated)?],
                )?;
                report.translations_added += 1;
            }
        }
    }
    if data.options.settings {
        merge_preferences(db, data, apply_settings, &mut report, cancel)?;
    }
    // A merge may combine individually valid archives into one recording. Keep
    // the resulting database exportable under the same record invariants rather
    // than committing 501 versions/references or incoherent attachments.
    let _ = snapshot(db, cancel)?;
    check(cancel)?;
    Ok(report)
}

fn merge_preferences(
    db: &Connection,
    data: &b::BackupData,
    apply: bool,
    report: &mut MergeReport,
    cancel: &Cancellation,
) -> Result<()> {
    for (key, value) in &data.inert_preferences {
        retain_preference(db, INERT, key, value, report, cancel)?;
    }
    if !data.android_preferences.is_empty() {
        check(cancel)?;
        let encoded = serde_json::to_string(&data.android_preferences)?;
        let mut compatible = true;
        if let Some(raw) = data.android_preferences.get("savedPreferences") {
            let values: BTreeMap<String, b::PreferenceValue> = serde_json::from_str(raw)?;
            for (key, value) in values {
                check(cancel)?;
                if let Some(old) = inert_get(db, &format!("{INERT}{key}"))? {
                    if serde_json::from_str::<b::PreferenceValue>(&old)? != value {
                        compatible = false;
                        break;
                    }
                }
            }
        }
        match inert_get(db, ANDROID)? {
            None if compatible => {
                inert_insert(db, ANDROID, &encoded)?;
                report.preferences_retained += data.android_preferences.len() as u64;
            }
            Some(old)
                if serde_json::from_str::<BTreeMap<String, String>>(&old)?
                    == data.android_preferences => {}
            _ => {
                report.conflicts += 1;
                retain_conflict(
                    db,
                    "android-outer",
                    "preferences",
                    &data.android_preferences,
                    report,
                    cancel,
                )?;
            }
        }
    }
    let mut settings = settings_read(db)?;
    for (key, value) in &data.settings {
        check(cancel)?;
        retain_preference(db, DESKTOP, key, value, report, cancel)?;
        // The active snapshot may supersede this field during export. Always keep
        // the supplied original separately so disallowed choices are not lost.
        retain_conflict(db, "desktop-original", key, value, report, cancel)?;
        if apply && apply_setting(&mut settings, key, value)? {
            report.settings_applied += 1;
        }
    }
    if apply && report.settings_applied != 0 {
        let settings = settings.sanitized()?;
        db.execute("INSERT INTO settings(id,content) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET content=excluded.content", [serde_json::to_string(&settings)?])?;
    }
    Ok(())
}
fn apply_setting(settings: &mut Settings, name: &str, value: &b::PreferenceValue) -> Result<bool> {
    let before = serde_json::to_string(settings)?;
    match (name, value) {
        ("textSize", b::PreferenceValue::Int(v)) => {
            if (14..=64).contains(v) {
                settings.text_size = *v as u32;
            }
        }
        ("textSize", b::PreferenceValue::Long(v)) => {
            if (14..=64).contains(v) {
                settings.text_size = *v as u32;
            }
        }
        ("opacity", b::PreferenceValue::Double(v)) => {
            if v.is_finite() && (0.2..=1.0).contains(v) {
                settings.opacity = *v;
            }
        }
        ("opacity", b::PreferenceValue::Float(v)) => {
            if v.is_finite() && (0.2..=1.0).contains(v) {
                settings.opacity = *v as f64;
            }
        }
        ("overlayMode", b::PreferenceValue::String(v)) => {
            if let Ok(mode) =
                serde_json::from_value::<m::OverlayMode>(serde_json::Value::String(v.clone()))
            {
                settings.overlay_mode = mode;
            }
        }
        ("appearance", b::PreferenceValue::String(v)) => {
            if let Ok(appearance) = serde_json::from_str::<m::Appearance>(v) {
                if appearance.opacity.is_finite()
                    && (0.2..=1.0).contains(&appearance.opacity)
                    && (14..=64).contains(&appearance.main.text_size)
                    && (14..=64).contains(&appearance.floating.text_size)
                {
                    settings.appearance = appearance;
                }
            }
        }
        // Never deserialize a whole supplied Settings object into active state.
        _ => {}
    }
    Ok(serde_json::to_string(settings)? != before)
}
