<script lang="ts">
  // Area chart of EXP rate samples in a dark slot; the dashed gold line is the best-session rate.
  let { values, best = null, height = 32 }: { values: number[]; best?: number | null; height?: number } = $props();

  const W = 100;
  const uid = `spark-${Math.random().toString(36).slice(2, 8)}`;
  const top = $derived(Math.max(...values, best ?? 0) * 1.15 || 1);
  const y = (v: number) => height - (v / top) * (height - 3) - 1.5;
  const pts = $derived(
    values.map((v, i) => `${((i / Math.max(1, values.length - 1)) * W).toFixed(2)},${y(v).toFixed(2)}`).join(" "),
  );
</script>

<div class="slot-chart" style:height="{height}px">
  {#if values.length >= 2}
    <svg viewBox="0 0 {W} {height}" preserveAspectRatio="none" role="img" aria-label="EXP per minute over time">
      <defs>
        <linearGradient id={uid} x1="0" x2="0" y1="0" y2="1">
          <stop offset="0" stop-color="var(--accent)" stop-opacity="0.32" />
          <stop offset="1" stop-color="var(--accent)" stop-opacity="0" />
        </linearGradient>
      </defs>
      {#if best}
        <line class="best" x1="0" x2={W} y1={y(best)} y2={y(best)} />
      {/if}
      <polygon points="0,{height} {pts} {W},{height}" fill="url(#{uid})" />
      <polyline points={pts} />
    </svg>
  {:else}
    <span class="empty">Chart starts after 20 s of session time</span>
  {/if}
</div>

<style>
  .slot-chart {
    position: relative;
    border-radius: 3px;
    background: var(--bg-0);
    box-shadow: inset 0 0 0 1px rgba(177, 193, 205, 0.14);
    overflow: hidden;
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
  }
  polyline {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.8;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }
  .best {
    stroke: var(--chat);
    stroke-width: 1;
    stroke-dasharray: 4 4;
    vector-effect: non-scaling-stroke;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 10px;
    font-weight: 700;
    color: var(--muted);
    white-space: nowrap;
  }
</style>
