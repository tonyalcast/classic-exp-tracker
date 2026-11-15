const DASH = "—";
const intFmt = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });
const compactFmt = new Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 2 });

export function int(n: number | null | undefined): string {
  return n == null ? DASH : intFmt.format(n);
}

export function compact(n: number | null | undefined): string {
  return n == null ? DASH : compactFmt.format(n);
}

// Backend rates are EXP per second.
export function perMin(perSec: number | null | undefined, short = false): string {
  return perSec == null ? DASH : short ? compact(perSec * 60) : int(perSec * 60);
}

export function perHour(perSec: number | null | undefined, short = false): string {
  return perSec == null ? DASH : short ? compact(perSec * 3600) : int(perSec * 3600);
}

export function clock(secs: number): string {
  const s = Math.floor(secs);
  const h = Math.floor(s / 3600);
  const mm = String(Math.floor((s % 3600) / 60)).padStart(2, "0");
  const ss = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}

export function eta(secs: number | null | undefined): string {
  if (secs == null || !isFinite(secs)) return DASH;
  const s = Math.round(secs);
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${String(m).padStart(2, "0")}m`;
  return `${m}m ${String(s % 60).padStart(2, "0")}s`;
}

export function pct(p: number | null | undefined): string {
  return p == null ? DASH : `${p.toFixed(2)}%`;
}

export function date(ms: number): string {
  return new Date(ms).toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

// Compact duration without seconds: "<1m", "11m", "1h 05m", "2d 3h".
export function span(secs: number | null | undefined): string {
  if (secs == null || !isFinite(secs)) return DASH;
  const s = Math.abs(Math.round(secs));
  if (s < 60) return "<1m";
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${String(m).padStart(2, "0")}m`;
  return `${m}m`;
}

// "Lv 36 in 11m" style time to the next level.
export function levelIn(level: number | null | undefined, secs: number | null | undefined): string {
  return level == null ? `Next level in ${span(secs)}` : `Lv ${level + 1} in ${span(secs)}`;
}

// Gap to the best session's pace, e.g. "▲ 2m 10s" or "▼ 45s".
export function vsBest(secs: number, long = false): string {
  const s = Math.round(Math.abs(secs));
  const m = Math.floor(s / 60);
  const text = m === 0 ? `${s}s` : long && m < 60 ? `${m}m ${String(s % 60).padStart(2, "0")}s` : span(s);
  return `${secs >= 0 ? "▲" : "▼"} ${text}`;
}

const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.platform || navigator.userAgent);

// Readable keys for a normalized accelerator, e.g. "shift+control+F9" → ["Ctrl", "⇧", "F9"].
export function combo(accel: string): string[] {
  const parts = accel.split("+");
  const key = parts.pop() ?? "";
  const mods = parts.map((m) => m.toLowerCase());
  const names: [string, string][] = [
    ["control", isMac ? "⌃" : "Ctrl"],
    ["alt", isMac ? "⌥" : "Alt"],
    ["shift", "⇧"],
    ["super", isMac ? "⌘" : "Win"],
  ];
  const out = names.filter(([m]) => mods.includes(m)).map(([, label]) => label);
  return [...out, keyName(key)];
}

function keyName(code: string): string {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^Numpad\d$/.test(code)) return `Num ${code.slice(6)}`;
  const named: Record<string, string> = {
    Backquote: "`", Minus: "-", Equal: "=", BracketLeft: "[", BracketRight: "]", Backslash: "\\",
    Semicolon: ";", Quote: "'", Comma: ",", Period: ".", Slash: "/", Space: "Space",
    ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→", PageUp: "PgUp", PageDown: "PgDn",
  };
  return named[code] ?? code;
}
