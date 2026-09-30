<script lang="ts">
import { FaceTracker, TrackingPanel } from "@ugoe/face-tracking";
import { onMount } from "svelte";
import DressUpPanel from "./lib/DressUpPanel.svelte";
import { readBrowserFile, readSampleFile, saveJsonFile, viewerHost } from "./lib/host";
import LicenseDialog from "./lib/LicenseDialog.svelte";
import LightingPanel from "./lib/LightingPanel.svelte";
import { detectModelKind, type ModelFile } from "./lib/modelFile";
import ParameterPanel from "./lib/ParameterPanel.svelte";
import { Player } from "./lib/player.svelte";
import { openSample, SAMPLES, type Sample } from "./lib/samples";
import Viewport from "./lib/Viewport.svelte";

const player = new Player();
// 外から操作できるようにする (ヘッドレスのブラウザでの確認、ドライバの登録など)。
window.__UGOE_VIEWER__ = player;
const tracker = new FaceTracker(player, { saveJson: saveJsonFile });
window.__UGOE_TRACKER__ = tracker;

let dropping = $state(false);
let sampleMenu = $state<HTMLDetailsElement | null>(null);
let licenseDialog = $state<LicenseDialog | null>(null);

async function openWith(get: () => Promise<ModelFile | null>) {
  try {
    const file = await get();
    if (file !== null) await player.load(file);
  } catch (e) {
    player.error = e instanceof Error ? e.message : String(e);
  }
}

/** 同梱の見本を開く (モデルと、着せ替えの見本ならモジュールも)。 */
async function openSampleFromMenu(sample: Sample) {
  if (sampleMenu) sampleMenu.open = false;
  try {
    await openSample(sample, player, readSampleFile);
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

/**
 * ドロップしたファイルを開く。モデルとモジュール (.ugoemod) を一緒に落としたら、モデルをモジュールと一緒に開く
 * (別のモデルなら前のモジュールは一覧から除かれ、落としたモジュールが付く)。
 */
async function openDropped(files: File[]) {
  try {
    const read = await Promise.all(files.map(readBrowserFile));
    const isModule = (f: ModelFile) => detectModelKind(f) === "module";
    const models = read.filter((f) => !isModule(f));
    const modules = read.filter(isModule);
    // モデルは最後の 1 つだけを開く。
    const model = models.at(-1);
    if (model !== undefined)
      await player.load(
        model,
        modules.map((file) => ({ file, attach: true })),
      );
    else for (const file of modules) await player.load(file);
  } catch (e) {
    player.error = e instanceof Error ? e.message : String(e);
  }
}

function onDrop(e: DragEvent) {
  e.preventDefault();
  dropping = false;
  const files = [...(e.dataTransfer?.files ?? [])];
  if (files.length > 0) void openDropped(files);
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
    <button
      type="button"
      onclick={openDialog}
      title="モデル (.ugoe・.json) か着せ替えのモジュール (.ugoemod) を開く (Ctrl+O)">開く</button
    >
    <details class="menu" bind:this={sampleMenu}>
      <summary title="同梱の見本を開く">サンプル</summary>
      <div class="menu-items">
        {#each SAMPLES as sample (sample.id)}
          <button type="button" onclick={() => void openSampleFromMenu(sample)}>
            <span class="menu-label">{sample.label}</span>
            <span class="menu-description">{sample.description}</span>
          </button>
        {/each}
      </div>
    </details>
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
    <!--
      右のパネル。縦にパネルを積む: フェイストラッキング → モデルの機能ごとのパネル → パラメータ (残りの高さ)。
      モデルの機能ごとのパネル (着せ替え・ライティング) は、そのモデルが使うときだけ出す
      (着せ替えはスロットがあるか、モジュールを読み込んだとき)。畳める <details> にして、
      高さの上限 (max-height) を付ける。
    -->
    <aside class="sidebar">
      <TrackingPanel {tracker} parameterIds={player.trackingParameterIds} values={player.values} />
      {#if player.slots.length > 0 || player.modules.length > 0 || player.dressUpError || player.restoreOffer || player.dressUpRemembered}
        <DressUpPanel {player} hasModel={player.status === "ready"} />
      {/if}
      {#if player.usesLighting}
        <LightingPanel {player} />
      {/if}
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
    <button type="button" class="link" onclick={() => licenseDialog?.open()}>ライセンス</button>
  </footer>
  <LicenseDialog bind:this={licenseDialog} />
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
  .menu {
    position: relative;
  }
  .menu summary {
    list-style: none;
    cursor: pointer;
    padding: 0.2rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.85rem;
  }
  .menu summary::-webkit-details-marker {
    display: none;
  }
  .menu summary::after {
    content: " ▾";
  }
  .menu-items {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 10;
    width: 22rem;
    display: flex;
    flex-direction: column;
    padding: 0.25rem;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.2);
  }
  .menu-items button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.1rem;
    text-align: left;
    border: none;
    background: none;
    padding: 0.4rem 0.5rem;
    border-radius: 4px;
  }
  .menu-items button:hover {
    background: var(--border);
  }
  .menu-label {
    font-weight: 600;
  }
  .menu-description {
    font-size: 0.75rem;
    color: var(--muted);
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
  .link {
    border: none;
    background: none;
    padding: 0;
    font-size: inherit;
    color: var(--muted);
    text-decoration: underline;
  }
  .warning {
    color: var(--warning);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
