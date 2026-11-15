/* Generated from Rust. Run npm run contract:generate; do not edit. */

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
export type Platform = "windows" | "linux" | "macos";
export type PlaybackStatus = "playing" | "paused" | "buffering" | "stopped" | "unknown";

/**
 * Export root keeps every boundary type reachable in a single versioned schema.
 */
export interface Contract {
  failure: CommandFailure;
  lyric_version: LyricVersion;
  snapshot: RuntimeSnapshot;
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
  capabilities: Capabilities;
  generation: Counter;
  revision: Counter;
  schema: number;
  sessions: PlaybackSnapshot[];
  trackingEnabled: boolean;
}
export interface Capabilities {
  libraryAvailable: boolean;
  mode: RuntimeMode;
  platform: Platform;
  reason: string;
  trackingAvailable: boolean;
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
}
export interface PlayerId {
  nativeId: string;
  platform: Platform;
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
