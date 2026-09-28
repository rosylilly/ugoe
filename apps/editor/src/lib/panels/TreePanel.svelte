<!--
  左パネル: パーツ / デフォーマのツリー。T0602 で実装する。
  選択は Selection (lib/selection.svelte.ts)、編集は EditorDocument.apply(Command) で行う。
-->
<script lang="ts">
import type { EditorDocument } from "../document.svelte";
import type { Selection } from "../selection.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
}

let { doc, selection }: Props = $props();
</script>

<h2>Parts</h2>
<p class="muted">TODO(T0602): パーツ / デフォーマツリー ({selection.items.length} selected, rev {doc.revision})</p>
