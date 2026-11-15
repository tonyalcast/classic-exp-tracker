import type { UnlistenFn } from "@tauri-apps/api/event";
import { call, on } from "./backend";
import type { RateWindow, ReaderStatus, Settings, Snapshot } from "./types";

const LEVEL_UP_MS = 2600;

export const DEFAULT_HOTKEYS = {
  toggle: "shift+control+F9",
  stop: "shift+control+F10",
  hud: "shift+control+F11",
  lock: "shift+control+F12",
};

export const DEFAULT_SETTINGS: Settings = {
  hudScale: 1,
  hudOpacity: 0.9,
  rateWindow: "5m",
  idlePauseMins: 3,
  hudChart: true,
  hotkeys: DEFAULT_HOTKEYS,
};

export const WINDOWS: { id: RateWindow; label: string }[] = [
  { id: "1m", label: "1 min" },
  { id: "5m", label: "5 min" },
  { id: "15m", label: "15 min" },
  { id: "session", label: "Session" },
];

// Rate in EXP per second for the chosen window.
export function rateFor(snap: Snapshot | null, w: RateWindow): number | null {
  if (!snap) return null;
  return { "1m": snap.rate1m, "5m": snap.rate5m, "15m": snap.rate15m, session: snap.rateSession }[w];
}

// Reactive view of backend state, fed by push events only.
export class Live {
  snap = $state<Snapshot | null>(null);
  hudLocked = $state(false);
  settings = $state<Settings>(DEFAULT_SETTINGS);
  reader = $state<ReaderStatus | null>(null);
  // True for a moment after the level goes up during a session.
  levelUp = $state(false);
  // Headline rate and time to level for the chosen window.
  rate = $derived(rateFor(this.snap, this.settings.rateWindow) ?? this.snap?.rateSession ?? null);
  secsToLevel = $derived(
    this.snap?.expToLevel != null && this.rate ? this.snap.expToLevel / this.rate : null,
  );
  #off: UnlistenFn[] = [];
  #levelTimer: ReturnType<typeof setTimeout> | undefined;

  async connect() {
    this.#off.push(await on<Snapshot>("snapshot", (p) => this.#setSnap(p)));
    this.#off.push(await on<boolean>("hud-locked", (p) => (this.hudLocked = p)));
    this.#off.push(await on<Settings>("settings", (p) => (this.settings = p)));
    this.#off.push(await on<ReaderStatus>("reader-status", (p) => (this.reader = p)));
    this.snap = await call<Snapshot>("get_snapshot");
    this.hudLocked = await call<boolean>("hud_is_locked");
    this.settings = (await call<Settings>("get_settings")) ?? DEFAULT_SETTINGS;
    this.reader = await call<ReaderStatus | null>("reader_status");
  }

  disconnect() {
    this.#off.forEach((off) => off());
    this.#off = [];
    clearTimeout(this.#levelTimer);
  }

  update(patch: Partial<Settings>) {
    return call<Settings>("update_settings", { patch }).catch(console.error);
  }

  #setSnap(next: Snapshot) {
    const before = this.snap?.level;
    if (before != null && next.level != null && next.level > before && next.state !== "idle") {
      this.levelUp = true;
      clearTimeout(this.#levelTimer);
      this.#levelTimer = setTimeout(() => (this.levelUp = false), LEVEL_UP_MS);
    }
    this.snap = next;
  }
}
