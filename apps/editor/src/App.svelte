<!--
  画面のレイアウトだけを持つ。状態は lib/ の EditorDocument (ドキュメント) と
  ViewportController (表示) に、各領域の中身は lib/panels/ と lib/viewport/ に置く。
  中央はビューポート (上) とタイムライン (下。モーションと表情、lib/panels/timeline/)。
  右の列はインスペクタ (上) とパラメータ (下) に分け、区切りをドラッグ (または矢印キー) で動かす (lib/layout.ts)。
  上の区画は「Inspector」「Physics」(lib/panels/physics/)「Glue」(lib/panels/glues/)「Slots」(lib/panels/slots/)
  「Poses」(lib/panels/poses/)「Verify」(lib/panels/verify/) をタブで切り替える。
  Undo / Redo のショートカット (lib/shortcuts.ts) もここで受ける。Undo / Redo のボタンは戻れない・進めないときに押せない。
  ヘッダーの「Settings」は AI の設定 (lib/ai/SettingsDialog.svelte)。外部の AI に送る前の確認
  (lib/ai/ConsentDialog.svelte) もここに置く。
  上の区画の「Lighting」タブはライティングの操作 (lib/panels/lighting/)。開いている間とプレビュー中は、ビューポートに
  光源の取っ手を重ねる。プレビューの光源 (LightingView) はいつもビューポートに渡す (docs/editor.md#ライティングの画面)。
  上の区画の「AI」タブは AI の提案 (lib/ai/ProposalsPanel.svelte)。提案をプレビューしている間は、ビューポートに
  提案を適用した複製を渡し、編集のツールを隠す (docs/editor.md#ai-の提案)。
  ヘッダーの「Agents」はエディタにつないだエージェント (lib/agents/。設定の「Connect agents」) の状態。エージェントの
  適用・保存の確認 (lib/agents/ApprovalDialog.svelte) もここに置き、ブリッジのイベントで文書と提案を読み直す。
  プレビューモード (lib/preview/) では、左の列・タイムライン・編集のツールと右の列を隠し、右にプレビューのパネルを出す。
  配置 (docs/editor.md#画面の配置): ヘッダーは左から「File」のメニュー・保存・書き出し・Undo・Redo・プロジェクトの名前、右端に
  プレビュー・エージェント・設定・ヘルプ。左右の列の幅とタイムラインの高さは区切り (lib/ui/Splitter.svelte) で変えて覚える。
  ファイルのショートカット (保存・別名で保存・開く) もここで受ける。
-->
<script lang="ts">
import { Button, Notice, Toaster, TooltipProvider } from "@ugoe/ui";
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
import FileMenu from "./lib/FileMenu.svelte";
import HelpButton from "./lib/HelpButton.svelte";
import { t } from "./lib/i18n";
import { type DifferenceItem, ipc } from "./lib/ipc";
import {
  isNarrowWindow,
  LEFT_WIDTH,
  loadSize,
  loadSplit,
  RIGHT_SPLIT,
  RIGHT_WIDTH,
  TIMELINE_HEIGHT,
  timelineOpen as timelineOpenFor,
} from "./lib/layout";
import ModuleErrorHelp from "./lib/ModuleErrorHelp.svelte";
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
import { loadPanelOpen } from "./lib/panels/timeline/timeline";
import VerifyPanel from "./lib/panels/verify/VerifyPanel.svelte";
import { VerifyView } from "./lib/panels/verify/verifyView.svelte";
import DifferenceOverlay from "./lib/preview/DifferenceOverlay.svelte";
import { differenceTarget } from "./lib/preview/differenceHighlight";
import PreviewPanel from "./lib/preview/PreviewPanel.svelte";
import { PreviewMode } from "./lib/preview/preview.svelte";
import { Selection } from "./lib/selection.svelte";
import { fileShortcut, historyShortcut, isInDialog, isMac, isTextEntry } from "./lib/shortcuts";
import Splitter from "./lib/ui/Splitter.svelte";
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
let fileMenu = $state<FileMenu>();
let settings = $state<SettingsDialog>();

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

// 開いた後にバックグラウンドで戻した分類の答え (docs/editor.md#分類の答えのキャッシュ) を AI タブに出す
// (戻している間の印も、戻せなかったときに下ろす)。
onMount(() => {
  const unlisten = ipc.onClassificationRestoreFinished(() => void proposals.refresh());
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
  const meshes = new Set(summarizeModel(doc.parsedModel).meshes.map((m) => m.id));
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

/** 右の列のタブはアイコンだけなので、Tooltip に名前と何をするタブかを出す (docs/editor.md#画面の配置)。 */
function paneTab(value: Exclude<PaneTab, "ai">): TabItem<PaneTab> {
  const label = t(`app.tabs.${value}.label`);
  return {
    value,
    icon: value,
    label,
    tooltip: t("app.tabs.tooltip", { label, description: t(`app.tabs.${value}.tooltip`) }),
  };
}

const paneTabs = $derived.by<TabItem<PaneTab>[]>(() => {
  const aiLabel =
    proposals.count > 0
      ? t("app.tabs.ai.count", { count: proposals.count })
      : t("app.tabs.ai.label");
  const aiDescription =
    proposals.staleRigs.length > 0
      ? t("app.tabs.ai.stale")
      : proposals.fresh
        ? t("app.tabs.ai.fresh")
        : t("app.tabs.ai.tooltip");
  return [
    paneTab("inspector"),
    paneTab("physics"),
    paneTab("glue"),
    paneTab("lighting"),
    paneTab("slots"),
    paneTab("poses"),
    paneTab("verify"),
    {
      value: "ai",
      icon: "ai",
      label: aiLabel,
      tooltip: t("app.tabs.tooltip", { label: aiLabel, description: aiDescription }),
      badge: proposals.count > 0 ? proposals.count : undefined,
      attention: proposals.fresh || proposals.staleRigs.length > 0,
    },
  ];
});

const mac = isMac(navigator.platform);

/**
 * Undo / Redo と、ファイルの操作 (保存・別名で保存・開く) のショートカット。入力欄の中では、入力欄自身の取り消しに任せ、
 * 保存も確定 (Enter・フォーカスを外す) の後に押してもらう。ダイアログの中では何もしない (lib/shortcuts.ts の isInDialog)。
 */
function onKeydown(e: KeyboardEvent): void {
  if (isTextEntry(e.target) || isInDialog(e.target)) return;
  const file = fileShortcut(e, mac);
  if (file) {
    e.preventDefault();
    if (file === "save") void fileMenu?.save();
    else void fileMenu?.pick(file);
    return;
  }
  const action = historyShortcut(e, mac);
  if (!action || preview.active) return;
  e.preventDefault();
  if (action === "undo") doc.undo();
  else doc.redo();
}

// 左右の列の幅・タイムラインの高さ・右の列の上下の分け方 (lib/layout.ts。区切りで変えて覚える)。
let innerWidth = $state(0);
const narrowWindow = $derived(isNarrowWindow(innerWidth));
let leftWidth = $state(loadSize(LEFT_WIDTH));
let rightWidth = $state(loadSize(RIGHT_WIDTH));
let timelineHeight = $state(loadSize(TIMELINE_HEIGHT));
let split = $state<number | null>(loadSplit());
// タイムラインは、空のプロジェクトと狭い窓では利用者が開閉するまで畳む (timelineOpen。覚えた開閉は変えない)。
let timelineStored = $state(loadPanelOpen());
let timelineTouched = $state(false);
const timelineOpen = $derived(
  timelineOpenFor(timelineStored, {
    empty: doc.empty,
    narrow: narrowWindow,
    touched: timelineTouched,
  }),
);
let timelineMeasured = $state(0);
let rightColumnHeight = $state(0);

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

<!-- 空のプロジェクトの次の操作 (docs/ui.md#デザイン の「空の状態」)。 -->
{#snippet emptyProject()}
  <p class="empty-text">{t("emptyState.viewport")}</p>
  <div class="ed-buttons empty-actions">
    <Button variant="primary" onclick={() => void fileMenu?.pick("newFromPsd")}>{t("fileMenu.newFromPsd.label")}</Button>
    <Button onclick={() => void fileMenu?.pick("open")}>{t("fileMenu.open.label")}</Button>
    <TipButton onclick={() => void fileMenu?.pick("openSample")} tooltip={t("fileMenu.openSample.tooltip")}
      >{t("fileMenu.openSample.label")}</TipButton
    >
  </div>
{/snippet}

<svelte:window onkeydown={onKeydown} bind:innerWidth />

<TooltipProvider>
<div class="shell" class:previewing={preview.active} style:--left-width="{leftWidth}px" style:--right-width="{rightWidth}px">
  <header>
    <!-- 画面の名前 (見出しの移動の起点。見た目には印の「ugoe」だけを出す) -->
    <h1 class="ugoe-visually-hidden">{t("app.title")}</h1>
    <strong class="brand" aria-hidden="true">ugoe</strong>
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
    >
      {#if !preview.active}
        <!-- 戻れない・進めないときは押せない (doc.history。編集・Undo・Redo の後に読み直す)。 -->
        <span class="history">
          <TipButton
            variant="ghost"
            disabled={!doc.canUndo}
            onclick={() => doc.undo()}
            tooltip={t("app.undo.tooltip")}
            shortcut="Mod+Z">{t("app.undo.label")}</TipButton
          >
          <TipButton
            variant="ghost"
            disabled={!doc.canRedo}
            onclick={() => doc.redo()}
            tooltip={t("app.redo.tooltip")}
            shortcut={mac ? "Mod+Shift+Z" : "Mod+Y"}>{t("app.redo.label")}</TipButton
          >
        </span>
      {/if}
    </FileMenu>
    <TipButton
      variant="ghost"
      pressed={preview.active}
      onclick={() => preview.toggle()}
      tooltip={preview.active ? t("app.preview.exitTooltip") : t("app.preview.tooltip")}
      >{preview.active ? t("app.preview.exit") : t("app.preview.label")}</TipButton
    >
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
  <aside class="left" id="editor-left" aria-label={t("app.columns.left")}>
    <TreePanel {doc} {selection} onfileaction={(action) => void fileMenu?.pick(action)} />
  </aside>
  <div class="column-splitter left-splitter">
    <Splitter label={t("app.splitter.left")} orientation="vertical" range={LEFT_WIDTH} bind:value={leftWidth}
      controls="editor-left" />
  </div>
  <main class="center">
    <div class="viewport-area">
      <Viewport controller={viewport} model={proposals.preview?.model ?? doc.model}
        empty={doc.empty && !proposals.preview ? emptyProject : undefined}>
        {#snippet overlay(controller)}
          {#if proposals.preview}
            <p class="proposal-banner" role="status">{t("app.proposalBanner")}</p>
          {:else if !preview.active && !doc.empty}
            <!-- 空のプロジェクトでは編集の道具を出さない (編集するものが無く、中央の次の操作と重なる) -->
            <ToolOverlay {controller} {doc} {selection} {physics} {verify} {glue} />
          {:else}<DifferenceOverlay {controller} {doc} {preview} />{/if}
          {#if !proposals.preview && (lighting.active || preview.active)}
            <LightingOverlay {controller} {doc} view={lighting} />
          {/if}
        {/snippet}
      </Viewport>
    </div>
    <div class="timeline-slot" id="editor-timeline" bind:clientHeight={timelineMeasured}
      style:--timeline-height={timelineHeight === null ? undefined : `${timelineHeight}px`}>
      {#if timelineOpen}
        <Splitter label={t("app.splitter.timeline")} orientation="horizontal" reverse range={TIMELINE_HEIGHT}
          bind:value={timelineHeight} measure={() => timelineMeasured} controls="editor-timeline" />
      {/if}
      <TimelinePanel {doc} {viewport} open={timelineOpen} ontoggle={(open) => {
        timelineStored = open;
        timelineTouched = true;
      }} />
    </div>
  </main>
  <div class="column-splitter right-splitter">
    <Splitter label={t("app.splitter.right")} orientation="vertical" reverse range={RIGHT_WIDTH}
      bind:value={rightWidth} controls="editor-right" />
  </div>
  <aside class="right" id="editor-right" aria-label={t("app.columns.right")} bind:clientHeight={rightColumnHeight}>
    <div class="pane inspector" id="editor-inspector" style:flex-basis="{(split ?? RIGHT_SPLIT.min) * 100}%">
      <div class="tabs">
        <TabStrip
          tabs={paneTabs}
          value={currentTab}
          aria-label={t("app.tabs.aria")}
          attentionLabel={t("app.tabs.ai.attention")}
          icons
          onselect={(tab) => (tab === "ai" ? openProposals() : openTab(tab))}
        />
      </div>
      {#if doc.error}
        <div class="doc-error">
          <Notice tone="danger" title={t("app.documentError")} ondismiss={() => (doc.error = null)}>
            <p>{doc.error}</p>
            <ModuleErrorHelp error={doc.failure} base={doc.module?.base ?? null}
              onopenbase={() => void fileMenu?.openModuleBase()} />
          </Notice>
        </div>
      {/if}
      <!-- 中身の見出し (<h2>) はタブと同じ名前なので、各パネルで見た目では隠し、読み上げと見出しの移動に残す。 -->
      <div class="tab-panel" role="tabpanel" aria-label={paneTabs.find((tab) => tab.value === currentTab)?.label}>
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
    </div>
    <Splitter label={t("app.splitter.aria")} orientation="horizontal" range={RIGHT_SPLIT} bind:value={split}
      scale={() => (rightColumnHeight > 0 ? 1 / rightColumnHeight : 0)} controls="editor-inspector" />
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
<SettingsDialog bind:this={settings} {doc} backend={viewport.backend} onchange={() => fileMenu?.refresh()} />

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
  .shell.previewing .column-splitter,
  .shell.previewing .timeline-slot {
    display: none;
  }
  .column-splitter {
    display: flex;
    min-height: 0;
  }
  .left-splitter {
    grid-area: lsplit;
  }
  .right-splitter {
    grid-area: rsplit;
  }
  .timeline-slot {
    display: flex;
    flex: none;
    flex-direction: column;
    min-height: 0;
    /* ビューポートを 120px は残す (区切りで大きくしすぎても潰れない) */
    max-height: calc(100% - 120px);
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
  .empty-text {
    margin: 0;
    color: var(--ugoe-color-text-muted);
  }
  .empty-actions {
    justify-content: center;
  }
  /* ヘッダーは 1 行に収める: プロジェクトの名前だけが縮んで省略される (FileMenu の .project-info)。
     幅が足りないときだけ折り返し、失敗の知らせの帯 (FileMenu の .bands) は最後の行に入る。 */
  header {
    flex-wrap: wrap;
    gap: var(--ugoe-space-1);
    min-height: 36px;
    padding-block: var(--ugoe-space-1);
  }
  header > :global(:not(.project-info, .bands)) {
    flex-shrink: 0;
    white-space: nowrap;
  }
  .brand {
    margin-right: var(--ugoe-space-2);
  }
  .history {
    display: flex;
    gap: var(--ugoe-space-1);
    margin-left: var(--ugoe-space-2);
  }
  /* ボタンの左右の余白を詰める (押せる大きさは変えない) */
  header :global(.ugoe-button--ghost) {
    padding-inline: var(--ugoe-space-2);
  }
  header :global(.agents) {
    display: inline-flex;
    align-items: center;
    gap: var(--ugoe-space-2);
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
  /* 送る枠は位置の基準にする (中の absolute の要素 (見せずに読ませる文字など) が、文書の全体を送れるようにしないため) */
  .pane {
    position: relative;
    min-height: 0;
    min-width: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 0 var(--ugoe-space-4) var(--ugoe-space-3);
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
  .doc-error {
    margin-bottom: var(--ugoe-space-3);
  }
  .doc-error p {
    margin: 0;
    overflow-wrap: anywhere;
  }
</style>
