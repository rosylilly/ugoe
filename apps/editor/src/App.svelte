<!--
  画面のレイアウトだけを持つ。状態は lib/ の EditorDocument (ドキュメント) と
  ViewportController (表示) に、各領域の中身は lib/panels/ と lib/viewport/ に置く。
-->
<script lang="ts">
import { EditorDocument } from "./lib/document.svelte";
import { EMPTY_VIEWPORT_HINT } from "./lib/emptyState";
import FileMenu from "./lib/FileMenu.svelte";
import { type AppInfo, ipc } from "./lib/ipc";
import InspectorPanel from "./lib/panels/InspectorPanel.svelte";
import ParametersPanel from "./lib/panels/ParametersPanel.svelte";
import TreePanel from "./lib/panels/TreePanel.svelte";
import { Selection } from "./lib/selection.svelte";
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

// ドキュメントを読み直すたびに、プロジェクト名と未保存の印も取り直す。
$effect(() => {
  if (doc.revision > 0) fileMenu?.refresh();
});
</script>

<div class="shell">
  <header>
    <strong>ugoe</strong>
    <span class="muted">{info ? `v${info.version}` : "…"}</span>
    <!-- 別のモデルになったら (新規・開く・取り込み) キャンバス全体を表示し直す。 -->
    <FileMenu
      bind:this={fileMenu}
      onchange={() => {
        viewport.fit();
        doc.refresh();
      }}
    />
    <span class="spacer"></span>
    <button onclick={() => doc.undo()}>Undo</button>
    <button onclick={() => doc.redo()}>Redo</button>
  </header>
  <aside class="left">
    <TreePanel {doc} {selection} />
  </aside>
  <main class="center">
    <Viewport controller={viewport} model={doc.model} hint={doc.empty ? EMPTY_VIEWPORT_HINT : null}>
      {#snippet overlay(controller)}
        <ToolOverlay {controller} {doc} {selection} />
      {/snippet}
    </Viewport>
  </main>
  <aside class="right">
    {#if doc.error}<p class="error">{doc.error}</p>{/if}
    <InspectorPanel {doc} {selection} {viewport} />
    <ParametersPanel parameters={doc.parameters} {viewport} {doc} {selection} />
  </aside>
</div>
