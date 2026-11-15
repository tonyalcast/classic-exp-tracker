<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { call, on } from "$lib/backend";
  import { Live, WINDOWS } from "$lib/live.svelte";
  import Accordion from "$lib/Accordion.svelte";
  import ExpBar from "$lib/ExpBar.svelte";
  import Icon from "$lib/Icon.svelte";
  import LevelBadge from "$lib/LevelBadge.svelte";
  import Sparkline from "$lib/Sparkline.svelte";
  import * as f from "$lib/format";
  import type { StoredSession } from "$lib/types";
  import CapturePanel from "./CapturePanel.svelte";
  import DevPanel from "./DevPanel.svelte";
  import HudPanel from "./HudPanel.svelte";
  import ShortcutsPanel from "./ShortcutsPanel.svelte";

  const CHART_KEY = "main:chart";
  const live = new Live();
  let history = $state<StoredSession[]>([]);
  let offHistory: UnlistenFn | undefined;
  let debugBuild = $state(false);
  let showChart = $state(readChartPref());

  const snap = $derived(live.snap);
  const status = $derived(snap?.state ?? "idle");
  const statusLabel = $derived(snap?.autoPaused ? "AFK" : status.toUpperCase());
  const vs = $derived(snap?.vsBestSecs ?? null);
  const idleOptions = [0, 2, 3, 5, 10];
  const rows = $derived([
    { id: "1m", label: "Last 1 min", v: snap?.rate1m },
    { id: "5m", label: "Last 5 min", v: snap?.rate5m },
    { id: "15m", label: "Last 15 min", v: snap?.rate15m },
    { id: "session", label: "Session", v: snap?.rateSession },
  ]);
  const toggleLabel = $derived(status === "idle" ? "Start" : status === "running" ? "Pause" : "Resume");
  const gains = $derived((snap?.recentGains ?? []).slice(0, 3));

  // Collapsed-section summaries.
  const hudSummary = $derived(
    `Size ${Math.round(live.settings.hudScale * 100)}% · Opacity ${Math.round(live.settings.hudOpacity * 100)}% · ` +
      `Chart ${live.settings.hudChart ? "on" : "off"} · ${live.hudLocked ? "Locked" : "Unlocked"}`,
  );
  const captureSummary = $derived(
    live.reader == null
      ? "Not checked yet"
      : live.reader.ok
        ? `${live.reader.message}${live.reader.readMs != null ? ` in ${live.reader.readMs.toFixed(1)} ms` : ""}`
        : live.reader.message,
  );
  const bestRate = $derived(
    history.reduce((m, h) => (h.activeSecs >= 300 ? Math.max(m, (h.expGained / h.activeSecs) * 3600) : m), 0),
  );
  const historySummary = $derived(
    history.length === 0
      ? "No saved sessions yet"
      : `${history.length} saved${bestRate > 0 ? ` · Best ${f.compact(bestRate)}/h` : ""}`,
  );
  const keysSummary = $derived(
    `Start/pause ${f.combo(live.settings.hotkeys.toggle).join("")} · Stop ${f.combo(live.settings.hotkeys.stop).join("")} · +2`,
  );

  function readChartPref(): boolean {
    try {
      return localStorage.getItem(CHART_KEY) !== "0";
    } catch {
      return true;
    }
  }

  function toggleChart() {
    showChart = !showChart;
    try {
      localStorage.setItem(CHART_KEY, showChart ? "1" : "0");
    } catch {
      // Not remembered without storage.
    }
  }

  async function loadHistory() {
    history = await call<StoredSession[]>("list_sessions", { limit: 10 });
  }

  onMount(async () => {
    await live.connect();
    offHistory = await on("history-changed", loadHistory);
    await loadHistory();
    debugBuild = await call<boolean>("is_debug_build");
  });
  onDestroy(() => {
    live.disconnect();
    offHistory?.();
  });

  const run = (cmd: string, args?: Record<string, unknown>) => call(cmd, args).catch(console.error);
  const time = (ms: number) => new Date(ms).toLocaleTimeString(undefined, { hour12: false });
</script>

