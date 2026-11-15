<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { call, inTauri } from "$lib/backend";
  import { Live } from "$lib/live.svelte";
  import ExpBar from "$lib/ExpBar.svelte";
  import Icon from "$lib/Icon.svelte";
  import LevelBadge from "$lib/LevelBadge.svelte";
  import Sparkline from "$lib/Sparkline.svelte";
  import * as f from "$lib/format";

  const live = new Live();
  const snap = $derived(live.snap);
  const status = $derived(snap?.state ?? "idle");
  const dot = $derived(snap?.autoPaused ? "auto" : status);
  const vs = $derived(snap?.vsBestSecs ?? null);
  const chart = $derived(live.settings.hudChart);

  onMount(() => live.connect());
  onDestroy(() => live.disconnect());

  function drag(e: MouseEvent) {
    if (inTauri && e.button === 0 && !live.hudLocked) getCurrentWindow().startDragging();
  }
  const run = (cmd: string) => call(cmd).catch(console.error);
  const stop = (e: Event) => e.stopPropagation();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="hud"
  class:unlocked={!live.hudLocked}
  class:levelup={live.levelUp}
  class:chart
  style:zoom={live.settings.hudScale}
  style:--a={live.settings.hudOpacity}
  onmousedown={drag}
>
  <div class="head">
    <LevelBadge level={snap?.level} size={0.75} />
    <span class="name">{snap?.character ?? "—"}</span>
    {#if snap?.autoPaused}
      <span class="tag afk mini">AFK</span>
    {:else if vs != null}
      <span class="tag mini {vs < 0 ? 'behind' : 'ahead'}" title="Against your best session at this level">{f.vsBest(vs)}</span>
    {/if}
    {#if !live.hudLocked}
      <span class="ctl" onmousedown={stop}>
        <button class="ib" onclick={() => run("session_toggle")} aria-label={status === "running" ? "Pause" : "Start"}>
          <svg viewBox="0 0 10 10" aria-hidden="true">
            {#if status === "running"}
              <rect x="1.5" y="1" width="2.4" height="8" fill="#fff" /><rect x="6" y="1" width="2.4" height="8" fill="#fff" />
            {:else}
              <path d="M2.5 1.5 L8.5 5 L2.5 8.5 Z" fill="#fff" />
            {/if}
          </svg>
        </button>
        <button class="ib" onclick={() => run("session_stop")} disabled={status === "idle"} aria-label="Stop and save">
          <svg viewBox="0 0 10 10" aria-hidden="true"><rect x="1.5" y="1.5" width="7" height="7" fill="var(--hp)" /></svg>
        </button>
      </span>
    {/if}
    <button
      class="ib chart-btn"
      class:on={chart}
      onmousedown={stop}
      onclick={() => live.update({ hudChart: !chart })}
      aria-pressed={chart}
      aria-label={chart ? "Hide chart" : "Show chart"}
      title={chart ? "Hide chart" : "Show chart"}
    >
      <Icon name="chart" size={9} color={chart ? "var(--accent)" : "var(--muted)"} />
    </button>
    <span class="dot {dot}" title={snap?.autoPaused ? "auto-paused" : status}></span>
  </div>

  <div class="body" class:dim={snap?.autoPaused}>
    <div class="rates">
      <div><span class="dmg v">{f.perMin(live.rate)}</span><span class="caps u">/min</span></div>
      <div><span class="dmg v">{f.perHour(live.rate, true)}</span><span class="caps u">/h</span></div>
    </div>
    {#if chart}
      <Sparkline values={snap?.trend ?? []} best={snap?.bestRate} height={20} />
    {/if}
    <ExpBar exp={snap?.exp} percent={snap?.percent} height={11} caption={false} />
    {#if snap?.autoPaused}
      <div class="note">Paused after {live.settings.idlePauseMins} min without EXP · resumes on next gain</div>
    {:else if status === "running" && live.reader && !live.reader.ok}
      <div class="note">{live.reader.message}</div>
    {:else}
      <div class="stats">
        <b class="gain num">+{f.compact(snap?.expGained)}</b>
        <b class="num">{f.clock(snap?.activeSecs ?? 0)}</b>
        <b class="lvin num">{f.levelIn(snap?.level, live.secsToLevel)}</b>
      </div>
    {/if}
  </div>
  {#if live.levelUp}
    <div class="burst"><span class="dmg">LEVEL UP</span></div>
  {/if}
</div>

<style>
  .hud {
    --a: 0.9;
    position: relative;
    width: 242px;
    height: 97px;
    margin: 4px;
    display: flex;
    flex-direction: column;
    background: linear-gradient(
      color-mix(in srgb, #182030 calc(var(--a) * 100%), transparent),
      color-mix(in srgb, #0a0e16 calc(var(--a) * 100%), transparent)
    );
    border: 1px solid var(--rim);
    border-radius: 6px;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.1),
      0 0 0 1px #000;
    overflow: hidden;
  }
  .hud.chart {
    height: 115px;
  }
  .hud.unlocked {
    cursor: move;
    border: 1px dashed var(--accent);
  }
  .hud.levelup {
    border-color: var(--chat);
    animation: glow 2.6s ease-out;
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    height: 21px;
    padding: 0 5px 0 6px;
    background: linear-gradient(var(--steel-a), var(--steel-b));
    border-bottom: 1px solid #000;
  }
  .name {
    flex: 1;
    min-width: 0;
    font-weight: 900;
    font-size: 11px;
    color: #fff;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag.mini {
    font-size: 8.5px;
    padding: 0 4px;
    line-height: 12px;
    border-radius: 2px;
  }
  .tag.mini.ahead {
    color: var(--accent-ink);
    background: var(--accent);
    border: none;
  }
  .tag.mini.behind {
    color: #fff;
    background: var(--hp);
    border: none;
  }
  .ctl {
    display: none;
    gap: 3px;
    flex: none;
    cursor: default;
  }
  .hud.unlocked:hover .ctl {
    display: flex;
  }
  .hud.unlocked:hover .tag.mini {
    display: none;
  }
  .ib {
    flex: none;
    width: 15px;
    height: 13px;
    padding: 0;
    display: grid;
    place-items: center;
    border-radius: 2px;
    border: 1px solid #6b7682;
    background: linear-gradient(#3a4148, #262c33);
    box-shadow: none;
    cursor: default;
  }
  .ib svg {
    width: 7px;
    height: 7px;
  }
  .ib.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    margin-left: 1px;
    border-radius: 50%;
    background: var(--muted);
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.6);
  }
  .dot.running {
    background: var(--accent);
    box-shadow: 0 0 4px var(--accent);
  }
  .dot.paused,
  .dot.auto {
    background: var(--chat);
  }
  .dot.auto {
    animation: blink 1.6s ease-in-out infinite;
  }
  .body {
    flex: 1;
    min-height: 0;
    padding: 5px 7px 6px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }
  .body.dim > :not(.note) {
    filter: grayscale(1);
    opacity: 0.45;
  }
  .rates {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    align-items: baseline;
  }
  .rates > div {
    white-space: nowrap;
  }
  .v {
    font-size: 20px;
  }
  .u {
    font-size: 8.5px;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin-left: 3px;
  }
  .stats {
    display: flex;
    justify-content: space-between;
    font-size: 9.5px;
    font-weight: 800;
    white-space: nowrap;
  }
  .stats b {
    color: var(--text);
  }
  .stats .gain {
    color: var(--chat);
  }
  .stats .lvin {
    color: var(--lv-text);
  }
  .note {
    font-size: 9.5px;
    font-weight: 800;
    color: var(--chat);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .burst {
    position: absolute;
    inset: 21px 0 0;
    display: grid;
    place-items: center;
    background: repeating-conic-gradient(from 0deg at 50% 60%, rgba(255, 212, 0, 0.16) 0 8deg, transparent 8deg 22deg);
    pointer-events: none;
    animation: burst 2.6s ease-out forwards;
  }
  .burst .dmg {
    font-size: 25px;
    letter-spacing: 0.03em;
  }
  @keyframes burst {
    0% {
      opacity: 0;
      transform: scale(0.6);
    }
    12% {
      opacity: 1;
      transform: scale(1.12);
    }
    25% {
      transform: scale(1);
    }
    75% {
      opacity: 1;
    }
    100% {
      opacity: 0;
    }
  }
  @keyframes glow {
    0%,
    60% {
      box-shadow:
        inset 0 1px 0 rgba(255, 255, 255, 0.1),
        0 0 0 1px #000,
        0 0 12px 2px rgba(255, 212, 0, 0.6);
    }
  }
  @keyframes blink {
    50% {
      opacity: 0.35;
    }
  }
</style>
