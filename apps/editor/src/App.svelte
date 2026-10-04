<!--
  画面のレイアウトだけを持つ。状態は lib/ の EditorDocument (ドキュメント) と
  ViewportController (表示) に、各領域の中身は lib/panels/ と lib/viewport/ に置く。
  中央はビューポート (上) とタイムライン (下。モーションと表情、lib/panels/timeline/)。
  右の列はインスペクタ (上) とパラメータ (下) に分け、区切りをドラッグ (または矢印キー) で動かす (lib/layout.ts)。
  上の区画は「Inspector」「Physics」(lib/panels/physics/)「Glue」(lib/panels/glues/)「Slots」(lib/panels/slots/)
  「Poses」(lib/panels/poses/)「Verify」(lib/panels/verify/) をタブで切り替える。
  Undo / Redo のショートカット (lib/shortcuts.ts) もここで受ける。
  ヘッダーの「Settings」は AI の設定 (lib/ai/SettingsDialog.svelte)。外部の AI に送る前の確認
  (lib/ai/ConsentDialog.svelte) もここに置く。
  上の区画の「Lighting」タブはライティングの操作 (lib/panels/lighting/)。開いている間とプレビュー中は、ビューポートに
  光源の取っ手を重ねる。プレビューの光源 (LightingView) はいつもビューポートに渡す (docs/editor.md#ライティングの画面)。
  上の区画の「AI」タブは AI の提案 (lib/ai/ProposalsPanel.svelte)。提案をプレビューしている間は、ビューポートに
  提案を適用した複製を渡し、編集のツールを隠す (docs/editor.md#ai-の提案)。
  ヘッダーの「Agents」はエディタにつないだエージェント (lib/agents/。設定の「Connect agents」) の状態。エージェントの
  適用・保存の確認 (lib/agents/ApprovalDialog.svelte) もここに置き、ブリッジのイベントで文書と提案を読み直す。
  プレビューモード (lib/preview/) では、左の列・タイムライン・編集のツールと右の列を隠し、右にプレビューのパネルを出す。
-->
<script lang="ts">
import { Toaster, TooltipProvider } from "@ugoe/ui";
import { onMount, untrack } from "svelte";
import ApprovalDialog from "./lib/agents/ApprovalDialog.svelte";
import { agentBridge } from "./lib/agents/bridge.svelte";
import { consentPrompt } from "./lib/ai/ai.svelte";
import ConsentDialog from "./lib/ai/ConsentDialog.svelte";
import ProposalsPanel from "./lib/ai/ProposalsPanel.svelte";
import { ProposalsView } from "./lib/ai/proposals.svelte";
import SettingsDialog from "./lib/ai/SettingsDialog.svelte";
import BaseLock from "./lib/BaseLock.svelte";
import { EditorDocument } from "./lib/document.svelte";
import { emptyViewportHint } from "./lib/emptyState";
import FileMenu from "./lib/FileMenu.svelte";
import HelpButton from "./lib/HelpButton.svelte";
import { t } from "./lib/i18n";
import { type AppInfo, type DifferenceItem, ipc } from "./lib/ipc";
import { loadSplit, saveSplit, splitAt, splitByKey } from "./lib/layout";
import { renamePrefix } from "./lib/moduleProject";
import GluesPanel from "./lib/panels/glues/GluesPanel.svelte";
import { GlueView } from "./lib/panels/glues/glueView.svelte";
import InspectorPanel from "./lib/panels/InspectorPanel.svelte";
import LightingPanel from "./lib/panels/lighting/LightingPanel.svelte";
import { LightingView } from "./lib/panels/lighting/lightingView.svelte";
import ParametersPanel from "./lib/panels/ParametersPanel.svelte";
import PhysicsPanel from "./lib/panels/physics/PhysicsPanel.svelte";
import { PhysicsView } from "./lib/panels/physics/physicsView.svelte";
import PosesPanel from "./lib/panels/poses/PosesPanel.svelte";
import SlotsPanel from "./lib/panels/slots/SlotsPanel.svelte";
import { SlotsView } from "./lib/panels/slots/slotsView.svelte";
import TreePanel from "./lib/panels/TreePanel.svelte";
import TimelinePanel from "./lib/panels/timeline/TimelinePanel.svelte";
import VerifyPanel from "./lib/panels/verify/VerifyPanel.svelte";
import { VerifyView } from "./lib/panels/verify/verifyView.svelte";
import DifferenceOverlay from "./lib/preview/DifferenceOverlay.svelte";
import { differenceTarget } from "./lib/preview/differenceHighlight";
import PreviewPanel from "./lib/preview/PreviewPanel.svelte";
import { PreviewMode } from "./lib/preview/preview.svelte";
import { Selection } from "./lib/selection.svelte";
import { historyShortcut, isInDialog, isMac, isTextEntry } from "./lib/shortcuts";
import TabStrip, { type TabItem } from "./lib/ui/TabStrip.svelte";
import TipButton from "./lib/ui/TipButton.svelte";
import { ViewportController } from "./lib/viewport/controller.svelte";
import ToolOverlay from "./lib/viewport/ToolOverlay.svelte";
import LightingOverlay from "./lib/viewport/tools/lighting/LightingOverlay.svelte";
import { summarizeModel } from "./lib/viewport/tools/model";
import Viewport from "./lib/viewport/Viewport.svelte";

const doc = new EditorDocument();
const viewport = new ViewportController();
const selection = new Selection();
const physics = new PhysicsView();
const glue = new GlueView();
const verify = new VerifyView();
const slots = new SlotsView();
const preview = new PreviewMode(doc, viewport);
const proposals = new ProposalsView(doc, viewport);
const lighting = new LightingView();
/** 「Poses」のタブを開いているか。 */
let posesActive = $state(false);
let info = $state<AppInfo | null>(null);
let fileMenu = $state<FileMenu>();
let settings = $state<SettingsDialog>();

ipc
  .appInfo()
  .then((i) => {
    info = i;
  })
  .catch((e) => console.warn("ugoe: could not read the app info:", e));
doc.refresh();
void lighting.load();
lighting.onsaved = () => fileMenu?.refresh();
void preview.dressUp.load();
preview.dressUp.onsaved = () => fileMenu?.refresh();

// プレビューの光源をビューポートに渡す (編集中もプレビュー中も)。
$effect(() => viewport.setLighting(lighting.json));

// エディタにつないだエージェント: 確認の求め・文書と提案の変化を受ける。
onMount(() => {
  void agentBridge.refresh();
  const unlisten = ipc.onMcpBridgeEvent((event) =>
    agentBridge.handle(event, {
      document: () => void doc.refresh(),
      proposals: () => void proposals.agentProposed(),
    }),
  );
  return () => {
    void unlisten.then((stop) => stop());
  };
});

/** AI タブを開く。 */
function openProposals(): void {
  proposals.active = true;
  proposals.fresh = false;
  physics.active = false;
  glue.active = false;
  verify.active = false;
  lighting.active = false;
  slots.active = false;
  posesActive = false;
}

/** 上の区画のタブを切り替える (AI タブは openProposals)。 */
function openTab(
  tab: "inspector" | "physics" | "glue" | "lighting" | "slots" | "poses" | "verify",
): void {
  physics.active = tab === "physics";
  glue.active = tab === "glue";
  lighting.active = tab === "lighting";
  slots.active = tab === "slots";
  posesActive = tab === "poses";
  verify.active = tab === "verify";
  proposals.active = false;
}

/**
 * 着せ替えの違いの行の要素を選ぶ: プレビューを出て、ツリーか Physics・Glue・Poses タブで選ぶ
 * (T0656・T0657・T0659)。
 */
function selectDifference(item: DifferenceItem): void {
  const meshes = new Set(summarizeModel(doc.model?.json).meshes.map((m) => m.id));
  const target = differenceTarget(item, doc.module?.name ?? "", (id) => meshes.has(id));
  if (!target) return;
  // ツリーと右の列のタブはプレビューの間は隠れるので、出てから選ぶ。
  preview.exit();
  switch (target.panel) {
    case "tree":
      selection.set(target.refs);
      return;
    case "glue":
      openTab("glue");
      glue.glue = target.glue;
      if (target.refs.length > 0) selection.set(target.refs);
      return;
    case "poses":
      openTab("poses");
      selection.set(target.refs);
      return;
    case "physics":
      openTab("physics");
      if (target.rig !== null) physics.rig = target.rig;
      if (target.collider !== null) physics.collider = target.collider;
  }
}

/** 上の区画のタブ。 */
type PaneTab = "inspector" | "physics" | "glue" | "lighting" | "slots" | "poses" | "verify" | "ai";

const currentTab = $derived<PaneTab>(
  proposals.active
    ? "ai"
    : verify.active
      ? "verify"
      : physics.active
        ? "physics"
        : glue.active
          ? "glue"
          : lighting.active
            ? "lighting"
            : slots.active
              ? "slots"
              : posesActive
                ? "poses"
                : "inspector",
);

const paneTabs = $derived<TabItem<PaneTab>[]>([
  { value: "inspector", label: t("app.tabs.inspector.label") },
  { value: "physics", label: t("app.tabs.physics.label") },
  { value: "glue", label: t("app.tabs.glue.label"), tooltip: t("app.tabs.glue.tooltip") },
  {
    value: "lighting",
    label: t("app.tabs.lighting.label"),
    tooltip: t("app.tabs.lighting.tooltip"),
  },
  { value: "slots", label: t("app.tabs.slots.label"), tooltip: t("app.tabs.slots.tooltip") },
  { value: "poses", label: t("app.tabs.poses.label"), tooltip: t("app.tabs.poses.tooltip") },
  { value: "verify", label: t("app.tabs.verify.label") },
  {
    value: "ai",
    label:
      proposals.count > 0
        ? t("app.tabs.ai.count", { count: proposals.count })
        : t("app.tabs.ai.label"),
    tooltip:
      proposals.staleRigs.length > 0
        ? t("app.tabs.ai.stale")
        : proposals.fresh
          ? t("app.tabs.ai.fresh")
          : t("app.tabs.ai.tooltip"),
    attention: proposals.fresh || proposals.staleRigs.length > 0,
  },
]);

const mac = isMac(navigator.platform);

/**
 * Undo / Redo のショートカット。入力欄の中では、入力欄自身の取り消しに任せる。ダイアログの中では何もしない
 * (lib/shortcuts.ts の isInDialog)。
 */
function onKeydown(e: KeyboardEvent): void {
  const action = historyShortcut(e, mac);
  if (!action || isTextEntry(e.target) || isInDialog(e.target) || preview.active) return;
  e.preventDefault();
  if (action === "undo") doc.undo();
  else doc.redo();
}

// 右の列の上下の分け方 (インスペクタの高さの割合)。
let split = $state(loadSplit());
let rightColumn = $state<HTMLElement>();
let dragging = $state(false);

function onSplitterDown(e: PointerEvent): void {
  dragging = true;
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
}

function onSplitterMove(e: PointerEvent): void {
  if (!dragging || !rightColumn) return;
  const rect = rightColumn.getBoundingClientRect();
  split = splitAt(e.clientY - rect.top, rect.height);
}

function onSplitterUp(): void {
  if (!dragging) return;
  dragging = false;
  saveSplit(split);
}

function onSplitterKey(e: KeyboardEvent): void {
  const next = splitByKey(split, e.key);
  if (next === null) return;
  e.preventDefault();
  split = next;
  saveSplit(split);
}

// ドキュメントを読み直すたびに、プロジェクト名と未保存の印も取り直す。
$effect(() => {
  if (doc.revision > 0) fileMenu?.refresh();
});

// ドキュメントが変わったら提案のプレビューをやめ、提案が古くなったかを取り直す。
$effect(() => {
  const revision = doc.revision;
  if (revision > 0) untrack(() => proposals.documentChanged(revision));
});
</script>

<svelte:window onkeydown={onKeydown} />

<TooltipProvider>
<div class="shell" class:previewing={preview.active}>
  <header>
    <strong>ugoe</strong>
    <span class="muted">{info ? `v${info.version}` : "…"}</span>
    <!-- 別のモデルになったら (新規・開く・取り込み) キャンバス全体を表示し直す。外での書き換えの後の開き直しでは保つ。 -->
    <FileMenu
      bind:this={fileMenu}
      {doc}
      onchange={(replaced, reloaded) => {
        preview.exit();
        if (!reloaded) viewport.fit();
        if (replaced) viewport.resetParametersOnNextModel();
        doc.refresh();
        void lighting.load();
        void preview.dressUp.load();
      }}
      onrenamemodule={(from, to) => selection.set(renamePrefix(selection.items, from, to))}
    />
    <TipButton
      variant="ghost"
      pressed={preview.active}
      onclick={() => preview.toggle()}
      tooltip={preview.active ? t("app.preview.exitTooltip") : t("app.preview.tooltip")}
      >{preview.active ? t("app.preview.exit") : t("app.preview.label")}</TipButton
    >
    {#if !preview.active}
      <TipButton variant="ghost" onclick={() => doc.undo()} tooltip={t("app.undo.tooltip")} shortcut="Mod+Z"
        >{t("app.undo.label")}</TipButton
      >
      <TipButton
        variant="ghost"
        onclick={() => doc.redo()}
        tooltip={t("app.redo.tooltip")}
        shortcut={mac ? "Mod+Shift+Z" : "Mod+Y"}>{t("app.redo.label")}</TipButton
      >
    {/if}
    {#if agentBridge.status?.running}
      <TipButton variant="ghost" class="agents" onclick={() => settings?.open("agents")} tooltip={t("app.agents.tooltip")}
        ><span class="dot" aria-hidden="true"></span>{agentBridge.connected > 0
          ? t("app.agents.connected", { count: agentBridge.connected })
          : t("app.agents.label")}</TipButton
      >
    {/if}
    <TipButton variant="ghost" onclick={() => settings?.open()} tooltip={t("app.settings.tooltip")}
      >{t("app.settings.label")}</TipButton
    >
    <HelpButton />
  </header>
  <aside class="left">
    <TreePanel {doc} {selection} />
  </aside>
  <main class="center">
    <div class="viewport-area">
      <Viewport controller={viewport} model={proposals.preview?.model ?? doc.model}
        hint={doc.empty && !proposals.preview ? emptyViewportHint() : null}>
        {#snippet overlay(controller)}
          {#if proposals.preview}
            <p class="proposal-banner" role="status">{t("app.proposalBanner")}</p>
          {:else if !preview.active}<ToolOverlay {controller} {doc} {selection} {physics} {verify} {glue} />
          {:else}<DifferenceOverlay {controller} {doc} {preview} />{/if}
          {#if !proposals.preview && (lighting.active || preview.active)}
            <LightingOverlay {controller} {doc} view={lighting} />
          {/if}
        {/snippet}
      </Viewport>
    </div>
    <div class="timeline-slot"><TimelinePanel {doc} {viewport} /></div>
  </main>
  <aside class="right" bind:this={rightColumn}>
    <div class="pane inspector" style:flex-basis="{split * 100}%">
      <div class="tabs">
        <TabStrip
          tabs={paneTabs}
          value={currentTab}
          ariaLabel={t("app.tabs.aria")}
          attentionLabel={t("app.tabs.ai.attention")}
          onselect={(tab) => (tab === "ai" ? openProposals() : openTab(tab))}
        />
      </div>
      {#if doc.error}<p class="error ugoe-selectable">{doc.error}</p>{/if}
      {#if proposals.active}
        <ProposalsPanel
          view={proposals}
          {viewport}
          module={doc.module !== null}
          onopensettings={() => settings?.open("ai")}
        />
      {:else if verify.active}
        <VerifyPanel {doc} {selection} {viewport} view={verify} {physics} />
      {:else if physics.active}
        <PhysicsPanel {doc} {selection} {viewport} view={physics} />
      {:else if glue.active}
        <GluesPanel {doc} {selection} view={glue} />
      {:else if lighting.active}
        <LightingPanel {doc} {selection} view={lighting} />
      {:else if slots.active}
        <BaseLock
          locked={doc.module !== null}
          hint={t("app.slotsBaseHint")}
        >
          <SlotsPanel {doc} {selection} view={slots} />
        </BaseLock>
      {:else if posesActive}
        <PosesPanel {doc} {selection} {viewport} />
      {:else}
        <InspectorPanel {doc} {selection} {viewport} staleRigs={proposals.staleRigs} onopenai={openProposals} />
      {/if}
    </div>
    <!-- フォーカスできる区切り (ARIA の separator) はキー操作を受ける部品。 -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="splitter"
      class:dragging
      role="separator"
      aria-orientation="horizontal"
      aria-label={t("app.splitter.aria")}
      aria-valuemin={15}
      aria-valuemax={85}
      aria-valuenow={Math.round(split * 100)}
      tabindex="0"
      onpointerdown={onSplitterDown}
      onpointermove={onSplitterMove}
      onpointerup={onSplitterUp}
      onpointercancel={onSplitterUp}
      onkeydown={onSplitterKey}
    ></div>
    <div class="pane parameters">
      <ParametersPanel parameters={doc.parameters} {viewport} {doc} {selection} />
    </div>
  </aside>
  {#if preview.active}
    <aside class="preview">
      <PreviewPanel {preview} {doc} {viewport} {lighting} onselect={selectDifference} />
    </aside>
  {/if}
</div>
<SettingsDialog bind:this={settings} onchange={() => fileMenu?.refresh()} />
<ConsentDialog prompt={consentPrompt} />
<ApprovalDialog
  proposals={proposals.value?.proposals ?? []}
  onshow={(id, excluded) => {
    preview.exit();
    openProposals();
    void proposals.previewRequested(id, excluded);
  }}
/>

</TooltipProvider>
<Toaster />

<style>
  /* プレビュー中は編集の列を隠す (中身は壊さず、出たときに元の状態で戻る)。 */
  .shell.previewing {
    grid-template-columns: minmax(0, 1fr) 320px;
    grid-template-areas: "header header" "viewport preview";
  }
  .shell.previewing .left,
  .shell.previewing .right,
  .shell.previewing .timeline-slot {
    display: none;
  }
  .timeline-slot {
    display: contents;
  }
  .preview {
    grid-area: preview;
    border-left: 1px solid var(--ugoe-color-border);
  }
  .proposal-banner {
    position: absolute;
    top: var(--ugoe-space-3);
    left: var(--ugoe-space-3);
    /* 右上の HUD (Viewport.svelte) と重ならないよう、その幅を空ける */
    max-width: calc(100% - var(--viewport-hud-width, 0px) - 3 * var(--ugoe-space-3));
    margin: 0;
    padding: var(--ugoe-space-2) var(--ugoe-space-3);
    border: 1px solid var(--ugoe-color-accent);
    border-radius: var(--ugoe-radius-md);
    background: var(--ugoe-color-surface);
    color: var(--ugoe-color-accent);
  }
  /* 幅が足りないときはファイルのメニューだけが縮んで折り返し、右のボタンと名前は縮めない
     (ヘッダーの高さが伸びる)。 */
  /* 失敗の知らせの帯 (FileMenu の .bands) は折り返して最後の行に入る。 */
  header {
    flex-wrap: wrap;
    min-height: 36px;
    padding-block: var(--ugoe-space-2);
  }
  header > :global(:not(.file-menu, .bands)) {
    flex-shrink: 0;
    white-space: nowrap;
  }
  /* ヘッダーを 1 行に収めるため、ボタンの左右の余白を詰める (押せる大きさは変えない) */
  header :global(.ugoe-button--ghost) {
    padding-inline: var(--ugoe-space-2);
  }
  header :global(.agents) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  header :global(.agents .dot) {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ugoe-color-success);
  }
  .center {
    display: flex;
    flex-direction: column;
  }
  .viewport-area {
    flex: 1;
    min-height: 0;
    position: relative;
  }
  .right {
    display: flex;
    flex-direction: column;
    padding: 0;
    overflow: hidden;
  }
  .pane {
    min-height: 0;
    min-width: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 0 12px 8px;
  }
  .pane.inspector {
    flex: 0 0 auto;
  }
  .tabs {
    position: sticky;
    top: 0;
    z-index: 1;
    padding-top: var(--ugoe-space-3);
    margin-bottom: var(--ugoe-space-2);
    background: var(--ugoe-color-surface);
  }
  .pane.parameters {
    flex: 1 1 0;
  }
  .splitter {
    flex: none;
    height: 6px;
    border-block: 1px solid var(--ugoe-color-border);
    background: var(--ugoe-color-bg);
    cursor: row-resize;
    touch-action: none;
  }
  .splitter:hover,
  .splitter:focus-visible,
  .splitter.dragging {
    background: var(--ugoe-color-accent);
    outline: none;
  }
</style>
