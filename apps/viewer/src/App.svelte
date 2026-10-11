<script lang="ts">
import {
  FaceTracker,
  loadStoredTuning,
  saveStoredTuning,
  TrackingPanel,
} from "@ugoe/face-tracking";
import { encodePng, type ModelFile, Player, visibleSlots } from "@ugoe/player";
import {
  announce,
  Button,
  type ButtonBaseProps,
  Icon,
  IconButton,
  Menu,
  showToast,
  Toaster,
  Tooltip,
  TooltipProvider,
} from "@ugoe/ui";
import { onMount, untrack } from "svelte";
import BackgroundPopover from "./lib/BackgroundPopover.svelte";
import DressUpPanel from "./lib/DressUpPanel.svelte";
import {
  type DropPreview,
  type DroppedItem,
  describeDropPaths,
  describeDropTypes,
  openDroppedItems,
} from "./lib/drop";
import { errorText } from "./lib/errors";
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
import { LoadedModelWatch } from "./lib/loadedModel";
import MemoryDialog from "./lib/MemoryDialog.svelte";
import MotionPanel from "./lib/MotionPanel.svelte";
import ParameterPanel from "./lib/ParameterPanel.svelte";
import PosePanel from "./lib/PosePanel.svelte";
import SettingsDialog from "./lib/SettingsDialog.svelte";
import { openSample, SAMPLES, type Sample } from "./lib/samples";
import { screenshotErrorText, screenshotFileName } from "./lib/screenshot";
import { STORAGE_KEYS } from "./lib/storageKeys";
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
// トラッキングの調整は覚えて、開き直しても戻す (T0738。既定値はキーを消す)。
tracker.setTuning(loadStoredTuning(STORAGE_KEYS["tracking.tuning"]));
$effect(() => {
  saveStoredTuning(STORAGE_KEYS["tracking.tuning"], tracker.tuning);
});

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
// モデルを開き終えたら読み上げる (足元の「読み込みました」は文が出るだけなので。docs/ui.md#状態の変化の知らせ)。
// 同じモデルを読み直したとき (着せ替え・作り直し) は読まない
const loadedWatch = new LoadedModelWatch();
$effect(() => {
  const name = player.status === "ready" ? player.modelName : null;
  const warnings = player.warnings.length;
  untrack(() => {
    if (!loadedWatch.changed(name) || name === null) return;
    announce(
      warnings > 0
        ? t("status.loadedAnnounceWarnings", { name, count: warnings })
        : t("status.loadedAnnounce", { name }),
    );
  });
});
let licenseOpen = $state(false);
let settingsOpen = $state(false);
let memoryDialog = $state<MemoryDialog | null>(null);
/** 画像を読み戻して保存している間 (ボタンを押せない)。 */
let capturing = $state(false);

let sampleEntries = $derived(
  SAMPLES.map((sample) => ({
    label: sample.label,
    description: sample.description,
    onselect: () => void openSampleFromMenu(sample),
  })),
);

/**
 * 今の表示を PNG で保存する (T0420)。画像は WebPuppet.read_pixels() で取る (canvas の読み戻しではない。PNG への変換は @ugoe/player の encodePng)。
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
    player.error = t("screenshot.error", { message: screenshotErrorText(e) });
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
    player.error = errorText(e);
  }
}

/** 同梱の見本を開く (モデルと、着せ替えの見本ならモジュールも)。 */
async function openSampleFromMenu(sample: Sample) {
  dropNotice = null;
  try {
    await openSample(sample, player, readSampleFile);
  } catch (e) {
    player.error = errorText(e);
  }
}

function openDialog() {
  void openWith(() => viewerHost().pickModelFile());
}

