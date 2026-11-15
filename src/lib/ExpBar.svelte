<script lang="ts">
  import * as f from "./format";

  // Lime EXP bar with the in-game "EXP. value [pct%]" caption.
  let {
    exp,
    percent,
    height = 14,
    caption = true,
  }: { exp: number | null | undefined; percent: number | null | undefined; height?: number; caption?: boolean } = $props();
</script>

<div class="bar" style:height="{height}px">
  <div class="fill" style:width="{Math.min(100, percent ?? 0)}%"></div>
  {#if caption}
    <div class="caption num">
      <b>EXP.</b>
      {f.int(exp)} <span class="br">[</span>{f.pct(percent)}<span class="br">]</span>
    </div>
  {/if}
</div>

<style>
  .bar {
    position: relative;
    background: linear-gradient(#4b525d, var(--bar-empty));
    border: 1px solid #0b0f16;
    border-radius: 3px;
    box-shadow: 0 0 0 1px var(--win-inner);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: linear-gradient(var(--exp-hi), var(--exp-lo));
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.45);
  }
  .caption {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 3px;
    font-size: 10px;
    font-weight: 700;
    color: #fff;
    text-shadow:
      0 1px 0 #000,
      1px 0 0 #000,
      -1px 0 0 #000,
      0 -1px 0 #000;
  }
  b {
    font-weight: 900;
  }
  .br {
    color: #9be03a;
  }
</style>
