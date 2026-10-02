<!--
  選んだ衣装のメッシュが、本体の変形を写したリグの提案を採用した後に形 (頂点・三角形・親) を編集したものなら、
  提案し直すよう知らせる (docs/editor.md#モジュールのプロジェクトでの-ai-の提案)。一覧は AI タブと同じ
  (`ProposalState.stale_rigs`。プロジェクトに残る)。
-->
<script lang="ts">
import { Button, Notice } from "@ugoe/ui";
import { t } from "../../i18n";
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
  <div class="stale">
    <Notice tone="warning" title={t("inspector.staleRig.title")}>
      {t("inspector.staleRig.description")}
      {#snippet actions()}
        <Button small onclick={onopen}>{t("inspector.staleRig.openAi")}</Button>
      {/snippet}
    </Notice>
  </div>
{/if}

<style>
.stale {
  margin: var(--ugoe-space-2) 0 var(--ugoe-space-3);
  overflow-wrap: anywhere;
}
</style>
