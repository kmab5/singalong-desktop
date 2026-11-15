use super::{LibraryRow, Result, Settings, StorageError};
use rusqlite::{Connection, OptionalExtension, params};
use singalong_core::{
    identity,
    model::{LyricVersion, Recording},
};
use std::{path::Path, time::Duration};

pub(super) struct Repository {
    connection: Connection,
}
impl Repository {
    pub fn record_play(
        &mut self,
        recording: String,
        occurrence: String,
        wall_ms: u64,
    ) -> Result<bool> {
        if occurrence.is_empty() || occurrence.len() > 256 || wall_ms > i64::MAX as u64 {
            return Err(StorageError::Invalid("invalid occurrence"));
        }
        let tx = self.connection.transaction()?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT recording_id FROM occurrences WHERE id=?",
                [&occurrence],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            if existing != recording {
                return Err(StorageError::Invalid(
                    "occurrence belongs to another recording",
                ));
            }
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO occurrences(id,recording_id,played_at_ms) VALUES(?1,?2,?3)",
            params![occurrence, recording, wall_ms as i64],
        )?;
        tx.execute("UPDATE recordings SET play_count=play_count+1,last_played_ms=MAX(COALESCE(last_played_ms,0),?2) WHERE id=?1", params![recording, wall_ms as i64])?;
        tx.commit()?;
        Ok(true)
    }
    pub fn open(path: &Path) -> Result<Self> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(3))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        let schema: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if schema > 1 {
            return Err(StorageError::UnsupportedSchema(schema));
        }
        if schema == 0 {
            let tx = connection.transaction()?;
            tx.execute_batch(include_str!("../../migrations/0001_initial.sql"))?;
            tx.commit()?;
        }
        // No destructive reset on failed migration, unknown schema or corrupt data.
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Ok(Self { connection })
    }

    pub fn resolve(&mut self, incoming: Recording, previous: Option<String>) -> Result<Recording> {
        self.match_recording(incoming, previous, true)?
            .ok_or(StorageError::Invalid("recording was not resolved"))
    }
    pub fn find(
        &mut self,
        incoming: Recording,
        previous: Option<String>,
    ) -> Result<Option<Recording>> {
        self.match_recording(incoming, previous, false)
    }
    fn match_recording(
        &mut self,
        incoming: Recording,
        previous: Option<String>,
        create: bool,
    ) -> Result<Option<Recording>> {
        if incoming.title.trim().is_empty()
            || incoming.title.len() > 16384
            || incoming.artists.len() > 100
        {
            return Err(StorageError::Invalid("invalid recording metadata"));
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let previous = previous
            .map(|id| {
                tx.query_row("SELECT content FROM recordings WHERE id=?", [id], |r| {
                    r.get::<_, String>(0)
                })
                .optional()
            })
            .transpose()?
            .flatten()
            .map(|json| serde_json::from_str::<Recording>(&json))
            .transpose()?;
        let previous =
            previous.filter(|recording| identity::same_observation(recording, &incoming));
        let candidates: Vec<Recording> = {
            let mut query =
                tx.prepare("SELECT content FROM recordings WHERE title_key=? ORDER BY id")?;
            let json = query
                .query_map([identity::normalize(&incoming.title)], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            json.iter()
                .map(|value| serde_json::from_str(value))
                .collect::<std::result::Result<_, _>>()?
        };
        let compatible: Vec<_> = candidates
            .iter()
            .filter(|recording| {
                !incoming.artists.is_empty() && identity::compatible(recording, &incoming)
            })
            .collect();
        let folder_candidates: Vec<Recording> = if previous.is_none() && compatible.is_empty() {
            let mut query = tx.prepare("SELECT content FROM recordings ORDER BY id")?;
            let json = query
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            json.iter()
                .map(|value| serde_json::from_str(value))
                .collect::<std::result::Result<_, _>>()?
        } else {
            vec![]
        };
        let hinted = identity::folder_match(&folder_candidates, &incoming);
        let exact: Vec<_> = compatible
            .iter()
            .copied()
            .filter(|recording| identity::exact(recording, &incoming))
            .collect();
        let existing = previous
            .as_ref()
            .or_else(|| {
                if compatible.len() == 1 {
                    Some(compatible[0])
                } else if exact.len() == 1 {
                    Some(exact[0])
                } else {
                    None
                }
            })
            .or(hinted);
        if !create {
            return Ok(existing.cloned());
        }
        let recording = if let Some(existing) = existing {
            identity::enrich_missing(existing, &incoming)
        } else {
            let mut recording = incoming;
            recording.id = uuid::Uuid::new_v4().to_string();
            recording
        };
        tx.execute("INSERT INTO recordings(id,title_key,content) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET content=excluded.content,title_key=excluded.title_key",params![recording.id,identity::normalize(&recording.title),serde_json::to_string(&recording)?])?;
        tx.commit()?;
        Ok(Some(recording))
    }

    pub fn save_versions(
        &mut self,
        recording_id: String,
        versions: Vec<LyricVersion>,
    ) -> Result<()> {
        self.save_versions_checked(recording_id, versions, &|| false)
    }
    pub fn save_versions_checked(
        &mut self,
        recording_id: String,
        versions: Vec<LyricVersion>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<()> {
        if cancelled() {
            return Err(StorageError::Invalid("lookup cancelled"));
        }
        if versions.is_empty() || versions.len() > 500 {
            return Err(StorageError::Invalid("invalid version count"));
        }
        let tx = self.connection.transaction()?;
        for version in versions {
            if version.recording_id != recording_id
                || version.id.is_empty()
                || version.lines.len() > 20000
                || !version
                    .lines
                    .iter()
                    .any(|line| !line.text.trim().is_empty())
                || version
                    .lines
                    .iter()
                    .map(|line| line.text.len())
                    .sum::<usize>()
                    > 2 * 1024 * 1024
            {
                return Err(StorageError::Invalid("invalid lyric version"));
            }
            let content = serde_json::to_string(&version)?;
            let existing: Option<String> = tx
                .query_row(
                    "SELECT content FROM lyric_versions WHERE id=?",
                    [&version.id],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(existing) = existing {
                if existing != content {
                    return Err(StorageError::Invalid("immutable version conflict"));
                }
            } else {
                tx.execute(
                    "INSERT INTO lyric_versions(id,recording_id,content) VALUES(?1,?2,?3)",
                    params![version.id, recording_id, content],
                )?;
            }
            tx.execute(
                "INSERT OR IGNORE INTO selections(recording_id,version_id) VALUES(?1,?2)",
                params![recording_id, version.id],
            )?;
        }
        if cancelled() {
            return Err(StorageError::Invalid("lookup cancelled"));
        }
        tx.commit()?;
        Ok(())
    }
    pub fn versions(&mut self, id: String) -> Result<Vec<LyricVersion>> {
        let mut query = self
            .connection
            .prepare("SELECT content FROM lyric_versions WHERE recording_id=? ORDER BY rowid")?;
        let json = query
            .query_map([id], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(json
            .iter()
            .map(|value| serde_json::from_str(value))
            .collect::<std::result::Result<_, _>>()?)
    }
    pub fn select(&mut self, recording: String, version: String) -> Result<()> {
        self.connection.execute("INSERT INTO selections(recording_id,version_id) VALUES(?1,?2) ON CONFLICT(recording_id) DO UPDATE SET version_id=excluded.version_id",params![recording,version])?;
        Ok(())
    }
    pub fn selected(&mut self, id: String) -> Result<Option<LyricVersion>> {
        let json:Option<String>=self.connection.query_row("SELECT v.content FROM selections s JOIN lyric_versions v ON v.id=s.version_id WHERE s.recording_id=?",[id],|r|r.get(0)).optional()?;
        Ok(json.map(|json| serde_json::from_str(&json)).transpose()?)
    }
    pub fn adjust(&mut self, id: String, delay: i64, estimated: bool) -> Result<()> {
        if !(-600000..=600000).contains(&delay) {
            return Err(StorageError::Invalid("delay outside supported range"));
        }
        self.connection.execute("INSERT INTO adjustments(version_id,delay_ms,estimated_sync) VALUES(?1,?2,?3) ON CONFLICT(version_id) DO UPDATE SET delay_ms=excluded.delay_ms,estimated_sync=excluded.estimated_sync",params![id,delay,estimated])?;
        Ok(())
    }
    pub fn adjustment(&mut self, id: String) -> Result<(i64, bool)> {
        Ok(self
            .connection
            .query_row(
                "SELECT delay_ms,estimated_sync FROM adjustments WHERE version_id=?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .unwrap_or((0, false)))
    }
    pub fn favorite(&mut self, id: String, value: bool) -> Result<()> {
        if self.connection.execute(
            "UPDATE recordings SET favorite=?1 WHERE id=?2",
            params![value, id],
        )? != 1
        {
            return Err(StorageError::Invalid("recording not found"));
        }
        Ok(())
    }
    pub fn list(&mut self, offset: u32, limit: u32) -> Result<Vec<LibraryRow>> {
        if limit == 0 || limit > 200 {
            return Err(StorageError::Invalid("page size must be 1..=200"));
        }
        let mut query=self.connection.prepare("SELECT content,favorite,play_count,last_played_ms FROM recordings ORDER BY title_key,id LIMIT ?1 OFFSET ?2")?;
        let rows = query
            .query_map(params![limit, offset], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, bool>(1)?,
                    r.get::<_, u64>(2)?,
                    r.get::<_, Option<u64>>(3)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(content, favorite, play_count, last_played_ms)| {
                Ok(LibraryRow {
                    recording: serde_json::from_str(&content)?,
                    favorite,
                    play_count,
                    last_played_ms,
                })
            })
            .collect()
    }
    pub fn settings(&mut self) -> Result<Settings> {
        let json: Option<String> = self
            .connection
            .query_row("SELECT content FROM settings WHERE id=1", [], |r| r.get(0))
            .optional()?;
        json.map(|json| serde_json::from_str::<Settings>(&json)?.sanitized())
            .unwrap_or_else(|| Ok(Settings::default()))
    }
    pub fn save_settings(&mut self, settings: Settings) -> Result<()> {
        let content = serde_json::to_string(&settings.sanitized()?)?;
        self.connection.execute("INSERT INTO settings(id,content) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET content=excluded.content",[content])?;
        Ok(())
    }
}
