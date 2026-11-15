/* Generated from Rust. Run npm run contract:generate; do not edit. */

export type RuntimeAction =
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
export type RuntimeMode = "native" | "fixture";
export type PlaybackStatus = "playing" | "paused" | "buffering" | "stopped" | "unknown";

/**
 * Export root keeps every boundary type reachable in a single versioned schema.
 */
export interface Contract {
  action: RuntimeAction;
  failure: CommandFailure;
  lyric_version: LyricVersion;
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
export interface RuntimeSnapshot {
  autoOverlay: boolean;
  capabilities: Capabilities;
  closeNotice: boolean;
  generation: Counter;
  players: PlayerInfo[];
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
export interface Recording {
  album?: string | null;
  artists: string[];
  durationMs?: Counter | null;
  fileName?: string | null;
  id: string;
  title: string;
  versionLabel?: string | null;
}
