<!--
  画面のレイアウトだけを持つ。状態は lib/ の EditorDocument (ドキュメント) と
  ViewportController (表示) に、各領域の中身は lib/panels/ と lib/viewport/ に置く。
-->
<script lang="ts">
import { EditorDocument } from "./lib/document.svelte";
import FileMenu from "./lib/FileMenu.svelte";
import { type AppInfo, ipc } from "./lib/ipc";
import ParametersPanel from "./lib/panels/ParametersPanel.svelte";
import { ViewportController } from "./lib/viewport/controller.svelte";
import Viewport from "./lib/viewport/Viewport.svelte";

const doc = new EditorDocument();
const viewport = new ViewportController();
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
    <FileMenu bind:this={fileMenu} onchange={() => doc.refresh()} />
    <span class="spacer"></span>
    <button onclick={() => doc.undo()}>Undo</button>
    <button onclick={() => doc.redo()}>Redo</button>
  </header>
  <aside class="left">
    <h2>Parts</h2>
    <p class="muted">TODO(T0602): パーツ / デフォーマツリー</p>
  </aside>
  <main class="center">
    <Viewport controller={viewport} model={doc.model} />
  </main>
  <aside class="right">
    {#if doc.error}<p class="error">{doc.error}</p>{/if}
    <ParametersPanel parameters={doc.parameters} {viewport} />
  </aside>
</div>
