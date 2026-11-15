/* Generated from Rust. Run npm run contract:generate; do not edit. */

export type RuntimeAction =
  | {
      kind: "show_overlay";
      visible: boolean;
    }
  | {
      enabled: boolean;
      kind: "reader_active";
    }
  | {
      enabled: boolean;
      kind: "offline";
    }
  | {
      kind: "retry_lyrics";
    }
  | {
      enabled: boolean;
      kind: "tracking";
    }
  | {
      enabled: boolean;
      kind: "enable_player";
      playerId: PlayerId;
    }
  | {
      kind: "select";
      mode: SelectionMode;
    }
  | {
      kind: "discover";
    }
  | {
      enabled: boolean;
      kind: "startup";
    }
  | {
      enabled: boolean;
      kind: "auto_overlay";
    }
  | {
      kind: "close_to_tray";
    }
  | {
      kind: "quit";
    };
export type Platform = "windows" | "linux" | "macos";
export type SelectionMode =
  | {
      kind: "automatic";
    }
  | {
      kind: "app_pin";
      playerId: PlayerId;
    }
  | {
      kind: "stream_pin";
      playerId: PlayerId;
      sessionId: string;
    };
export type FailureCode =
  | "invalid_input"
  | "unsupported"
  | "permission_denied"
  | "offline"
  | "rate_limited"
  | "storage_unavailable"
  | "cancelled";
export type Counter = string;
export type MatchConfidence = "confident" | "uncertain";
export type LyricOrigin = "provider" | "import" | "personal";
export type TimingKind = "supplied" | "unsynced";
export type ProductCommand =
  | {
      kind: "backup_preview";
      limits: BackupLimits;
    }
  | {
      kind: "backup_apply";
      settings: boolean;
      token: string;
    }
  | {
      kind: "backup_export";
      limits: BackupLimits;
    }
  | {
      kind: "backup_poll";
    }
  | {
      kind: "backup_cancel";
    }
  | {
      kind: "diagnostics_preview";
    }
  | {
      kind: "diagnostics_export";
    }
  | {
      kind: "current_detail";
    }
  | {
      kind: "hide_overlay";
    }
  | {
      kind: "library";
      query: LibraryQuery;
    }
  | {
      id: string;
      kind: "detail";
    }
  | {
      id: string;
      kind: "favorite";
      value: boolean;
    }
  | {
      action: ReaderAction;
      kind: "reader";
    }
  | {
      kind: "appearance";
      value: Appearance;
    }
  | {
      kind: "reset_appearance";
    }
  | {
      kind: "search";
      query: SearchQuery;
    }
  | {
      kind: "search_poll";
      token: string;
    }
  | {
      context: string;
      kind: "search_cancel";
    }
  | {
      kind: "save_search";
      token: string;
      versionId: string;
    }
  | {
      kind: "clear_downloads";
    }
  | {
      kind: "clear_history";
    }
  | {
      kind: "overlay";
      mode: OverlayMode;
    }
  | {
      enabled: boolean;
      kind: "click_through";
    }
  | {
      kind: "dock";
      side: string;
    }
  | {
      kind: "reset_overlay";
    }
  | {
      kind: "open_releases";
    };
export type LibrarySort = "title" | "artist" | "recent" | "most_played";
export type ReaderAction =
  | {
      kind: "select";
      versionId: string;
    }
  | {
      delayMs: number;
      estimated: boolean;
      kind: "adjust";
      versionId: string;
    }
  | {
      kind: "align";
      line: number;
      versionId: string;
    };
export type TextAlignment = "left" | "center" | "right";
export type OverlayMode = "compact" | "expanded" | "minimal";
export type ProductReply =
  | {
      kind: "backup";
      state: BackupProgress;
    }
  | {
      kind: "diagnostics";
      report: string;
    }
  | {
      kind: "done";
    }
  | {
      items: LibraryItem[];
      kind: "library";
      more: boolean;
    }
  | {
      detail: LibraryDetail;
      kind: "detail";
    }
  | {
      kind: "search";
      message: string;
      status: LookupStatus;
      token: string;
      versions: LyricVersion[];
    };
export type BackupStage = "idle" | "busy" | "preview" | "done" | "failed" | "cancelling";
export type LookupStatus =
  | "idle"
  | "waiting"
  | "loading"
  | "ready"
  | "offline"
  | "not_found"
  | "instrumental"
  | "insufficient_metadata"
  | "failed";
export type RuntimeMode = "native" | "fixture";
export type PlaybackStatus = "playing" | "paused" | "buffering" | "stopped" | "unknown";