onMount(() => {
  const host = viewerHost();
  const initial = host.initialModelFile;
  if (initial !== undefined) void openWith(() => initial.call(host));
  const stops: (() => void)[] = [];
  let disposed = false;
  /** 受け取りを始め、やめる関数を覚える (先に画面が消えていたらすぐやめる)。 */
  const keep = (started: Promise<() => void>) =>
    void started.then(
      (stop) => {
        if (disposed) stop();
        else stops.push(stop);
      },
      (e) => {
        player.error = errorText(e);
      },
    );
  // OS から渡されたファイル (起動時の引数・「このアプリで開く」。T0732) は、ドロップと同じに開く。
  if (host.listenOpenedFiles !== undefined) keep(host.listenOpenedFiles(openPaths));
  if (host.listenFileDrop !== undefined) {
    nativeDrop = true;
    keep(
      host.listenFileDrop((event) => {
        if (event.type === "hover") {
          dropping = event.active;
          dropPreview = event.active && event.paths ? describeDropPaths(event.paths) : null;
        } else {
          dropping = false;
          dropPreview = null;
          openPaths(event.paths);
        }
      }),
    );
  }
  return () => {
    disposed = true;
    for (const stop of stops) stop();
  };
});

/**
 * パスのファイルを、ドロップと同じに開く (モデルを開き、`.ugoemod` は付ける)。パスから読むので `path:` の場所が付き、
 * 起動し直した後も覚えた組や光源を戻せる (T0713)。
 */
