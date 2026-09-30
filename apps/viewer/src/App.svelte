<script lang="ts">
import { FaceTracker, TrackingPanel } from "@ugoe/face-tracking";
import { onMount } from "svelte";
import DressUpPanel from "./lib/DressUpPanel.svelte";
import {
  type DropPreview,
  type DroppedItem,
  describeDropPaths,
  openDroppedItems,
} from "./lib/drop";
import { readBrowserFile, readFileAt, readSampleFile, saveJsonFile, viewerHost } from "./lib/host";
import LicenseDialog from "./lib/LicenseDialog.svelte";
import LightingPanel from "./lib/LightingPanel.svelte";
import MemoryDialog from "./lib/MemoryDialog.svelte";
import type { ModelFile } from "./lib/modelFile";
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
/** ドラッグ中のものが開けるかの見立て (Tauri はパスが分かる。ブラウザでは分からないので `null`。T0717)。 */
let dropPreview = $state<DropPreview | null>(null);
/** ドロップで飛ばしたものの知らせ (次に開くまで出す。T0717)。 */
let dropNotice = $state<string | null>(null);
/** OS からのドロップをホストがパスで受け取るか (Tauri)。そのときは HTML のドロップを扱わない。 */
let nativeDrop = false;
let sampleMenu = $state<HTMLDetailsElement | null>(null);
let licenseDialog = $state<LicenseDialog | null>(null);
let memoryDialog = $state<MemoryDialog | null>(null);

async function openWith(get: () => Promise<ModelFile | null>) {
  dropNotice = null;
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
  dropNotice = null;
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
  const host = viewerHost();
  if (host.listenFileDrop === undefined) return;
  nativeDrop = true;
  let unlisten: (() => void) | null = null;
  let disposed = false;
  void host
    .listenFileDrop((event) => {
      if (event.type === "hover") {
        dropping = event.active;
        dropPreview = event.active && event.paths ? describeDropPaths(event.paths) : null;
      } else {
        dropping = false;
        dropPreview = null;
        // パスから読むので `path:` の場所が付き、起動し直した後も覚えた組や光源を戻せる (T0713)。
        void openDropped(
          event.paths.map((path) => ({ name: path, read: () => readFileAt(`path:${path}`) })),
        );
      }
    })
    .then((f) => {
      if (disposed) f();
      else unlisten = f;
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
});

function onDragOver(e: DragEvent) {
  if (nativeDrop || !e.dataTransfer?.types.includes("Files")) return;
  e.preventDefault();
  e.dataTransfer.dropEffect = "copy";
  dropping = true;
}

/** ドロップしたものを読んで開く (`drop.ts`)。開けないもの・読めないものは飛ばして知らせる。 */
async function openDropped(items: DroppedItem[]) {
  dropNotice = null;
  try {
    dropNotice = await openDroppedItems(items, player);
  } catch (e) {
    player.error = e instanceof Error ? e.message : String(e);
  }
}

function onDrop(e: DragEvent) {
  if (nativeDrop) return;
  e.preventDefault();
  dropping = false;
  const files = [...(e.dataTransfer?.files ?? [])];
  if (files.length > 0)
    void openDropped(files.map((file) => ({ name: file.name, read: () => readBrowserFile(file) })));
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

<div class="app">
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
    <button
      type="button"
      onclick={() => memoryDialog?.open()}
      title="モデルごとに覚えた光源と着せ替えの組の一覧。モデルごとに、またはすべて忘れる"
      >覚えたもの</button
    >
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
    {#if dropping}
      <!--
        落とすときの枠。ビューポートの canvas の上にも見えるように、main の上に重ねる (T0717)。
        Tauri ではパスで開けるかを見立て、枠の色 (一部を飛ばす・開けるものが無い) と文で示す。
      -->
      <div class="drop-frame" data-kind={dropPreview?.kind ?? "all"}>
        {#if dropPreview}
          <div class="drop-preview" role="status">{dropPreview.message}</div>
        {/if}
      </div>
    {/if}
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
        <LightingPanel {player} onshowmemory={() => memoryDialog?.open()} />
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
    {:else if !dropNotice}
      <span>{player.status === "ready" ? "読み込みました" : ""}</span>
    {/if}
    {#if dropNotice}
      <span class="warning drop-notice" role="status" title={dropNotice}>{dropNotice}</span>
    {/if}
    <span class="spacer"></span>
    {#if player.backend}<span class="backend">{player.backend}</span>{/if}
    <button type="button" class="link" onclick={() => licenseDialog?.open()}>ライセンス</button>
  </footer>
  <LicenseDialog bind:this={licenseDialog} />
  <MemoryDialog {player} bind:this={memoryDialog} />
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .drop-frame {
    --frame: var(--accent);
    position: absolute;
    inset: 3px;
    z-index: 5;
    border: 3px dashed var(--frame);
    pointer-events: none;
  }
  /* 一部を飛ばす・開けるものが無い (Tauri のドラッグ中にパスで見立てる、T0717)。 */
  .drop-frame[data-kind="some"] {
    --frame: var(--warning);
  }
  .drop-frame[data-kind="none"] {
    --frame: var(--danger);
  }
  .drop-preview {
    position: absolute;
    top: 1rem;
    left: 50%;
    transform: translateX(-50%);
    max-width: 80%;
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
    color: var(--text);
    background: var(--panel);
    border: 2px solid var(--frame);
    border-radius: 6px;
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
    position: relative;
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
