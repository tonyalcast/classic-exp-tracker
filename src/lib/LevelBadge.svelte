<script lang="ts">
  // "LV." label with one orange tile per digit, like the game's status bar.
  let { level, size = 1 }: { level: number | null | undefined; size?: number } = $props();
  const digits = $derived(level == null ? ["?"] : String(level).split(""));
</script>

<span class="lv" style:--k={size}>
  <span class="label">LV.</span>
  {#each digits as d, i (i)}
    <span class="tile">{d}</span>
  {/each}
</span>

<style>
  .lv {
    display: inline-flex;
    align-items: center;
    gap: calc(2px * var(--k));
  }
  .label {
    font-weight: 900;
    font-style: italic;
    font-size: calc(15px * var(--k));
    color: #fff;
    margin-right: calc(4px * var(--k));
    text-shadow:
      0 1px 0 #000,
      1px 0 0 #000,
      -1px 0 0 #000,
      0 -1px 0 #000;
  }
  .tile {
    display: inline-grid;
    place-items: center;
    width: calc(15px * var(--k));
    height: calc(19px * var(--k));
    border-radius: calc(3px * var(--k));
    background: linear-gradient(var(--lv-hi), var(--lv) 50%, var(--lv-lo));
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.25);
    color: #fff;
    font-weight: 900;
    font-size: calc(14px * var(--k));
    line-height: 1;
    text-shadow: 0 1px 0 rgba(120, 40, 0, 0.9);
  }
</style>
