CREATE TABLE recordings (
    id TEXT PRIMARY KEY NOT NULL,
    title_key TEXT NOT NULL,
    content TEXT NOT NULL CHECK(json_valid(content)),
    favorite INTEGER NOT NULL DEFAULT 0 CHECK(favorite IN (0,1)),
    play_count INTEGER NOT NULL DEFAULT 0 CHECK(play_count >= 0),
    last_played_ms INTEGER CHECK(last_played_ms >= 0)
);
CREATE INDEX recordings_title ON recordings(title_key);
CREATE INDEX recordings_history ON recordings(last_played_ms DESC, id);
CREATE TABLE lyric_versions (
    id TEXT PRIMARY KEY NOT NULL,
    recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    content TEXT NOT NULL CHECK(json_valid(content)),
    UNIQUE(recording_id, id)
);
CREATE INDEX versions_recording ON lyric_versions(recording_id);
CREATE TABLE selections (
    recording_id TEXT PRIMARY KEY NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    version_id TEXT NOT NULL,
    FOREIGN KEY(recording_id, version_id) REFERENCES lyric_versions(recording_id, id) ON DELETE CASCADE
);
CREATE TABLE adjustments (
    version_id TEXT PRIMARY KEY NOT NULL REFERENCES lyric_versions(id) ON DELETE CASCADE,
    delay_ms INTEGER NOT NULL CHECK(delay_ms BETWEEN -600000 AND 600000),
    estimated_sync INTEGER NOT NULL CHECK(estimated_sync IN (0,1))
);
CREATE TABLE occurrences (
    id TEXT PRIMARY KEY NOT NULL,
    recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    played_at_ms INTEGER NOT NULL CHECK(played_at_ms >= 0)
);
CREATE INDEX occurrences_recording ON occurrences(recording_id, played_at_ms);
CREATE TABLE lyric_references (
    id TEXT PRIMARY KEY NOT NULL,
    recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    content TEXT NOT NULL CHECK(json_valid(content))
);
CREATE TABLE translations (
    version_id TEXT PRIMARY KEY NOT NULL REFERENCES lyric_versions(id) ON DELETE CASCADE,
    content TEXT NOT NULL CHECK(json_valid(content))
);
CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK(id=1), content TEXT NOT NULL CHECK(json_valid(content)));
CREATE TABLE inert_preferences (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
PRAGMA user_version=1;
