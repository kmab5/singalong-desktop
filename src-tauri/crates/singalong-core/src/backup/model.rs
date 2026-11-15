//! Backup-only wire models. Integers are lossless Rust integers, not frontend IPC.
use std::collections::BTreeMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Kotlin nullable fields without defaults are still required JSON properties.
pub(super) fn required_nullable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where D: serde::Deserializer<'de>, T: Deserialize<'de> { Option::<T>::deserialize(d) }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recording {
    pub id: String,
    pub title: String,
    pub artists: Vec<String>,
    #[serde(deserialize_with = "required_nullable")] pub album: Option<String>,
    #[serde(deserialize_with = "required_nullable")] pub duration_ms: Option<i64>,
    #[serde(default)] pub version_label: Option<String>,
    #[serde(default)] pub file_name: Option<String>,
}
impl Recording {
    pub fn identity_only(&self) -> Self {
        Self { album: None, duration_ms: None, version_label: None, file_name: None, ..self.clone() }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LyricOrigin { Provider, Import, Personal }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatchConfidence { Confident, Uncertain }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LyricWord { pub text: String, pub start_ms: i64, #[serde(default)] pub end_ms: Option<i64> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LyricLine {
    pub id: String, #[serde(deserialize_with = "required_nullable")] pub start_ms: Option<i64>, pub text: String,
    #[serde(default)] pub end_ms: Option<i64>,
    #[serde(default)] pub words: Vec<LyricWord>,
}
fn retention() -> String { "personal offline copy; no redistribution grant".into() }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LyricVersion {
    pub id: String, pub recording_id: String, pub content_hash: String,
    pub origin: LyricOrigin, pub lines: Vec<LyricLine>,
    #[serde(deserialize_with = "required_nullable")] pub source_name: Option<String>,
    #[serde(deserialize_with = "required_nullable")] pub source_url: Option<String>,
    pub confidence: MatchConfidence,
    #[serde(default)] pub provider_id: Option<String>,
    #[serde(default)] pub attribution: Option<String>,
    #[serde(default = "retention")] pub retention: String,
    #[serde(default)] pub provider_duration_ms: Option<i64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LyricReference {
    pub id: String, pub recording_id: String, pub content_hash: String,
    pub origin: LyricOrigin,
    #[serde(deserialize_with = "required_nullable")] pub source_name: Option<String>,
    #[serde(deserialize_with = "required_nullable")] pub source_url: Option<String>,
    pub confidence: MatchConfidence,
    #[serde(default)] pub provider_id: Option<String>,
    #[serde(default)] pub attribution: Option<String>,
    #[serde(default = "retention")] pub retention: String,
    #[serde(default)] pub provider_duration_ms: Option<i64>,
}
impl From<&LyricVersion> for LyricReference {
    fn from(v: &LyricVersion) -> Self {
        Self { id: v.id.clone(), recording_id: v.recording_id.clone(), content_hash: v.content_hash.clone(),
            origin: v.origin.clone(), source_name: v.source_name.clone(), source_url: v.source_url.clone(),
            confidence: v.confidence.clone(), provider_id: v.provider_id.clone(), attribution: v.attribution.clone(),
            retention: v.retention.clone(), provider_duration_ms: v.provider_duration_ms }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Adjustment { pub version_id: String, pub delay_ms: i64, pub allow_uncertain_sync: bool }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Translation {
    pub version_id: String, pub content_hash: String, pub provider: String,
    pub source_language: String, pub lines: BTreeMap<String, String>, pub attribution: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupSong {
    pub recording: Recording, pub plays: u64,
    #[serde(deserialize_with = "required_nullable")] pub last_played: Option<i64>, pub favorite: bool,
    pub versions: Vec<LyricVersion>,
    #[serde(deserialize_with = "required_nullable")] pub selected: Option<String>,
    #[serde(default)] pub adjustments: Vec<Adjustment>,
    #[serde(default)] pub translations: Vec<Translation>,
    #[serde(default)] pub lyric_references: Vec<LyricReference>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LyricsMode { #[default] Full, LinksMetadata, None }
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VersionScope { #[default] All, Preferred }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupOptions {
    pub lyrics: LyricsMode, pub song_metadata: bool, pub history: bool,
    pub settings: bool, pub version_scope: VersionScope,
}
impl Default for BackupOptions {
    fn default() -> Self { Self { lyrics: LyricsMode::Full, song_metadata: true, history: true, settings: true, version_scope: VersionScope::All } }
}
/// Typed preferences are data only. Applying settings needs a separate explicit allowlist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "lowercase", deny_unknown_fields)]
pub enum PreferenceValue {
    Boolean(bool), Int(i32), Long(i64), Float(f32), Double(f64), String(String), Strings(Vec<String>), Bytes(String),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupData {
    pub songs: Vec<BackupSong>,
    /// False/None means omitted, never a request to clear existing data.
    pub options: BackupOptions,
    pub settings: BTreeMap<String, PreferenceValue>,
    /// Exact Android outer string map, including savedPreferences JSON.
    pub android_preferences: BTreeMap<String, String>,
    /// Typed migration data retained without activating it.
    pub inert_preferences: BTreeMap<String, PreferenceValue>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackupSource { AndroidJsonV1, AndroidZipV1, DesktopV1 }
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupSummary {
    pub recordings: u64, pub versions: u64, pub references: u64, pub translations: u64,
    pub inert_preferences: u64, pub unapplied_settings: u64,
    pub omitted_sections: Vec<String>, pub preferred_only: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecodedBackup { pub source: BackupSource, pub data: BackupData, pub summary: BackupSummary }
