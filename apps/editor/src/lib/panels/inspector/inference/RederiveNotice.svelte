<!--
  自動の作り直しで失敗したことの通知 (docs/editor.md#推定とミラー)。ミラー元の頂点を動かしすぎた・対を外したなどで
  ミラーをやり直せなかった格子は、コマンドは適用されたままミラーのセルが前の値で残るので、ここで知らせる。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import { describeWarning } from "./inference";

interface Props {
  doc: EditorDocument;
}

let { doc }: Props = $props();
</script>

{#if doc.warnings.length > 0}
  <div class="notice" role="status">
    <ul>
      {#each doc.warnings as warning (`${warning.target.kind}:${warning.target.id}<-${warning.source.kind}:${warning.source.id}`)}
        <li>{describeWarning(warning)}</li>
      {/each}
    </ul>
    <p class="muted">Run “Mirror” on the grid to retry, or fix the source.</p>
    <button onclick={() => doc.dismissWarnings()}>Dismiss</button>
  </div>
{/if}

<style>
.notice {
  margin: 4px 0 8px;
  padding: 6px 8px;
  border: 1px solid #d97706;
  border-radius: 4px;
  background: color-mix(in srgb, #d97706 12%, transparent);
}

ul {
  margin: 0;
  padding-left: 16px;
}

p {
  margin: 4px 0;
  font-size: 11px;
}
</style>