function openPaths(paths: readonly string[]) {
  void openDropped(paths.map((path) => ({ name: path, read: () => readFileAt(`path:${path}`) })));
}

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
    player.error = errorText(e);
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
      <!-- 画面の名前 (見出しの移動の起点。見た目にはツールバーがあるので隠す) -->
      <h1 class="ugoe-visually-hidden">{t("app.title")}</h1>
      <Tooltip text={t("toolbar.open.tooltip")} shortcut="Mod+O">
        {#snippet trigger(attrs)}
          <Button {...attrs as ButtonBaseProps} variant="primary" onclick={openDialog}
            >{t("toolbar.open.label")}</Button
          >
        {/snippet}
      </Tooltip>
      <Menu label={t("toolbar.samples.label")} entries={sampleEntries} />
      <Tooltip text={t("toolbar.remembered.tooltip")}>
        {#snippet trigger(attrs)}
          <Button {...attrs as ButtonBaseProps} onclick={() => memoryDialog?.open()}
            >{t("toolbar.remembered.label")}</Button
          >
        {/snippet}
      </Tooltip>
      <span class="name">{player.modelName ?? t("toolbar.noModel")}</span>
      <span class="spacer"></span>
      <BackgroundPopover
        color={player.background}
        transparent={player.transparent}
        onchange={(color, transparent) => player.setBackground(color, transparent)}
      />
      <Tooltip text={t("toolbar.screenshot.tooltip")}>
        {#snippet trigger(attrs)}
          <Button
            {...attrs as ButtonBaseProps}
            onclick={() => void saveScreenshot()}
            disabled={player.status !== "ready" || capturing}>{t("toolbar.screenshot.label")}</Button
          >
        {/snippet}
      </Tooltip>
      <IconButton label={t("toolbar.settings.aria")} onclick={() => (settingsOpen = true)}>
        <Icon name="settings" />
      </IconButton>
    </header>

    <main>
      <Viewport {player}>
        {#snippet empty()}
          <!-- 空の状態: 何が無いかと次の操作 (docs/ui.md#デザイン。T0725) -->
          <p class="empty-title">{t("viewport.emptyTitle")}</p>
          <p>{t("viewport.empty")}</p>
          <div class="empty-actions">
            <Button onclick={openDialog}>{t("viewport.open")}</Button>
            <Menu label={t("viewport.samples")} entries={sampleEntries} />
          </div>
        {/snippet}
      </Viewport>
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
        右のパネル。上の段に機能のパネル (フェイストラッキング → モデルの機能ごとのパネル) を、下の段にパラメータを置く。
        モデルの機能ごとのパネル (着せ替え・ライティング・モーションと表情) は、そのモデルが使うときだけ出す
        (着せ替えはスロットがあるか、モジュールを読み込んだとき。モーションと表情はバンドルにどちらかがあるとき。
        ポーズはグループがあるとき (付けたモジュールのグループも数える))。
        機能のパネルは畳めるまとまり (@ugoe/ui の Section) で、それぞれの中身の高さのまま縦に並べ、上の段ごと送る
        (パネルごとに高さの上限を付けると、トラッキングを開いたときにライティングなどが潰れた。T0725)。
        パラメータは残りの高さで、一覧の中を送る (見出しと絞り込みは送らずに残す)。狭い幅では右の列の全体を 1 つで送る。
      -->
      <aside class="sidebar">
        <div class="features">
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
          {#if player.poseGroups.length > 0}
            <PosePanel {player} />
          {/if}
        </div>
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
      {:else if !dropNotice && player.status === "ready"}
        <span class="loaded">{t("status.loaded")}</span>
        <!-- 狭い幅ではツールバーに名前を出さないので、ここに出す -->
        <span class="name">{player.modelName}</span>
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
  .toolbar .name {
    font-weight: var(--ugoe-font-weight-strong);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 30ch;
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
    background: var(--ugoe-color-surface);
    border-left: 1px solid var(--ugoe-color-border);
    min-height: 0;
    overflow: hidden;
  }
  /* 上の段 (機能のパネル)。中身が少なければその高さ、多ければ右の列の 6 割までにして中を送る。
     パネルは縮めない (各パネルは高さの上限と中の送りを持たず、この段がまとめて送る)。 */
  .features {
    flex: 0 1 auto;
    max-height: 60%;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
    /* 右はスクロールバーの分を空ける (docs/ui.md#レイアウト の「送り」) */
    padding: var(--ugoe-space-3) var(--ugoe-scrollbar-gap) var(--ugoe-space-3) var(--ugoe-space-4);
  }
  .features > :global(*) {
    flex: none;
  }
  /* 下の段 (パラメータ)。残りの高さで、最低でも右の列の 4 割を残す。 */
  .sidebar > :global(.parameters) {
    flex: 1 1 0;
    min-height: 40%;
    /* 右は中の一覧のスクロールバーの分 (一覧が右の端まで広がり、その padding に描かせる。ParameterPanel.svelte) */
    padding: var(--ugoe-space-3) var(--ugoe-scrollbar-gap) 0 var(--ugoe-space-4);
    border-top: 1px solid var(--ugoe-color-border);
  }
  /* 十分に広い幅では、機能のパネルとパラメータを横に 2 列に並べ、どちらも上から下まで使う。 */
  @media (min-width: 1200px) {
    .sidebar {
      flex-direction: row;
      width: 660px;
    }
    .features {
      flex: none;
      width: 330px;
      max-height: none;
      border-right: 1px solid var(--ugoe-color-border);
    }
    .sidebar > :global(.parameters) {
      min-height: 0;
      min-width: 0;
      border-top: none;
    }
  }
  /* 狭い幅 (docs/ui.md#そのほか の 360px) では、パネルをビューポートの下に置き、右の列の全体を 1 つで送る。
     モデルの表示に高さを多く残すため、ツールバーを 2 行に収め (モデルの名前は下の帯へ)、パネルは 45% にする。 */
  @media (max-width: 720px) {
    main {
      flex-direction: column;
    }
    .sidebar {
      width: auto;
      height: 45%;
      border-left: none;
      border-top: 1px solid var(--ugoe-color-border);
      /* 中の段 (機能のパネル・パラメータ) が右の padding でスクロールバーの分を空ける */
      overflow-y: auto;
    }
    .features {
      max-height: none;
      overflow: visible;
    }
    /* 縦に積んだ段を縮めず、右の列ごと送る (縮めるとライティングなどの中身がほぼ見えなくなる。T0907)。
       パラメータの高さの下限と一覧の中の送りより強くするため、クラスを重ねて詳細度を上げる。 */
    .sidebar.sidebar > :global(*) {
      flex: none;
      min-height: 0;
      max-height: none;
      overflow: visible;
    }
    /* パラメータの一覧も中で送らない (グループの見出しが右の列の送りに対して上に残るように)。 */
    .sidebar :global(.parameters .list) {
      overflow: visible;
    }
    .toolbar {
      row-gap: var(--ugoe-space-2);
    }
    .toolbar .name {
      display: none;
    }
  }
  .status .name {
    display: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (max-width: 720px) {
    .status .name {
      display: inline;
      min-width: 0;
    }
    .status .loaded {
      display: none;
    }
  }
  .empty-title {
    font-size: var(--ugoe-font-size-lg);
    font-weight: var(--ugoe-font-weight-strong);
    color: var(--ugoe-color-text);
  }
  .empty-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--ugoe-space-2);
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
