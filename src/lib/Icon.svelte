<script lang="ts" module>
  export type IconName = "rate" | "session" | "hud" | "capture" | "history" | "keys" | "chart" | "dev" | "leaf";
</script>

<script lang="ts">
  // Stroke icons used in quick-slot squares and small buttons.
  let { name, size = 18, color = "currentColor" }: { name: IconName; size?: number; color?: string } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke={color}
  stroke-width="2.2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "rate"}
    <path d="M4 17l5-6 4 3 7-8" /><path d="M15 6h5v5" />
  {:else if name === "session"}
    <circle cx="12" cy="13" r="8" /><path d="M12 9v4l2.5 2M9 2h6" />
  {:else if name === "hud"}
    <rect x="3" y="5" width="18" height="12" rx="2" /><path d="M8 21h8M12 17v4" />
  {:else if name === "capture"}
    <circle cx="12" cy="12" r="8" /><circle cx="12" cy="12" r="3" /><path d="M12 1v4M12 19v4M1 12h4M19 12h4" />
  {:else if name === "history"}
    <path d="M7 4h10v5a5 5 0 01-10 0zM7 6H4a3 3 0 003 4M17 6h3a3 3 0 01-3 4M12 14v4M8 21h8" />
  {:else if name === "keys"}
    <rect x="2" y="6" width="20" height="12" rx="2" /><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10" />
  {:else if name === "chart"}
    <path d="M3 3v18h18" /><path d="M7 15l4-5 3 3 5-7" />
  {:else if name === "dev"}
    <path d="M8 6l-6 6 6 6M16 6l6 6-6 6" />
  {:else if name === "leaf"}
    <path
      fill={color}
      stroke="#7a2c00"
      stroke-width="1"
      d="M12 2l1.6 3.4 2.6-1.2-.6 4 3.4-.8-1 2.6 2.8 1.4-3.8 2.6.6 1.8-4.2-.6L12 22l-1.4-6.8-4.2.6.6-1.8L3.2 11.4 6 10l-1-2.6 3.4.8-.6-4 2.6 1.2z"
    />
  {/if}
</svg>
