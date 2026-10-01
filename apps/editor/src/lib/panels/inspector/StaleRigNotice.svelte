<!--
  選んだ衣装のメッシュが、本体の変形を写したリグの提案を採用した後に形 (頂点・三角形・親) を編集したものなら、
  提案し直すよう知らせる (docs/editor.md#モジュールのプロジェクトでの-ai-の提案)。一覧は AI タブと同じ
  (`ProposalState.stale_rigs`。プロジェクトに残る)。
-->
<script lang="ts">
import type { Selection } from "../../selection.svelte";

interface Props {
  selection: Selection;
  staleRigs: readonly { mesh: string; name: string }[];
  /** AI タブを開く (提案し直す・知らせを消すのはそこで行う)。 */
  onopen: () => void;
}

let { selection, staleRigs, onopen }: Props = $props();

const stale = $derived(
  selection.primary?.kind === "mesh" && staleRigs.some((s) => s.mesh === selection.primary?.id),
);
</script>

{#if stale}
  <div class="notice" role="status">
    <p>
      <strong>Propose the rig again:</strong> this costume mesh was edited (vertices, triangles or parent) after an
      adopted rig proposal wrote the base's motion into it, so it no longer follows the base exactly.
    </p>
    <button onclick={onopen}>Open AI tab</button>
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

p {
  margin: 0 0 4px;
  font-size: 12px;
  overflow-wrap: anywhere;
}
</style>
