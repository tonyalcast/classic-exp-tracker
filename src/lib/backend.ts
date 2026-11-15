import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { HotkeyAction, ReaderStatus, Settings, Snapshot, StoredSession } from "./types";

// False when the UI runs in a plain browser (design preview with fake data).
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return inTauri ? invoke<T>(cmd, args) : Promise.resolve(mock(cmd, args) as T);
}

export async function on<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  if (inTauri) return listen<T>(event, (e) => cb(e.payload));
  const handler = (e: Event) => cb((e as CustomEvent<T>).detail);
  window.addEventListener(`mock:${event}`, handler);
  return () => window.removeEventListener(`mock:${event}`, handler);
}

// Fake backend for the browser preview only.
const preview = {
  locked: false,
  settings: {
    hudScale: 1,
    hudOpacity: 0.9,
    rateWindow: "5m",
    idlePauseMins: 3,
    hudChart: true,
    hotkeys: {
      toggle: "shift+control+F9",
      stop: "shift+control+F10",
      hud: "shift+control+F11",
      lock: "shift+control+F12",
    },
  } as Settings,
  snap: {
    state: "running",
    autoPaused: false,
    character: "Tamberlyn",
    level: 36,
    exp: 54244,
    percent: 31.14,
    expGained: 38_520,
    levelsGained: 0,
    activeSecs: 1_342,
    rateSession: 28.7,
    rate1m: 35.2,
    rate5m: 31.9,
    rate15m: 29.4,
    expToLevel: 119_972,
    secsToLevel: 3_761,
    trend: Array.from({ length: 60 }, (_, i) => 30 + 6 * Math.sin(i / 5) + (i % 7) - 3),
    bestRate: 30.1,
    vsBestSecs: 96,
    recentGains: [
      { atMs: Date.now() - 2_000, exp: 32 },
      { atMs: Date.now() - 6_000, exp: 32 },
      { atMs: Date.now() - 11_000, exp: 45 },
    ],
  } as Snapshot,
  reader: {
    ok: true,
    message: "Reading EXP",
    expText: "54,244[31.14%]",
    readMs: 0.4,
    learned: 0,
    atMs: Date.now(),
  } as ReaderStatus,
  history: [
    { id: 2, startedAtMs: Date.now() - 86_400_000, endedAtMs: 0, activeSecs: 3_600, character: "Tamberlyn", startLevel: 35, endLevel: 36, expGained: 118_000, levelsGained: 1 },
    { id: 1, startedAtMs: Date.now() - 172_800_000, endedAtMs: 0, activeSecs: 2_400, character: "Tamberlyn", startLevel: 35, endLevel: 35, expGained: 61_500, levelsGained: 0 },
  ] as StoredSession[],
};

// Lets the preview fake pushes from the console, e.g. levels and auto-pause.
if (typeof window !== "undefined" && !inTauri) {
  (window as unknown as { previewSnap: (p: Partial<Snapshot>) => void }).previewSnap = (p) => {
    preview.snap = { ...preview.snap, ...p };
    emit("snapshot", preview.snap);
  };
}

// Same ordering as the backend: shift, control, alt, super, key.
function previewNormalize(accel: string): string {
  const parts = accel.split("+");
  const key = parts.pop()!;
  const order = ["Shift", "Control", "Alt", "Super"].filter((m) => parts.includes(m));
  return [...order.map((m) => m.toLowerCase()), key].join("+");
}

function emit(event: string, detail: unknown) {
  window.dispatchEvent(new CustomEvent(`mock:${event}`, { detail }));
}

function mock(cmd: string, args?: Record<string, unknown>): unknown {
  switch (cmd) {
    case "get_snapshot":
      return preview.snap;
    case "reader_status":
      return preview.reader;
    case "list_sessions":
      return preview.history;
    case "hud_is_locked":
      return preview.locked;
    case "get_settings":
      return preview.settings;
    case "update_settings":
      preview.settings = { ...preview.settings, ...(args?.patch as Partial<Settings>) };
      emit("settings", preview.settings);
      return preview.settings;
    case "hud_set_locked":
      preview.locked = Boolean(args?.locked);
      emit("hud-locked", preview.locked);
      return null;
    case "session_toggle": {
      const s = preview.snap.state;
      preview.snap = { ...preview.snap, autoPaused: false, state: s === "running" ? "paused" : "running" };
      emit("snapshot", preview.snap);
      return null;
    }
    case "session_stop":
      preview.snap = { ...preview.snap, state: "idle" };
      emit("snapshot", preview.snap);
      return null;
    case "hotkey_set": {
      const action = args?.action as HotkeyAction;
      const accel = (args?.accel as string | null) ?? null;
      if (accel && !/(Control|Alt|Super)\+|^(Shift\+)?F\d+$/.test(accel)) {
        return Promise.reject("Use Ctrl, Alt or Cmd/Win, or a function key, so the game keeps its keys");
      }
      const norm = accel ? previewNormalize(accel) : `shift+control+F${{ toggle: 9, stop: 10, hud: 11, lock: 12 }[action]}`;
      preview.settings = { ...preview.settings, hotkeys: { ...preview.settings.hotkeys, [action]: norm } };
      emit("settings", preview.settings);
      return null;
    }
    case "hotkeys_failed":
      return [];
    case "capture_permission":
      return true;
    case "is_debug_build":
      return false;
    default:
      return null;
  }
}
