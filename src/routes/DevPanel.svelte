<script lang="ts">
  import { call } from "$lib/backend";
  import type { Snapshot } from "$lib/types";

  // Debug-only stand-in for OCR readings.
  let { snap }: { snap: Snapshot | null } = $props();

  let level = $state(30);
  let required = $state(1_000_000);
  let exp = $state(0);
  let last = $state("");

  async function push() {
    const percent = Math.min(100, Math.floor((exp / required) * 10000) / 100);
    last = await call<string>("dev_push_reading", { reading: { level, exp, percent } }).catch(String);
  }

  async function kill() {
    exp += 2_000 + Math.floor(Math.random() * 3_000);
    if (exp >= required) {
      exp -= required;
      level += 1;
    }
    await push();
  }
</script>

<div class="panel">
  <p class="hint">Pushes fake readings to test the UI without the game.</p>
  <div class="row">
    <label>Lv <input type="number" bind:value={level} /></label>
    <label>EXP <input type="number" bind:value={exp} /></label>
    <label>Req <input type="number" bind:value={required} /></label>
  </div>
  <div class="row">
    <button onclick={push} disabled={snap?.state !== "running"}>Push reading</button>
    <button onclick={kill} disabled={snap?.state !== "running"}>Simulate kill</button>
  </div>
  {#if last}<code>{last}</code>{/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--muted);
  }
  .row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
    gap: 8px;
  }
  label {
    font-size: 11px;
    color: var(--muted);
  }
  code {
    font-size: 11px;
    color: var(--muted);
  }
</style>
