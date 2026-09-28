<!--
  ビューポートの上に重ねる編集ツール (選択枠・頂点・ギズモ) の置き場所。
  メッシュ編集は T0606、デフォーマのギズモは T0607 で、lib/viewport/tools/ にツールを置き、ここで切り替える。
  座標は ViewportController の canvasToScreen / screenToCanvas / hitTest (CSS ピクセル) を使う。
-->
<script lang="ts">
import type { EditorDocument } from "../document.svelte";
import type { Selection } from "../selection.svelte";
import type { ViewportController } from "./controller.svelte";

interface Props {
  controller: ViewportController;
  doc: EditorDocument;
  selection: Selection;
}

let { controller, doc, selection }: Props = $props();
</script>

<!-- TODO(T0606): 選択とメッシュ編集のツール。controller / doc / selection を使う。 -->
