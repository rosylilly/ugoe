<!--
  右パネル: 選択中の要素のプロパティとキーフォーム。
  プロパティは T0606 / T0607、キーの格子とキーフォームの編集は T0608、推定の操作は T0613 で追加する。
  それぞれ lib/panels/inspector/ にコンポーネントを置き、ここではそれを並べるだけにする。
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

<h2>Inspector</h2>
{#if selection.primary}
  <p>{selection.primary.kind}: {selection.primary.id}</p>
{:else}
  <p class="muted">Nothing selected (rev {doc.revision})</p>
{/if}
