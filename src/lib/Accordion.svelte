<script lang="ts">
  import { untrack, type Snippet } from "svelte";
  import Icon, { type IconName } from "./Icon.svelte";

  // Collapsible glass panel with a quick-slot icon; open state is remembered per id.
  let {
    id,
    title,
    icon,
    tint = "var(--text)",
    summary = "",
    summaryTint = "var(--muted)",
    open = false,
    children,
  }: {
    id: string;
    title: string;
    icon: IconName;
    tint?: string;
    summary?: string;
    summaryTint?: string;
    open?: boolean;
    children: Snippet;
  } = $props();

  const key = untrack(() => `accordion:${id}`);
  let isOpen = $state(untrack(() => stored() ?? open));

  function stored(): boolean | null {
    try {
      const v = localStorage.getItem(key);
      return v === null ? null : v === "1";
    } catch {
      return null;
    }
  }

  $effect(() => {
    try {
      localStorage.setItem(key, isOpen ? "1" : "0");
    } catch {
      // Storage unavailable: the state just isn't remembered.
    }
  });
</script>

<details class="glass" bind:open={isOpen}>
  <summary>
    <span class="slot"><Icon name={icon} color={tint} /></span>
    <span class="head">
      <span class="caps title">{title}</span>
      {#if summary && !isOpen}<span class="sum" style:color={summaryTint}>{summary}</span>{/if}
    </span>
    <span class="sq" aria-hidden="true">{isOpen ? "−" : "+"}</span>
  </summary>
  <div class="body">{@render children()}</div>
</details>

<style>
  summary {
    list-style: none;
    cursor: pointer;
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  summary:hover .sq {
    filter: brightness(1.2);
  }
  .head {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .title {
    font-size: 12px;
    color: var(--title);
  }
  .sum {
    font-size: 11px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sq {
    flex: none;
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 3px;
    font-weight: 900;
    font-size: 14px;
    line-height: 1;
    color: #fff;
    background: linear-gradient(var(--steel-a), var(--steel-b));
    border: 1px solid var(--steel-edge);
  }
  .body {
    padding: 2px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
</style>
