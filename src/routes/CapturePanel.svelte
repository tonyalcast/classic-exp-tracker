<script lang="ts">
  import { onMount } from "svelte";
  import { call } from "$lib/backend";
  import type { Detection, ReaderStatus } from "$lib/types";

  let { reader }: { reader: ReaderStatus | null } = $props();
  let permission = $state<boolean | null>(null);
  let detection = $state<Detection | null>(null);
  let busy = $state(false);
  let error = $state("");

  async function refresh() {
    permission = await call<boolean>("capture_permission");
  }

  async function request() {
    permission = await call<boolean>("capture_request_permission");
  }

  async function detect() {
    busy = true;
    error = "";
    try {
      detection = await call<Detection>("capture_detect");
      permission = detection.permission;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(refresh);
</script>

<div class="panel">
  <div class="row">
    <span class="lbl">Screen permission</span>
    <span class="status" class:ok={permission} class:bad={permission === false}>
      {permission == null ? "…" : permission ? "Granted" : "Not granted"}
    </span>
  </div>
  <div class="btns">
    <button onclick={request} disabled={permission === true}>Request permission</button>
    <button onclick={() => call("capture_open_settings")}>Open settings</button>
    <button onclick={detect} disabled={busy}>{busy ? "Detecting…" : "Detect game window"}</button>
    <button onclick={() => call("reader_identify")}>Re-read name &amp; level</button>
  </div>
  <div class="row">
    <span class="lbl">Reader</span>
    <span class="status" class:ok={reader?.ok} class:bad={reader && !reader.ok}>
      {reader ? reader.message : "Waiting for the game"}
    </span>
  </div>
  {#if reader}
    <dl>
      <dt>EXP text</dt><dd class="num">{reader.expText ?? "—"}</dd>
      <dt>Read time</dt><dd class="num">{reader.readMs?.toFixed(1) ?? "—"} ms</dd>
      <dt>Learned glyphs</dt><dd class="num">{reader.learned}</dd>
    </dl>
  {/if}
  {#if permission === false}
    <p class="hint">
      If no prompt appears, use Open settings and enable this app (in dev mode: the app that launched it), then restart it.
    </p>
  {/if}
  {#if error}
    <p class="err">{error}</p>
  {/if}
  {#if detection}
    {#if detection.window}
      <dl>
        <dt>App</dt><dd>{detection.window.appName || "—"}</dd>
        <dt>Title</dt><dd>{detection.window.title || "—"}</dd>
        <dt>Window</dt><dd class="num">{detection.window.width}×{detection.window.height} @ {detection.window.x},{detection.window.y}</dd>
        <dt>Capture</dt><dd class="num">{detection.imageWidth}×{detection.imageHeight} px · {detection.captureMs?.toFixed(1)} ms</dd>
        <dt>Saved</dt><dd class="path">{detection.savedPath}</dd>
        <dt>Status bar</dt><dd>{detection.statusBar ? `Found (${detection.statusBar})` : "Not found"}</dd>
      </dl>
      {#if detection.preview}
        <img src={detection.preview} alt="Game window capture" />
      {/if}
    {:else}
      <p class="err">
        Game window not found ({detection.visibleWindows} windows visible).
        {detection.permission ? "Is the game running and not minimized?" : "Screen permission is missing."}
      </p>
    {/if}
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
  }
  .lbl {
    color: var(--muted);
  }
  .status.ok {
    color: var(--good);
  }
  .status.bad {
    color: var(--warn);
  }
  .btns {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .btns button {
    flex: 1 1 auto;
    font-size: 12px;
  }
  .hint,
  .err {
    margin: 0;
    font-size: 11px;
    color: var(--muted);
  }
  .err {
    color: var(--warn);
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 3px 10px;
    font-size: 12px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .path {
    font-size: 10px;
    color: var(--muted);
    user-select: text;
  }
  img {
    width: 100%;
    border-radius: 6px;
    border: 1px solid var(--rim);
  }
</style>
