<script lang="ts">
  import { call } from "$lib/backend";
  import Icon from "$lib/Icon.svelte";
  import type { Live } from "$lib/live.svelte";

  let { live }: { live: Live } = $props();
  // Content size: 100% is the original HUD, the minimum.
  const size = $derived(Math.round(live.settings.hudScale * 100));
  const opacity = $derived(Math.round(live.settings.hudOpacity * 100));

  const setSize = (p: number) => live.update({ hudScale: Math.min(200, Math.max(100, p)) / 100 });
  const setOpacity = (p: number) => live.update({ hudOpacity: Math.min(100, Math.max(30, p)) / 100 });
  const run = (cmd: string, args?: Record<string, unknown>) => call(cmd, args).catch(console.error);
</script>

{#snippet slider(id: string, label: string, value: number, min: number, max: number, step: number, set: (p: number) => void)}
  <div class="row">
    <label for={id}>{label}</label>
    <span class="chip num">{value}%</span>
  </div>
  <div class="slider">
    <button class="step" onclick={() => set(value - step)} disabled={value <= min} aria-label="Less">−</button>
    <input {id} type="range" {min} {max} {step} {value} oninput={(e) => set(Number(e.currentTarget.value))} />
    <button class="step" onclick={() => set(value + step)} disabled={value >= max} aria-label="More">+</button>
  </div>
{/snippet}

{@render slider("hud-size", "Content size", size, 100, 200, 10, setSize)}
{@render slider("hud-opacity", "Background opacity", opacity, 30, 100, 5, setOpacity)}
<div class="row">
  <span class="lbl">Rate chart</span>
  <button
    class="toggle"
    class:on={live.settings.hudChart}
    aria-pressed={live.settings.hudChart}
    onclick={() => live.update({ hudChart: !live.settings.hudChart })}
  >
    <Icon name="chart" size={12} color={live.settings.hudChart ? "var(--accent)" : "var(--muted)"} />
    {live.settings.hudChart ? "Shown" : "Hidden"}
  </button>
</div>
<div class="btns">
  <button onclick={() => run("hud_toggle")}>Show / hide</button>
  <button onclick={() => run("hud_set_locked", { locked: !live.hudLocked })}>
    {live.hudLocked ? "Unlock" : "Lock"} position
  </button>
</div>
<p class="hint">Unlocked, the HUD can be dragged and shows Start/Pause, Stop and the chart button on hover.</p>

<style>
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
  }
  label,
  .lbl {
    color: var(--muted);
    font-weight: 700;
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px;
    font-size: 11px;
    color: var(--muted);
  }
  .toggle.on {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .slider {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 8px;
    align-items: center;
  }
  .step {
    width: 28px;
    padding: 2px 0;
    font-size: 15px;
  }
  input[type="range"] {
    padding: 0;
    border: none;
    background: transparent;
    accent-color: var(--accent);
  }
  .btns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--muted);
  }
</style>
