use schemars::{
    JsonSchema,
    schema::{InstanceType, Schema, SchemaObject, StringValidation},
};
use serde::{Deserialize, Serialize};

/// Canonical decimal u64 on the wire, including values beyond JS safe integers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Counter(u64);
impl Counter {
    pub fn value(&self) -> u64 {
        self.0
    }
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}
impl TryFrom<String> for Counter {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let parsed = value
            .parse::<u64>()
            .map_err(|_| "invalid unsigned counter")?;
        if parsed.to_string() != value {
            return Err("counter must be canonical decimal");
        }
        Ok(Self(parsed))
    }
}
impl From<Counter> for String {
    fn from(value: Counter) -> Self {
        value.0.to_string()
    }
}
impl JsonSchema for Counter {
    fn schema_name() -> String {
        "Counter".into()
    }
    fn json_schema(_: &mut schemars::r#gen::SchemaGenerator) -> Schema {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            format: Some("uint64-decimal".into()),
            string: Some(Box::new(StringValidation {
                pattern: Some("^(0|[1-9][0-9]{0,19})$".into()),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    Linux,
    Macos,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerId {
    pub platform: Platform,
    pub native_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SessionId(pub String);
impl From<&str> for SessionId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recording {
    pub id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub duration_ms: Option<Counter>,
    pub version_label: Option<String>,
    pub file_name: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Buffering,
    Stopped,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackSnapshot {
    pub session_id: SessionId,
    pub player_id: PlayerId,
    pub player_name: String,
    pub recording: Option<Recording>,
    pub status: PlaybackStatus,
    pub position_ms: Option<Counter>,
    pub observed_at_ms: Counter,
    pub rate: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LyricWord {
    pub text: String,
    pub start_ms: Counter,
    pub end_ms: Option<Counter>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LyricLine {
    pub id: String,
    pub text: String,
    pub start_ms: Option<Counter>,
    pub end_ms: Option<Counter>,
    pub words: Vec<LyricWord>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LyricOrigin {
    Provider,
    Import,
    Personal,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatchConfidence {
    Confident,
    Uncertain,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TimingKind {
    Supplied,
    Unsynced,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LyricVersion {
    pub id: String,
    pub recording_id: String,
    pub content_hash: String,
    pub origin: LyricOrigin,
    pub lines: Vec<LyricLine>,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub confidence: MatchConfidence,
    pub timing: TimingKind,
    pub provider_id: Option<String>,
    pub attribution: Option<String>,
    pub retention: String,
    pub provider_duration_ms: Option<Counter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMode {
    Native,
    Fixture,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capabilities {
    pub mode: RuntimeMode,
    pub platform: Platform,
    pub tracking_available: bool,
    pub library_available: bool,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSnapshot {
    #[schemars(range(min = 1, max = 1))]
    pub schema: u8,
    pub revision: Counter,
    pub generation: Counter,
    pub capabilities: Capabilities,
    pub sessions: Vec<PlaybackSnapshot>,
    pub tracking_enabled: bool,
}
impl RuntimeSnapshot {
    pub fn foundation() -> Self {
        Self {
            schema: 1,
            revision: Counter::new(0),
            generation: Counter::new(0),
            capabilities: Capabilities {
                mode: RuntimeMode::Native,
                platform: if cfg!(target_os = "windows") {
                    Platform::Windows
                } else if cfg!(target_os = "macos") {
                    Platform::Macos
                } else {
                    Platform::Linux
                },
                tracking_available: false,
                library_available: false,
                reason:
                    "Playback tracking and your library are coming in the next development steps."
                        .into(),
            },
            sessions: vec![],
            tracking_enabled: false,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    InvalidInput,
    Unsupported,
    PermissionDenied,
    Offline,
    RateLimited,
    StorageUnavailable,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandFailure {
    pub code: FailureCode,
    pub message: String,
    pub retry_after_ms: Option<Counter>,
}

/// Export root keeps every boundary type reachable in a single versioned schema.
#[derive(JsonSchema)]
pub struct Contract {
    pub snapshot: RuntimeSnapshot,
    pub lyric_version: LyricVersion,
    pub failure: CommandFailure,
}
