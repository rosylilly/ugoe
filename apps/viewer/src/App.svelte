<script lang="ts">
import { FaceTracker, TrackingPanel } from "@ugoe/face-tracking";
import { formatNumber } from "@ugoe/i18n";
import { type ModelFile, Player, visibleSlots } from "@ugoe/player";
import {
  Button,
  type ButtonBaseProps,
  ColorPicker,
  IconButton,
  Menu,
  showToast,
  Toaster,
  Toggle,
  Toolbar,
  Tooltip,
  TooltipProvider,
} from "@ugoe/ui";
import { onMount } from "svelte";
import DressUpPanel from "./lib/DressUpPanel.svelte";
import {
  type DropPreview,
  type DroppedItem,
  describeDropPaths,
  describeDropTypes,
  openDroppedItems,
} from "./lib/drop";
import {
  readBrowserFile,
  readFileAt,
  readSampleFile,
  saveJsonFile,
  savePngFile,
  viewerHost,
} from "./lib/host";
import { t } from "./lib/i18n";
import LicenseDialog from "./lib/LicenseDialog.svelte";
import LightingPanel from "./lib/LightingPanel.svelte";
import MemoryDialog from "./lib/MemoryDialog.svelte";
import MotionPanel from "./lib/MotionPanel.svelte";
import ParameterPanel from "./lib/ParameterPanel.svelte";
import SettingsDialog from "./lib/SettingsDialog.svelte";
import { openSample, SAMPLES, type Sample } from "./lib/samples";
import { encodePng, screenshotFileName } from "./lib/screenshot";
import Viewport from "./lib/Viewport.svelte";
import { loadWasm } from "./lib/wasm/load";

const player = new Player({
  loadWasm,
  readFile: readFileAt,
  wasmMissingMessage: t("errors.wasmMissing"),
});
// 外から操作できるようにする (ヘッドレスのブラウザでの確認、ドライバの登録など)。
window.__UGOE_VIEWER__ = player;
const tracker = new FaceTracker(player, { saveJson: saveJsonFile });
window.__UGOE_TRACKER__ = tracker;

let dropping = $state(false);
/**
 * ドラッグ中のものが開けるかの見立て (Tauri はパスから。T0717。ブラウザは名前が分からないので数と種類から。T0718)。
 * 見立てられなければ `null`。
 */
let dropPreview = $state<DropPreview | null>(null);
/** ドロップで飛ばしたものの知らせ (次に開くまで出す。T0717)。 */
let dropNotice = $state<string | null>(null);
/** OS からのドロップをホストがパスで受け取るか (Tauri)。そのときは HTML のドロップを扱わない。 */
let nativeDrop = false;
let licenseOpen = $state(false);
let settingsOpen = $state(false);
let memoryDialog = $state<MemoryDialog | null>(null);
/** 画像を読み戻して保存している間 (ボタンを押せない)。 */
let capturing = $state(false);

let sampleEntries = $derived(
  SAMPLES.map((sample) => ({
    label: sample.label,
    onselect: () => void openSampleFromMenu(sample),
  })),
);

/**
 * 今の表示を PNG で保存する (T0420)。画像は WebPuppet.read_pixels() で取る (canvas の読み戻しではない。screenshot.ts)。
 * ダイアログを開く前に読み戻すので、写るのはボタンを押したときの姿。保存した場所は知らせ (Toast) で出す。
 */
async function saveScreenshot() {
  if (capturing) return;
  capturing = true;
  try {
    const shot = await player.capture();
    if (shot === null) return;
    const png = await encodePng(shot);
    const saved = await savePngFile(screenshotFileName(player.modelName, new Date()), png);
    if (saved !== null) showToast(t("screenshot.saved", { path: saved }), { tone: "success" });
  } catch (e) {
    player.error = t("screenshot.error", { message: e instanceof Error ? e.message : String(e) });
  } finally {
    capturing = false;
  }
}

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
  dropPreview = describeDropTypes([...e.dataTransfer.items]);
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
  dropPreview = null;
  const files = [...(e.dataTransfer?.files ?? [])];
  if (files.length > 0)
    void openDropped(files.map((file) => ({ name: file.name, read: () => readBrowserFile(file) })));
}

