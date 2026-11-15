use super::*;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextAlignment {
    Left,
    #[default]
    Center,
    Right,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReaderAppearance {
    pub text_size: u32,
    pub alignment: TextAlignment,
    pub words: bool,
}
impl Default for ReaderAppearance {
    fn default() -> Self {
        Self {
            text_size: 24,
            alignment: TextAlignment::Center,
            words: true,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Appearance {
    pub main: ReaderAppearance,
    pub floating: ReaderAppearance,
    pub opacity: f64,
    pub reduced_motion: bool,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            main: ReaderAppearance::default(),
            floating: ReaderAppearance::default(),
            opacity: 0.95,
            reduced_motion: false,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayMode {
    #[default]
    Compact,
    Expanded,
    Minimal,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowGeometry {
    pub monitor: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionSummary {
    pub id: String,
    pub source: String,
    pub confidence: MatchConfidence,
    pub timing: TimingKind,
    pub origin: LyricOrigin,
}
impl From<&LyricVersion> for VersionSummary {
    fn from(v: &LyricVersion) -> Self {
        Self {
            id: v.id.clone(),
            source: v
                .source_name
                .clone()
                .unwrap_or_else(|| "Saved lyrics".into()),
            confidence: v.confidence.clone(),
            timing: v.timing.clone(),
            origin: v.origin.clone(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LibrarySort {
    Title,
    Artist,
    Recent,
    MostPlayed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryQuery {
    pub text: String,
    pub favorites_only: bool,
    pub sort: LibrarySort,
    pub offset: u32,
    pub limit: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryItem {
    pub recording: Recording,
    pub favorite: bool,
    pub play_count: Counter,
    pub last_played_ms: Option<Counter>,
    pub has_lyrics: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryDetail {
    pub item: LibraryItem,
    pub versions: Vec<LyricVersion>,
    pub selected: Option<String>,
    pub adjustments: Vec<VersionAdjustment>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionAdjustment {
    pub version_id: String,
    pub delay_ms: i32,
    pub estimated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReaderAction {
    Select {
        #[serde(rename = "versionId")]
        version_id: String,
    },
    Adjust {
        #[serde(rename = "versionId")]
        version_id: String,
        #[serde(rename = "delayMs")]
        delay_ms: i32,
        estimated: bool,
    },
    Align {
        #[serde(rename = "versionId")]
        version_id: String,
        line: u32,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchQuery {
    pub context: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub target: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProductCommand {
    BackupPreview {
        limits: BackupLimits,
    },
    BackupApply {
        token: String,
        settings: bool,
    },
    BackupExport {
        limits: BackupLimits,
    },
    BackupPoll,
    BackupCancel,
    DiagnosticsPreview,
    DiagnosticsExport,
    CurrentDetail,
    HideOverlay,
    Library {
        query: LibraryQuery,
    },
    Detail {
        id: String,
    },
    Favorite {
        id: String,
        value: bool,
    },
    Reader {
        action: ReaderAction,
    },
    Appearance {
        value: Appearance,
    },
    ResetAppearance,
    Search {
        query: SearchQuery,
    },
    SearchPoll {
        token: String,
    },
    SearchCancel {
        context: String,
    },
    SaveSearch {
        token: String,
        #[serde(rename = "versionId")]
        version_id: String,
    },
    ClearDownloads,
    ClearHistory,
    Overlay {
        mode: OverlayMode,
    },
    ClickThrough {
        enabled: bool,
    },
    Dock {
        side: String,
    },
    ResetOverlay,
    OpenReleases,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProductReply {
    Backup {
        state: BackupProgress,
    },
    Diagnostics {
        report: String,
    },
    Done,
    Library {
        items: Vec<LibraryItem>,
        more: bool,
    },
    Detail {
        detail: LibraryDetail,
    },
    Search {
        token: String,
        status: LookupStatus,
        message: String,
        versions: Vec<LyricVersion>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupLimits {
    pub max_bytes: Option<Counter>,
    pub max_nodes: Counter,
    pub max_string_bytes: Counter,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackupStage {
    #[default]
    Idle,
    Busy,
    Preview,
    Done,
    Failed,
    Cancelling,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupProgress {
    pub stage: BackupStage,
    pub token: Option<String>,
    pub message: String,
    pub summary: Vec<String>,
}
