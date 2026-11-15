use super::{Result, Storage, StorageError, repository::Repository};
use rusqlite::OptionalExtension;
use singalong_core::{identity, model::*};
impl Storage {
    pub fn library(&self, query: LibraryQuery) -> Result<(Vec<LibraryItem>, bool)> {
        self.call(move |db| db.library(query))
    }
    pub fn detail(&self, id: String) -> Result<LibraryDetail> {
        self.call(move |db| db.detail(id))
    }
    pub fn version(&self, id: String) -> Result<LyricVersion> {
        self.call(move |db| db.version(id))
    }
    pub fn clear_downloads(&self) -> Result<()> {
        self.call(|db| db.clear_downloads())
    }
    pub fn clear_history(&self) -> Result<()> {
        self.call(|db| db.clear_history())
    }
}
impl Repository {
    pub(super) fn library(&mut self, query: LibraryQuery) -> Result<(Vec<LibraryItem>, bool)> {
        if query.text.len() > 4096 || query.limit == 0 || query.limit > 200 {
            return Err(StorageError::Invalid("invalid library query"));
        }
        let text = identity::normalize(&query.text);
        let order = match query.sort {
            LibrarySort::Title => "title_key,id",
            LibrarySort::Artist => "lower(json_extract(content,'$.artists[0]')),title_key,id",
            LibrarySort::Recent => "last_played_ms DESC,title_key,id",
            LibrarySort::MostPlayed => "play_count DESC,title_key,id",
        };
        let sql = format!(
            "SELECT content,favorite,play_count,last_played_ms,EXISTS(SELECT 1 FROM lyric_versions v WHERE v.recording_id=recordings.id) FROM recordings WHERE (?1=0 OR favorite=1) ORDER BY {order}"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let mut rows = statement.query([query.favorites_only])?;
        let mut items = Vec::new();
        let mut skipped = 0u32;
        let mut more = false;
        while let Some(row) = rows.next()? {
            let recording: Recording = serde_json::from_str(&row.get::<_, String>(0)?)?;
            if !text.is_empty()
                && !identity::normalize(&recording.title).contains(&text)
                && !recording
                    .artists
                    .iter()
                    .any(|artist| identity::normalize(artist).contains(&text))
            {
                continue;
            }
            if skipped < query.offset {
                skipped += 1;
                continue;
            }
            if items.len() == query.limit as usize {
                more = true;
                break;
            }
            items.push(LibraryItem {
                recording,
                favorite: row.get(1)?,
                play_count: Counter::new(row.get(2)?),
                last_played_ms: row.get::<_, Option<u64>>(3)?.map(Counter::new),
                has_lyrics: row.get(4)?,
            });
        }
        Ok((items, more))
    }
    pub(super) fn version(&mut self, id: String) -> Result<LyricVersion> {
        let value: Option<String> = self
            .connection
            .query_row(
                "SELECT content FROM lyric_versions WHERE id=?",
                [id],
                |row| row.get(0),
            )
            .optional()?;
        serde_json::from_str(&value.ok_or(StorageError::Invalid("lyric version not found"))?)
            .map_err(Into::into)
    }
    pub(super) fn detail(&mut self, id: String) -> Result<LibraryDetail> {
        let row: Option<(String, bool, u64, Option<u64>)> = self
            .connection
            .query_row(
                "SELECT content,favorite,play_count,last_played_ms FROM recordings WHERE id=?",
                [&id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let (content, favorite, count, last) =
            row.ok_or(StorageError::Invalid("recording not found"))?;
        let versions = self.versions(id.clone())?;
        let selected = self.selected(id)?.map(|v| v.id);
        let adjustments = versions
            .iter()
            .map(|version| {
                let (delay, estimated) = self.adjustment(version.id.clone())?;
                Ok(VersionAdjustment {
                    version_id: version.id.clone(),
                    delay_ms: delay as i32,
                    estimated,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(LibraryDetail {
            item: LibraryItem {
                recording: serde_json::from_str(&content)?,
                favorite,
                play_count: Counter::new(count),
                last_played_ms: last.map(Counter::new),
                has_lyrics: !versions.is_empty(),
            },
            versions,
            selected,
            adjustments,
        })
    }
    pub(super) fn clear_downloads(&mut self) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM lyric_versions WHERE json_extract(content,'$.origin')='provider' AND NOT EXISTS(SELECT 1 FROM inert_preferences p WHERE p.key='backup.imported-version/'||lyric_versions.id) AND recording_id IN (SELECT id FROM recordings WHERE favorite=0)",[])?;
        tx.commit()?;
        Ok(())
    }
    pub(super) fn clear_history(&mut self) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM occurrences", [])?;
        tx.execute("UPDATE recordings SET play_count=0,last_played_ms=NULL", [])?;
        tx.commit()?;
        Ok(())
    }
}