function onKeyDown(e: KeyboardEvent) {
  const target = e.target as HTMLElement | null;
  if (target?.closest("input, textarea, select, [role=combobox], [role=listbox]")) return;
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
    if (e.relatedTarget === null) {
      dropping = false;
      dropPreview = null;
    }
  }}
  ondrop={onDrop}
/>

<TooltipProvider>
  <div class="app">
    <header class="toolbar">
      <Tooltip text={t("toolbar.open.tooltip")} shortcut="Mod+O">
        {#snippet children(attrs)}
          <Button {...attrs as ButtonBaseProps} variant="primary" onclick={openDialog}
            >{t("toolbar.open.label")}</Button
          >
        {/snippet}
      </Tooltip>
      <Menu label={t("toolbar.samples.label")} entries={sampleEntries} />
      <Tooltip text={t("toolbar.remembered.tooltip")}>
        {#snippet children(attrs)}
          <Button {...attrs as ButtonBaseProps} onclick={() => memoryDialog?.open()}
            >{t("toolbar.remembered.label")}</Button
          >
        {/snippet}
      </Tooltip>
      <span class="name">{player.modelName ?? t("toolbar.noModel")}</span>
      <span class="spacer"></span>
      <Toolbar label={t("toolbar.view.aria")}>
        <IconButton label={t("toolbar.zoomOut.aria")} onclick={() => player.zoomBy(1 / 1.25)}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <path d="M3.5 8h9" stroke-linecap="round" />
          </svg>
        </IconButton>
        <span class="zoom">{formatNumber(player.zoom, { style: "percent", maximumFractionDigits: 0 })}</span>
        <IconButton label={t("toolbar.zoomIn.aria")} onclick={() => player.zoomBy(1.25)}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <path d="M3.5 8h9M8 3.5v9" stroke-linecap="round" />
          </svg>
        </IconButton>
        <IconButton label={t("toolbar.fit.aria")} shortcut="F" onclick={() => player.fit()}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <path
              d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </IconButton>
      </Toolbar>
      <ColorPicker
        label={t("toolbar.background.label")}
        value={player.background}
        onchange={(color) => player.setBackground(color, player.transparent)}
      />
      <Toggle
        label={t("toolbar.transparent.label")}
        checked={player.transparent}
        onchange={(on) => player.setBackground(player.background, on)}
      />
      <Tooltip text={t("toolbar.screenshot.tooltip")}>
        {#snippet children(attrs)}
          <Button
            {...attrs as ButtonBaseProps}
            onclick={() => void saveScreenshot()}
            disabled={player.status !== "ready" || capturing}>{t("toolbar.screenshot.label")}</Button
          >
        {/snippet}
      </Tooltip>
      <IconButton label={t("toolbar.settings.aria")} onclick={() => (settingsOpen = true)}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
          <circle cx="8" cy="8" r="2.2" />
          <path
            d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"
            stroke-linecap="round"
          />
        </svg>
      </IconButton>
    </header>

    <main>
      <Viewport {player} />
      {#if dropping}
        <!--
          落とすときの枠。ビューポートの canvas の上にも見えるように、main の上に重ねる (T0717)。
          開けるかを見立て (Tauri はパスから、ブラウザは種類と数から。T0718)、枠の色 (一部を飛ばす・開けるものが無い) と文で示す。
        -->
        <div class="drop-frame" data-kind={dropPreview?.kind ?? "all"}>
          {#if dropPreview}
            <div class="drop-preview" role="status">{dropPreview.message}</div>
          {/if}
        </div>
      {/if}
      <!--
        右のパネル。縦にパネルを積む: フェイストラッキング → モデルの機能ごとのパネル → パラメータ (残りの高さ)。
        モデルの機能ごとのパネル (着せ替え・ライティング・モーションと表情) は、そのモデルが使うときだけ出す
        (着せ替えはスロットがあるか、モジュールを読み込んだとき。モーションと表情はバンドルにどちらかがあるとき)。
        畳めるまとまり (@ugoe/ui の Section) にして、高さの上限 (max-height) を付ける。
      -->
      <aside class="sidebar">
        <TrackingPanel {tracker} parameterIds={player.trackingParameterIds} values={player.values} />
        {#if visibleSlots(player.slots, player.publishedSlots).length > 0 || player.modules.length > 0 || player.dressUpError || player.restoreOffer || player.dressUpRemembered}
          <DressUpPanel {player} hasModel={player.status === "ready"} />
        {/if}
        {#if player.usesLighting}
          <LightingPanel {player} onshowmemory={() => memoryDialog?.open()} />
        {/if}
        {#if player.motions.length > 0 || player.expressions.length > 0}
          <MotionPanel {player} />
        {/if}
        <ParameterPanel {player} />
      </aside>
    </main>

    <footer class="status">
      {#if player.error}
        <span class="error ugoe-selectable" role="alert">{player.error}</span>
      {:else if player.warnings.length > 0}
        <span class="warning" title={player.warnings.join("\n")}>
          {t("status.warnings", { count: player.warnings.length, first: player.warnings[0] })}
        </span>
      {:else if !dropNotice}
        <span>{player.status === "ready" ? t("status.loaded") : ""}</span>
      {/if}
      {#if dropNotice}
        <span class="warning drop-notice" role="status" title={dropNotice}>{dropNotice}</span>
      {/if}
      <span class="spacer"></span>
      {#if player.backend}<span class="backend">{player.backend}</span>{/if}
      <Button variant="ghost" small onclick={() => (licenseOpen = true)}>{t("status.license")}</Button>
    </footer>
    <LicenseDialog bind:open={licenseOpen} />
    <SettingsDialog bind:open={settingsOpen} />
    <MemoryDialog {player} bind:this={memoryDialog} />
  </div>
  <Toaster />
</TooltipProvider>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .drop-frame {
    --frame: var(--ugoe-color-accent);
    position: absolute;
    inset: 3px;
    z-index: 5;
    border: 3px dashed var(--frame);
    pointer-events: none;
  }
  /* 一部を飛ばす・開けるものが無い (Tauri のドラッグ中にパスで見立てる、T0717)。 */
  .drop-frame[data-kind="some"] {
    --frame: var(--ugoe-color-warning);
  }
  .drop-frame[data-kind="none"] {
    --frame: var(--ugoe-color-danger);
  }
  .drop-preview {
    position: absolute;
    top: var(--ugoe-space-5);
    left: 50%;
    transform: translateX(-50%);
    max-width: 80%;
    padding: var(--ugoe-space-2) var(--ugoe-space-4);
    font-size: var(--ugoe-font-size-sm);
    color: var(--ugoe-color-text);
    background: var(--ugoe-color-surface);
    border: 2px solid var(--frame);
    border-radius: var(--ugoe-radius-lg);
  }
  .toolbar,
  .status {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-3);
    padding: var(--ugoe-space-2) var(--ugoe-space-4);
    background: var(--ugoe-color-surface);
    border-bottom: 1px solid var(--ugoe-color-border);
  }
  .toolbar {
    flex-wrap: wrap;
  }
  .status {
    border-top: 1px solid var(--ugoe-color-border);
    border-bottom: none;
    padding-block: var(--ugoe-space-1);
    font-size: var(--ugoe-font-size-xs);
    color: var(--ugoe-color-text-muted);
    min-height: calc(var(--ugoe-control-height) + 2 * var(--ugoe-space-1));
  }
  .spacer {
    flex: 1;
  }
  .name {
    font-weight: var(--ugoe-font-weight-strong);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 30ch;
  }
  .zoom {
    min-width: 5ch;
    text-align: center;
    font-size: var(--ugoe-font-size-sm);
    font-variant-numeric: tabular-nums;
  }
  main {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .sidebar {
    width: 340px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
    padding: var(--ugoe-space-3) var(--ugoe-space-4);
    background: var(--ugoe-color-surface);
    border-left: 1px solid var(--ugoe-color-border);
    min-height: 0;
    overflow: hidden;
  }
  /* 狭い幅 (docs/ui.md#そのほか の 360px) では、パネルをビューポートの下に置く。 */
  @media (max-width: 720px) {
    main {
      flex-direction: column;
    }
    .sidebar {
      width: auto;
      height: 55%;
      flex-shrink: 0;
      border-left: none;
      border-top: 1px solid var(--ugoe-color-border);
      overflow-y: auto;
    }
  }
  .error {
    color: var(--ugoe-color-danger);
  }
  .warning {
    color: var(--ugoe-color-warning);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
