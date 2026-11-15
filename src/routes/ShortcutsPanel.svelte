<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { call, on } from "$lib/backend";
  import { DEFAULT_HOTKEYS, type Live } from "$lib/live.svelte";
  import * as f from "$lib/format";
  import type { HotkeyAction } from "$lib/types";

  let { live }: { live: Live } = $props();

  const ACTIONS: { id: HotkeyAction; label: string }[] = [
    { id: "toggle", label: "Start / pause" },
    { id: "stop", label: "Stop & save" },
    { id: "hud", label: "Show / hide HUD" },
    { id: "lock", label: "Lock / unlock HUD" },
  ];
  const MODIFIER_CODES = /^(Shift|Control|Alt|Meta|OS)(Left|Right)?$/;

  let recording = $state<HotkeyAction | null>(null);
  let errors = $state<Partial<Record<HotkeyAction, string>>>({});
  let failed = $state<HotkeyAction[]>([]);
  let off: UnlistenFn | undefined;

  const isDefault = $derived(ACTIONS.every((a) => live.settings.hotkeys[a.id] === DEFAULT_HOTKEYS[a.id]));

  onMount(async () => {
    off = await on<HotkeyAction[]>("hotkeys-failed", (p) => (failed = p));
    failed = (await call<HotkeyAction[]>("hotkeys_failed")) ?? [];
  });
  onDestroy(() => {
    off?.();
    if (recording) stop();
  });

  // Global hotkeys are off while recording, so a bound combo reaches this window.
  async function record(action: HotkeyAction) {
    if (recording === action) return stop();
    if (recording) await stop();
    errors = { ...errors, [action]: undefined };
    await call("hotkeys_suspend", { suspend: true }).catch(console.error);
    recording = action;
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("blur", stop);
  }

  async function stop() {
    window.removeEventListener("keydown", onKey, true);
    window.removeEventListener("blur", stop);
    if (!recording) return;
    recording = null;
    await call("hotkeys_suspend", { suspend: false }).catch(console.error);
  }

  async function onKey(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    const action = recording;
    if (!action || e.repeat || MODIFIER_CODES.test(e.code)) return;
    if (e.code === "Escape" && !e.ctrlKey && !e.altKey && !e.metaKey) return stop();
    const mods = [e.ctrlKey && "Control", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
    const accel = [...mods, e.code].join("+");
    await stop();
    await set(action, accel);
  }

  async function set(action: HotkeyAction, accel: string | null) {
    try {
      await call("hotkey_set", { action, accel });
      errors = { ...errors, [action]: undefined };
    } catch (err) {
      errors = { ...errors, [action]: String(err) };
    }
  }

  async function resetAll() {
    errors = {};
    await call("hotkeys_reset").catch(console.error);
  }
</script>

<div class="list">
  {#each ACTIONS as a (a.id)}
    {@const accel = live.settings.hotkeys[a.id]}
    <div class="row">
      <span class="label">{a.label}</span>
      <button class="key" class:rec={recording === a.id} onclick={() => record(a.id)} title="Click, then press the new combo">
        {#if recording === a.id}
          Press keys… <span class="esc">Esc cancels</span>
        {:else}
          {#each f.combo(accel) as k, i (i)}<kbd>{k}</kbd>{/each}
        {/if}
      </button>
      <button
        class="undo"
        onclick={() => set(a.id, null)}
        disabled={accel === DEFAULT_HOTKEYS[a.id]}
        aria-label="Restore default"
        title="Restore default">↺</button
      >
    </div>
    {#if errors[a.id]}
      <p class="err">{errors[a.id]}</p>
    {:else if failed.includes(a.id)}
      <p class="err">Not active: taken by the system or another app.</p>
    {/if}
  {/each}
</div>
<div class="foot">
  <p class="hint">Click a combo, then press the new keys. Needs Ctrl, Alt or {f.combo("super+X")[0]}, or a function key.</p>
  <button onclick={resetAll} disabled={isDefault}>Reset all</button>
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: 6px;
  }
  .label {
    font-size: 12px;
    font-weight: 700;
    color: var(--muted);
  }
  .key {
    min-width: 132px;
    display: inline-flex;
    justify-content: center;
    align-items: center;
    gap: 3px;
    padding: 3px 8px;
    font-size: 11px;
  }
  .key.rec {
    color: var(--gold);
    border-color: var(--gold);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .esc {
    color: var(--muted);
    font-weight: 700;
    font-size: 10px;
  }
  kbd {
    font-family: var(--font);
    font-weight: 900;
    font-size: 11px;
    color: var(--title);
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--win-inner);
    border-radius: 3px;
    padding: 0 5px;
    min-width: 12px;
    text-align: center;
  }
  .undo {
    width: 26px;
    padding: 3px 0;
    font-size: 13px;
  }
  .err {
    margin: -2px 0 2px;
    font-size: 11px;
    font-weight: 700;
    color: var(--bad);
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-top: 8px;
  }
  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--muted);
  }
  .foot button {
    flex: none;
    padding: 4px 10px;
    font-size: 12px;
  }
  @keyframes pulse {
    50% {
      box-shadow: 0 0 0 2px rgba(255, 210, 122, 0.35);
    }
  }
</style>