/**
 * Export root keeps every boundary type reachable in a single versioned schema.
 */
export interface Contract {
  action: RuntimeAction;
  failure: CommandFailure;
  lyric_version: LyricVersion;
  product_command: ProductCommand;
  product_reply: ProductReply;
  snapshot: RuntimeSnapshot;
}
export interface PlayerId {
  nativeId: string;
  platform: Platform;
}
export interface CommandFailure {
  code: FailureCode;
  message: string;
  retryAfterMs?: Counter | null;
}
export interface LyricVersion {
  attribution?: string | null;
  confidence: MatchConfidence;
  contentHash: string;
  id: string;
  lines: LyricLine[];
  origin: LyricOrigin;
  providerDurationMs?: Counter | null;
  providerId?: string | null;
  recordingId: string;
  retention: string;
  sourceName?: string | null;
  sourceUrl?: string | null;
  timing: TimingKind;
}
export interface LyricLine {
  endMs?: Counter | null;
  id: string;
  startMs?: Counter | null;
  text: string;
  words: LyricWord[];
}
export interface LyricWord {
  endMs?: Counter | null;
  startMs: Counter;
  text: string;
}
export interface BackupLimits {
  maxBytes?: Counter | null;
  maxNodes: Counter;
  maxStringBytes: Counter;
}
export interface LibraryQuery {
  favoritesOnly: boolean;
  limit: number;
  offset: number;
  sort: LibrarySort;
  text: string;
}
export interface Appearance {
  floating: ReaderAppearance;
  main: ReaderAppearance;
  opacity: number;
  reducedMotion: boolean;
}
export interface ReaderAppearance {
  alignment: TextAlignment;
  textSize: number;
  words: boolean;
}
export interface SearchQuery {
  album?: string | null;
  artist: string;
  context: string;
  target?: string | null;
  title: string;
}
export interface BackupProgress {
  message: string;
  stage: BackupStage;
  summary: string[];
  token?: string | null;
}
export interface LibraryItem {
  favorite: boolean;
  hasLyrics: boolean;
  lastPlayedMs?: Counter | null;
  playCount: Counter;
  recording: Recording;
}
export interface Recording {
  album?: string | null;
  artists: string[];
  durationMs?: Counter | null;
  fileName?: string | null;
  id: string;
  title: string;
  versionLabel?: string | null;
}
export interface LibraryDetail {
  adjustments: VersionAdjustment[];
  item: LibraryItem;
  selected?: string | null;
  versions: LyricVersion[];
}
export interface VersionAdjustment {
  delayMs: number;
  estimated: boolean;
  versionId: string;
}
export interface RuntimeSnapshot {
  appearance: Appearance;
  autoOverlay: boolean;
  capabilities: Capabilities;
  clickThrough: boolean;
  closeNotice: boolean;
  generation: Counter;
  offline: boolean;
  overlayCollapsed: boolean;
  overlayDocked: boolean;
  overlayMode: OverlayMode;
  overlayVisible: boolean;
  players: PlayerInfo[];
  reader: ReaderState;
  revision: Counter;
  schema: number;
  selectedSessionId?: string | null;
  selection: SelectionMode;
  sessions: PlaybackSnapshot[];
  startAtSignIn: boolean;
  statusMessage: string;
  trackingEnabled: boolean;
}
export interface Capabilities {
  libraryAvailable: boolean;
  mode: RuntimeMode;
  platform: Platform;
  reason: string;
  trackingAvailable: boolean;
}
export interface PlayerInfo {
  available: boolean;
  enabled: boolean;
  id: PlayerId;
  name: string;
}
export interface ReaderState {
  activeLine?: number | null;
  cached: boolean;
  countdownSeconds?: Counter | null;
  delayMs: number;
  estimated: boolean;
  estimatedAvailable: boolean;
  message: string;
  positionAvailable: boolean;
  retryAfterMs?: Counter | null;
  status: LookupStatus;
  version?: LyricVersion | null;
  versions: VersionSummary[];
  wordEmphasis: number[];
}
export interface VersionSummary {
  confidence: MatchConfidence;
  id: string;
  origin: LyricOrigin;
  source: string;
  timing: TimingKind;
}
export interface PlaybackSnapshot {
  observedAtMs: Counter;
  playerId: PlayerId;
  playerName: string;
  positionMs?: Counter | null;
  rate?: number | null;
  recording?: Recording | null;
  sessionId: string;
  status: PlaybackStatus;
  streamPinAvailable: boolean;
}
