<!--
  画面のレイアウトだけを持つ。状態は lib/ の EditorDocument (ドキュメント) と
  ViewportController (表示) に、各領域の中身は lib/panels/ と lib/viewport/ に置く。
  中央はビューポート (上) とタイムライン (下。モーションと表情、lib/panels/timeline/)。
  右の列はインスペクタ (上) とパラメータ (下) に分け、区切りをドラッグ (または矢印キー) で動かす (lib/layout.ts)。
  上の区画は「Inspector」「Physics」(lib/panels/physics/)「Verify」(lib/panels/verify/) をタブで切り替える。
  Undo / Redo のショートカット (lib/shortcuts.ts) もここで受ける。
  プレビューモード (lib/preview/) では、左の列・タイムライン・編集のツールと右の列を隠し、右にプレビューのパネルを出す。
-->
<script lang="ts">
import { EditorDocument } from "./lib/document.svelte";
import { EMPTY_VIEWPORT_HINT } from "./lib/emptyState";
import FileMenu from "./lib/FileMenu.svelte";
import HelpButton from "./lib/HelpButton.svelte";
import { type AppInfo, ipc } from "./lib/ipc";
import { loadSplit, saveSplit, splitAt, splitByKey } from "./lib/layout";
import InspectorPanel from "./lib/panels/InspectorPanel.svelte";
import ParametersPanel from "./lib/panels/ParametersPanel.svelte";
import PhysicsPanel from "./lib/panels/physics/PhysicsPanel.svelte";
import { PhysicsView } from "./lib/panels/physics/physicsView.svelte";
import TreePanel from "./lib/panels/TreePanel.svelte";
import TimelinePanel from "./lib/panels/timeline/TimelinePanel.svelte";
import VerifyPanel from "./lib/panels/verify/VerifyPanel.svelte";
import { VerifyView } from "./lib/panels/verify/verifyView.svelte";
import PreviewPanel from "./lib/preview/PreviewPanel.svelte";
import { PreviewMode } from "./lib/preview/preview.svelte";
import { Selection } from "./lib/selection.svelte";
import { historyShortcut, isMac, isTextEntry } from "./lib/shortcuts";
import { ViewportController } from "./lib/viewport/controller.svelte";
import ToolOverlay from "./lib/viewport/ToolOverlay.svelte";
import Viewport from "./lib/viewport/Viewport.svelte";

const doc = new EditorDocument();
const viewport = new ViewportController();
const selection = new Selection();
const physics = new PhysicsView();
const verify = new VerifyView();
const preview = new PreviewMode(doc, viewport);
let info = $state<AppInfo | null>(null);
let fileMenu = $state<FileMenu>();

ipc.appInfo().then((i) => {
  info = i;
});
doc.refresh();

const mac = isMac(navigator.platform);
const undoKeys = mac ? "Cmd+Z" : "Ctrl+Z";
const redoKeys = mac ? "Cmd+Shift+Z" : "Ctrl+Shift+Z / Ctrl+Y";

/** Undo / Redo のショートカット。入力欄の中では、入力欄自身の取り消しに任せる。 */
function onKeydown(e: KeyboardEvent): void {
  const action = historyShortcut(e, mac);
  if (!action || isTextEntry(e.target) || preview.active) return;
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
</script>

<svelte:window onkeydown={onKeydown} />

<div class="shell" class:previewing={preview.active}>
  <header>
    <strong>ugoe</strong>
    <span class="muted">{info ? `v${info.version}` : "…"}</span>
    <!-- 別のモデルになったら (新規・開く・取り込み) キャンバス全体を表示し直す。 -->
    <FileMenu
      bind:this={fileMenu}
      onchange={(replaced) => {
        preview.exit();
        viewport.fit();
        if (replaced) viewport.resetParametersOnNextModel();
        doc.refresh();
      }}
    />
    <span class="spacer"></span>
    <button class:active={preview.active} onclick={() => preview.toggle()}
      title={preview.active
        ? "Leave the preview and go back to the pose you were editing"
        : "Preview the model with the runtime (physics, eye blink, breath, motions) and all parameters"}
      >{preview.active ? "Exit preview" : "Preview"}</button>
    {#if !preview.active}
      <button onclick={() => doc.undo()} title="Undo ({undoKeys})">Undo</button>
      <button onclick={() => doc.redo()} title="Redo ({redoKeys})">Redo</button>
    {/if}
    <HelpButton />
  </header>
  <aside class="left">
    <TreePanel {doc} {selection} />
  </aside>
  <main class="center">
    <div class="viewport-area">
      <Viewport controller={viewport} model={doc.model} hint={doc.empty ? EMPTY_VIEWPORT_HINT : null}>
        {#snippet overlay(controller)}
          {#if !preview.active}<ToolOverlay {controller} {doc} {selection} {physics} {verify} />{/if}
        {/snippet}
      </Viewport>
    </div>
    <div class="timeline-slot"><TimelinePanel {doc} {viewport} /></div>
  </main>
  <aside class="right" bind:this={rightColumn}>
    <div class="pane inspector" style:flex-basis="{split * 100}%">
      <div class="tabs" role="tablist">
        <button role="tab" aria-selected={!physics.active && !verify.active}
          class:current={!physics.active && !verify.active}
          onclick={() => ((physics.active = false), (verify.active = false))}>Inspector</button>
        <button role="tab" aria-selected={physics.active} class:current={physics.active}
          onclick={() => ((physics.active = true), (verify.active = false))}>Physics</button>
        <button role="tab" aria-selected={verify.active} class:current={verify.active}
          onclick={() => ((verify.active = true), (physics.active = false))}>Verify</button>
      </div>
      {#if doc.error}<p class="error">{doc.error}</p>{/if}
      {#if verify.active}
        <VerifyPanel {doc} {selection} {viewport} view={verify} {physics} />
      {:else if physics.active}
        <PhysicsPanel {doc} {selection} {viewport} view={physics} />
      {:else}
        <InspectorPanel {doc} {selection} {viewport} />
      {/if}
    </div>
    <!-- フォーカスできる区切り (ARIA の separator) はキー操作を受ける部品。 -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="splitter"
      class:dragging
      role="separator"
      aria-orientation="horizontal"
      aria-label="Resize the inspector and the parameters"
      aria-valuemin={15}
      aria-valuemax={85}
      aria-valuenow={Math.round(split * 100)}
      tabindex="0"
      title="Drag (or press ↑ / ↓) to resize the inspector and the parameters"
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
      <PreviewPanel {preview} {doc} {viewport} />
    </aside>
  {/if}
</div>

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
    border-left: 1px solid var(--border);
  }
  header button.active {
    color: var(--accent);
    border-color: var(--accent);
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
    display: flex;
    gap: 4px;
    padding: 8px 0 4px;
    margin-bottom: 4px;
    background: var(--panel, var(--bg));
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    border-bottom-left-radius: 0;
    border-bottom-right-radius: 0;
    border-bottom: 2px solid transparent;
  }
  .tabs button.current {
    border-bottom-color: var(--accent);
    color: var(--accent);
  }
  .pane.parameters {
    flex: 1 1 0;
  }
  .splitter {
    flex: none;
    height: 6px;
    border-block: 1px solid var(--border);
    background: var(--bg);
    cursor: row-resize;
    touch-action: none;
  }
  .splitter:hover,
  .splitter:focus-visible,
  .splitter.dragging {
    background: var(--accent);
    outline: none;
  }
</style>
