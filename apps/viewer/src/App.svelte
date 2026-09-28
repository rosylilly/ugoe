<script lang="ts">
import { onMount } from "svelte";
import { readBrowserFile, viewerHost } from "./lib/host";
import type { ModelFile } from "./lib/modelFile";
import ParameterPanel from "./lib/ParameterPanel.svelte";
import { Player } from "./lib/player.svelte";
import Viewport from "./lib/Viewport.svelte";

const player = new Player();
// 外から操作できるようにする (ヘッドレスのブラウザでの確認、ドライバの登録など)。
window.__UGOE_VIEWER__ = player;

let dropping = $state(false);

async function openWith(get: () => Promise<ModelFile | null>) {
  try {
    const file = await get();
    if (file !== null) await player.load(file);
  } catch (e) {
    player.error = e instanceof Error ? e.message : String(e);
  }
}

function openDialog() {
  void openWith(() => viewerHost().pickModelFile());
}

onMount(() => {
  void openWith(() => viewerHost().initialModelFile());
});

function onDragOver(e: DragEvent) {
  if (!e.dataTransfer?.types.includes("Files")) return;
  e.preventDefault();
  e.dataTransfer.dropEffect = "copy";
  dropping = true;
}

function onDrop(e: DragEvent) {
  e.preventDefault();
  dropping = false;
  const file = e.dataTransfer?.files[0];
  if (file) void openWith(() => readBrowserFile(file));
}

function onKeyDown(e: KeyboardEvent) {
  const target = e.target as HTMLElement | null;
  if (target?.closest("input, textarea, select")) return;
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "o") {
    e.preventDefault();
    openDialog();
  } else if (!e.ctrlKey && !e.metaKey && !e.altKey && e.key.toLowerCase() === "f") {
    player.fit();
  }
}
</script>

<svelte:window
  onkeydown={onKeyDown}
  ondragover={onDragOver}
  ondragleave={(e) => {
    if (e.relatedTarget === null) dropping = false;
  }}
  ondrop={onDrop}
/>

<div class="app" class:dropping>
  <header class="toolbar">
    <button type="button" onclick={openDialog} title="モデルを開く (Ctrl+O)">開く</button>
    <span class="name" title={player.modelName ?? ""}>{player.modelName ?? "モデルなし"}</span>
    <span class="spacer"></span>
    <div class="group" aria-label="表示">
      <button type="button" onclick={() => player.zoomBy(1 / 1.25)} title="縮小">−</button>
      <span class="zoom">{Math.round(player.zoom * 100)}%</span>
      <button type="button" onclick={() => player.zoomBy(1.25)} title="拡大">+</button>
      <button type="button" onclick={() => player.fit()} title="全体を表示 (F、ダブルクリック)"
        >Fit</button
      >
    </div>
    <label class="group" title="背景色">
      背景
      <input
        type="color"
        value={player.background}
        oninput={(e) => player.setBackground(e.currentTarget.value, player.transparent)}
      />
    </label>
    <label class="group" title="背景を透明にする (市松模様で表示)">
      <input
        type="checkbox"
        checked={player.transparent}
        onchange={(e) => player.setBackground(player.background, e.currentTarget.checked)}
      />
      透明
    </label>
  </header>

  <main>
    <Viewport {player} />
    <!-- 右のパネル。フェイストラッキング (T0703) の映像と値のパネルは、パラメータの上に足す。 -->
    <aside class="sidebar">
      <ParameterPanel {player} />
    </aside>
  </main>

  <footer class="status">
    {#if player.error}
      <span class="error">{player.error}</span>
    {:else if player.warnings.length > 0}
      <span class="warning" title={player.warnings.join("\n")}>
        警告 {player.warnings.length} 件: {player.warnings[0]}
      </span>
    {:else}
      <span>{player.status === "ready" ? "読み込みました" : ""}</span>
    {/if}
    <span class="spacer"></span>
    {#if player.backend}<span class="backend">{player.backend}</span>{/if}
  </footer>
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .app.dropping main {
    outline: 3px dashed var(--accent);
    outline-offset: -6px;
  }
  .toolbar,
  .status {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.4rem 0.75rem;
    background: var(--panel);
    border-bottom: 1px solid var(--border);
  }
  .status {
    border-top: 1px solid var(--border);
    border-bottom: none;
    font-size: 0.75rem;
    color: var(--muted);
    min-height: 1.5rem;
  }
  .spacer {
    flex: 1;
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 30ch;
  }
  .group {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.85rem;
  }
  .zoom {
    min-width: 4ch;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  main {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .sidebar {
    width: 320px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 0.75rem;
    background: var(--panel);
    border-left: 1px solid var(--border);
  }
  .error {
    color: var(--danger);
  }
  .warning {
    color: var(--warning);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
