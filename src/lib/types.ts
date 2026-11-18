export type SessionState = "idle" | "running" | "paused";

// Mirrors exp_core::Snapshot; rates are EXP per second.
export interface Snapshot {
  state: SessionState;
  autoPaused: boolean;
  character: string | null;
  level: number | null;
  exp: number | null;
  percent: number | null;
  expGained: number;
  levelsGained: number;
  activeSecs: number;
  rateSession: number | null;
  rate1m: number | null;
  rate5m: number | null;
  rate15m: number | null;
  expToLevel: number | null;
  secsToLevel: number | null;
  // 1-minute rate sampled every 20 s of active time, oldest first.
  trend: number[];
  bestRate: number | null;
  // Positive when ahead of the best saved session's pace.
  vsBestSecs: number | null;
  // Newest first; added by the app layer, not exp_core.
  recentGains: Gain[];
}

export interface Gain {
  atMs: number;
  exp: number;
}

export type RateWindow = "1m" | "5m" | "15m" | "session";

export type HotkeyAction = "toggle" | "stop" | "hud" | "lock";

// Normalized accelerators, e.g. "shift+control+F9".
export type Hotkeys = Record<HotkeyAction, string>;

// Mirrors settings::Settings.
export interface Settings {
  hudScale: number;
  hudOpacity: number;
  rateWindow: RateWindow;
  idlePauseMins: number;
  hudChart: boolean;
  hotkeys: Hotkeys;
}

// Mirrors storage::StoredSession.
export interface StoredSession {
  id: number;
  startedAtMs: number;
  endedAtMs: number;
  activeSecs: number;
  character: string | null;
  startLevel: number | null;
  endLevel: number | null;
  expGained: number;
  levelsGained: number;
}

// Mirrors detect::WindowDto.
export interface GameWindowInfo {
  id: number;
  pid: number;
  appName: string;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

// Mirrors detect::Detection.
export interface Detection {
  permission: boolean;
  visibleWindows: number;
  window: GameWindowInfo | null;
  captureMs: number | null;
  imageWidth: number | null;
  imageHeight: number | null;
  preview: string | null;
  savedPath: string | null;
  statusBar: string | null;
}

// Mirrors reader::ReaderStatus.
export interface ReaderStatus {
  ok: boolean;
  message: string;
  expText: string | null;
  readMs: number | null;
  learned: number;
  atMs: number;
}