<main>
  <section class="glass who">
    <span class="badge" class:levelup={live.levelUp}><LevelBadge level={snap?.level} size={1.2} /></span>
    <div class="id">
      <div class="name">{snap?.character ?? "Reading character…"}</div>
      <span class="state {status}" class:auto={snap?.autoPaused}>
        <span class="dot"></span>{statusLabel}
      </span>
    </div>
    {#if live.levelUp}
      <span class="dmg lvup">LEVEL UP</span>
    {:else}
      <div class="clock">
        <span class="k">Session</span>
        <b class="num">{f.clock(snap?.activeSecs ?? 0)}</b>
      </div>
    {/if}
  </section>

  <section class="glass rate">
    <div class="tabs" role="tablist" aria-label="Rate window">
      {#each WINDOWS as w (w.id)}
        <button
          role="tab"
          aria-selected={live.settings.rateWindow === w.id}
          class:on={live.settings.rateWindow === w.id}
          onclick={() => live.update({ rateWindow: w.id })}>{w.label}</button
        >
      {/each}
    </div>
    <div class="content">
      <div class="rates">
        <div>
          <div class="dmg value">{f.perMin(live.rate)}</div>
          <div class="caps unit">EXP / min</div>
        </div>
        <div>
          <div class="dmg value">{f.perHour(live.rate)}</div>
          <div class="caps unit">EXP / hour</div>
        </div>
      </div>

      <div class="pace">
        {#if vs != null}
          <span class="tag {vs < 0 ? 'behind' : 'ahead'}" title="Against your best saved session at this level">
            {f.vsBest(vs, true)}
            {vs < 0 ? "BEHIND" : "AHEAD OF"} YOUR BEST{snap?.level != null ? ` · LV ${snap.level}` : ""}
          </span>
        {:else if snap?.bestRate == null && status !== "idle"}
          <span class="k">No best yet at this level</span>
        {/if}
      </div>

      <div class="chart-wrap">
        {#if showChart}
          <Sparkline values={snap?.trend ?? []} best={snap?.bestRate} height={56} />
          {#if snap?.bestRate}
            <span class="caps best">Best {f.perHour(snap.bestRate, true)}/h</span>
          {/if}
        {/if}
        <button
          class="chart-btn"
          class:on={showChart}
          class:floating={showChart}
          onclick={toggleChart}
          aria-pressed={showChart}
          aria-label={showChart ? "Hide chart" : "Show chart"}
          title={showChart ? "Hide chart" : "Show chart"}
        >
          <Icon name="chart" size={12} color={showChart ? "var(--accent)" : "var(--muted)"} />
          {#if !showChart}<span>Show chart</span>{/if}
        </button>
      </div>

      <ExpBar exp={snap?.exp} percent={snap?.percent} height={18} />

      <div class="foot">
        <b class="lvin num">{f.levelIn(snap?.level, live.secsToLevel)}</b>
        <span class="k num">{f.int(snap?.expToLevel)} EXP to go</span>
      </div>

      <div class="feed">
        {#each gains as g, i (g.atMs + ":" + i)}
          <div style:opacity={1 - i * 0.2}><span>+{f.int(g.exp)} EXP</span><span class="t">{time(g.atMs)}</span></div>
        {:else}
          <div class="none">EXP gains show up here while a session runs.</div>
        {/each}
      </div>
    </div>
  </section>

  <section class="controls">
    <button class="primary big" onclick={() => run("session_toggle")}>
      {#if status === "running"}
        <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true"
          ><rect x="3" y="2" width="3.6" height="12" rx="1" fill="currentColor" /><rect
            x="9.4"
            y="2"
            width="3.6"
            height="12"
            rx="1"
            fill="currentColor"
          /></svg
        >
      {:else}
        <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true"><path d="M4 2l10 6-10 6z" fill="currentColor" /></svg>
      {/if}
      {toggleLabel.toUpperCase()}
    </button>
    <button class="big" disabled={status === "idle"} onclick={() => run("session_stop")}>
      <svg width="10" height="10" viewBox="0 0 12 12" aria-hidden="true"><rect x="1" y="1" width="10" height="10" rx="1.5" fill="var(--hp)" /></svg>
      STOP &amp; SAVE
    </button>
  </section>

  <Accordion id="session" title="Session" icon="session" tint="var(--lv-text)" open>
    <div class="grid">
      <div><span class="k">EXP gained</span><b class="num">{f.int(snap?.expGained)}</b></div>
      <div><span class="k">Session time</span><b class="num">{f.clock(snap?.activeSecs ?? 0)}</b></div>
      <div><span class="k">EXP to level</span><b class="num">{f.int(snap?.expToLevel)}</b></div>
      <div><span class="k">Levels gained</span><b class="num">{snap?.levelsGained ?? 0}</b></div>
    </div>
    <table>
      <thead>
        <tr><th>Window</th><th>EXP / min</th><th>EXP / hour</th></tr>
      </thead>
      <tbody>
        {#each rows as r (r.id)}
          <tr class:sel={live.settings.rateWindow === r.id}>
            <td>{r.label}</td>
            <td class="num">{f.perMin(r.v)}</td>
            <td class="num">{f.perHour(r.v, true)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <div class="setting">
      <span class="k">Auto-pause after no EXP for</span>
      <select
        value={live.settings.idlePauseMins}
        onchange={(e) => live.update({ idlePauseMins: Number(e.currentTarget.value) })}
        aria-label="Auto-pause delay"
      >
        {#each idleOptions as m (m)}
          <option value={m}>{m === 0 ? "Off" : `${m} min`}</option>
        {/each}
      </select>
    </div>
    <p class="hint">Idle time is removed from the session and the next EXP gain resumes it.</p>
  </Accordion>

  <Accordion id="hud" title="HUD" icon="hud" tint="#6fa8ff" summary={hudSummary}>
    <HudPanel {live} />
  </Accordion>

  <Accordion
    id="capture"
    title="Game capture"
    icon="capture"
    tint="var(--accent)"
    summary={captureSummary}
    summaryTint={live.reader && !live.reader.ok ? "var(--warn)" : "var(--accent)"}
  >
    <CapturePanel reader={live.reader} />
  </Accordion>

  <Accordion id="history" title="History" icon="history" tint="var(--chat)" summary={historySummary}>
    {#if history.length === 0}
      <p class="empty">No saved sessions yet.</p>
    {:else}
      <table>
        <thead>
          <tr><th>When</th><th>Time</th><th>Gained</th><th>EXP / h</th><th>Lv</th></tr>
        </thead>
        <tbody>
          {#each history as h (h.id)}
            <tr>
              <td>{f.date(h.startedAtMs)}</td>
              <td class="num">{f.clock(h.activeSecs)}</td>
              <td class="num">{f.compact(h.expGained)}</td>
              <td class="num">{f.compact(h.activeSecs > 0 ? (h.expGained / h.activeSecs) * 3600 : null)}</td>
              <td class="num">{h.startLevel ?? "—"}→{h.endLevel ?? "—"}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </Accordion>

  <Accordion id="shortcuts" title="Shortcuts" icon="keys" summary={keysSummary}>
    <ShortcutsPanel {live} />
  </Accordion>

  {#if debugBuild}
    <Accordion id="developer" title="Developer" icon="dev" summary="Fake readings, debug builds only">
      <DevPanel {snap} />
    </Accordion>
  {/if}
</main>

<style>
  main {
    min-height: 100vh;
    background:
      radial-gradient(120% 40% at 50% 0%, var(--night-glow) 0%, transparent 70%),
      linear-gradient(var(--bg-1), var(--bg-0));
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .k {
    color: var(--muted);
    font-size: 11px;
    font-weight: 700;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
  }
  .badge {
    border-radius: 6px;
  }
  .badge.levelup {
    animation: badge-glow 2.6s ease-out;
  }
  .id {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .name {
    font-weight: 900;
    font-size: 19px;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .state {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    font-weight: 900;
    letter-spacing: 0.1em;
    color: var(--muted);
  }
  .state .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }
  .state.running {
    color: var(--accent);
  }
  .state.running .dot {
    box-shadow: 0 0 6px var(--accent);
  }
  .state.paused {
    color: var(--chat);
  }
  .state.auto {
    color: var(--chat);
  }
  .state.auto .dot {
    animation: blink 1.6s ease-in-out infinite;
  }
  .clock {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }
  .clock b {
    font-size: 16px;
    font-weight: 800;
    color: #fff;
  }
  .lvup {
    font-size: 22px;
    animation: lvup 2.6s ease-out forwards;
  }
  .rate {
    overflow: hidden;
  }
  .tabs {
    display: flex;
    border-bottom: 1px solid #0b0f16;
  }
  .tabs button {
    flex: 1;
    height: 28px;
    padding: 0;
    border: none;
    border-right: 1px solid #0b0f16;
    border-radius: 0;
    box-shadow: none;
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--muted);
    background: linear-gradient(#1b222c, #11161e);
  }
  .tabs button:last-child {
    border-right: none;
  }
  .tabs button.on {
    color: #fff;
    background: linear-gradient(#4a535b, var(--steel-b));
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .content {
    padding: 14px 14px 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .rates {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    text-align: center;
  }
  .rates > div + div {
    border-left: 1px solid rgba(177, 193, 205, 0.18);
  }
  .value {
    font-size: 38px;
  }
  .unit {
    margin-top: 6px;
    font-size: 11px;
    letter-spacing: 0.14em;
    color: var(--muted);
  }
  .pace {
    display: flex;
    justify-content: center;
    min-height: 20px;
  }
  .chart-wrap {
    position: relative;
    display: flex;
    flex-direction: column;
  }
  .best {
    position: absolute;
    left: 5px;
    bottom: 4px;
    padding: 1px 4px;
    border-radius: 2px;
    font-size: 9px;
    color: var(--chat);
    background: rgba(7, 11, 19, 0.8);
  }
  .chart-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    align-self: flex-end;
    height: 20px;
    padding: 0 6px;
    font-size: 10px;
    color: var(--muted);
    border-color: #6b7682;
    background: linear-gradient(#3a4148, #262c33);
    box-shadow: none;
  }
  .chart-btn.floating {
    position: absolute;
    right: 4px;
    top: 4px;
    width: 22px;
    padding: 0;
  }
  .chart-btn.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .foot {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .lvin {
    font-size: 15px;
    font-weight: 900;
    color: var(--lv-text);
  }
  .feed {
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.45);
    padding: 6px 10px;
    font-size: 12px;
    font-weight: 700;
    color: var(--chat);
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 58px;
  }
  .feed > div {
    display: flex;
    justify-content: space-between;
  }
  .feed .t {
    color: #8d7a2a;
    font-variant-numeric: tabular-nums;
  }
  .feed .none {
    color: var(--muted);
    font-size: 11px;
    margin: auto 0;
  }
  .controls {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 8px;
  }
  .big {
    height: 46px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-size: 12px;
    letter-spacing: 0.08em;
    border-radius: 5px;
  }
  .primary.big {
    font-size: 16px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
  }
  .grid > div {
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(177, 193, 205, 0.15);
    border-radius: 4px;
    padding: 6px 10px;
    display: flex;
    flex-direction: column;
  }
  .grid b {
    font-size: 18px;
    font-weight: 900;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
    border: 1px solid #0b0f16;
  }
  th {
    color: #fff;
    font-weight: 900;
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    text-align: right;
    padding: 5px 8px;
    background: linear-gradient(#4a535b, var(--steel-b));
  }
  td {
    text-align: right;
    padding: 5px 8px;
    font-weight: 700;
    background: var(--row);
  }
  tbody tr:nth-child(even) td {
    background: rgba(0, 0, 0, 0.4);
  }
  tbody tr.sel td {
    color: var(--accent);
    background: var(--accent-soft);
  }
  th:first-child,
  td:first-child {
    text-align: left;
  }
  td:first-child {
    color: var(--muted);
  }
  tr.sel td:first-child {
    color: var(--accent);
  }
  .setting {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .hint {
    margin: -4px 0 0;
    font-size: 11px;
    color: var(--muted);
  }
  .empty {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  @keyframes lvup {
    0% {
      opacity: 0;
      transform: scale(0.6);
    }
    15% {
      opacity: 1;
      transform: scale(1.1);
    }
    30% {
      transform: scale(1);
    }
    80% {
      opacity: 1;
    }
    100% {
      opacity: 0;
    }
  }
  @keyframes badge-glow {
    0%,
    60% {
      box-shadow: 0 0 14px 4px rgba(255, 212, 0, 0.7);
    }
  }
  @keyframes blink {
    50% {
      opacity: 0.35;
    }
  }
</style>
