<!--
  自動の作り直しで失敗したことの通知 (docs/editor.md#推定とミラー)。ミラー元の頂点を動かしすぎた・対を外したなどで
  ミラーをやり直せなかった格子は、コマンドは適用されたままミラーのセルが前の値で残るので、ここで知らせる。
-->
<script lang="ts">
import { Notice } from "@ugoe/ui";
import type { EditorDocument } from "../../../document.svelte";
import { t } from "../../../i18n";
import { describeWarning } from "./inference";

interface Props {
  doc: EditorDocument;
}

let { doc }: Props = $props();
</script>

{#if doc.warnings.length > 0}
  <div class="notice">
    <Notice
      tone="warning"
      ondismiss={() => doc.dismissWarnings()}
      dismissLabel={t("inspector.rederive.dismiss")}
    >
      <ul>
        {#each doc.warnings as warning (`${warning.target.kind}:${warning.target.id}<-${warning.source.kind}:${warning.source.id}`)}
          <li>{describeWarning(warning)}</li>
        {/each}
      </ul>
      <p class="ed-hint">{t("inspector.rederive.hint")}</p>
    </Notice>
  </div>
{/if}

<style>
.notice {
  margin: var(--ugoe-space-2) 0 var(--ugoe-space-3);
}

ul {
  margin: 0;
  padding-left: var(--ugoe-space-5);
}

p {
  margin: var(--ugoe-space-2) 0 0;
}
</style>
