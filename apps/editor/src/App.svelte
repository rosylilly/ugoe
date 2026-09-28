<!--
  画面のレイアウトだけを持つ。状態は lib/ の EditorDocument (ドキュメント) と
  ViewportController (表示) に、各領域の中身は lib/panels/ と lib/viewport/ に置く。
  中央はビューポート (上) とタイムライン (下。モーションと表情、lib/panels/timeline/)。
  右の列はインスペクタ (上) とパラメータ (下) に分け、区切りをドラッグ (または矢印キー) で動かす (lib/layout.ts)。
  Undo / Redo のショートカット (lib/shortcuts.ts) もここで受ける。
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
import TreePanel from "./lib/panels/TreePanel.svelte";
import TimelinePanel from "./lib/panels/timeline/TimelinePanel.svelte";
import { Selection } from "./lib/selection.svelte";
import { historyShortcut, isMac, isTextEntry } from "./lib/shortcuts";
import { ViewportController } from "./lib/viewport/controller.svelte";
import ToolOverlay from "./lib/viewport/ToolOverlay.svelte";
import Viewport from "./lib/viewport/Viewport.svelte";

const doc = new EditorDocument();
const viewport = new ViewportController();
const selection = new Selection();
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
  if (!action || isTextEntry(e.target)) return;
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

<div class="shell">
  <header>
    <strong>ugoe</strong>
    <span class="muted">{info ? `v${info.version}` : "…"}</span>
    <!-- 別のモデルになったら (新規・開く・取り込み) キャンバス全体を表示し直す。 -->
    <FileMenu
      bind:this={fileMenu}
      onchange={(replaced) => {
        viewport.fit();
        if (replaced) viewport.resetParametersOnNextModel();
        doc.refresh();
      }}
    />
    <span class="spacer"></span>
    <button onclick={() => doc.undo()} title="Undo ({undoKeys})">Undo</button>
    <button onclick={() => doc.redo()} title="Redo ({redoKeys})">Redo</button>
    <HelpButton />
  </header>
  <aside class="left">
    <TreePanel {doc} {selection} />
  </aside>
  <main class="center">
    <div class="viewport-area">
      <Viewport controller={viewport} model={doc.model} hint={doc.empty ? EMPTY_VIEWPORT_HINT : null}>
        {#snippet overlay(controller)}
          <ToolOverlay {controller} {doc} {selection} />
        {/snippet}
      </Viewport>
    </div>
    <TimelinePanel {doc} {viewport} />
  </main>
  <aside class="right" bind:this={rightColumn}>
    <div class="pane inspector" style:flex-basis="{split * 100}%">
      {#if doc.error}<p class="error">{doc.error}</p>{/if}
      <InspectorPanel {doc} {selection} {viewport} />
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
</div>

<style>
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
