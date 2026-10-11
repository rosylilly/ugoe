<!--
  「Open base project」で本体を開いている間の、モジュールに戻る帯 (docs/editor.md#モジュールに戻る、T1622)。
  FileMenu がヘッダーの下の知らせの帯に出す。「Return to module」でモジュールのプロジェクトを開き直す (保存した本体の上に
  組み直し、設定の画面で拒否された設定なら適用し直す)。本体を保存していなければ、先に保存するよう添える
  (押すと捨てるかを確かめる)。閉じると戻らずに本体の編集を続ける。
-->
<script lang="ts">
import { Notice } from "@ugoe/ui";
import type { ModuleReturn } from "./fileMenuFlow.svelte";
import { t } from "./i18n";
import TipButton from "./ui/TipButton.svelte";

interface Props {
  back: ModuleReturn;
  /** 本体に保存していない変更があるか。 */
  modified: boolean;
  busy: boolean;
  onreturn: () => void;
  ondismiss: () => void;
}

let { back, modified, busy, onreturn, ondismiss }: Props = $props();
</script>

<div class="return">
  <Notice tone="info" title={t("moduleReturn.title", { name: back.name })} {ondismiss}
    dismissLabel={t("moduleReturn.dismiss")}>
    <p>{back.settings ? t("moduleReturn.hintSettings") : t("moduleReturn.hint")}</p>
    {#if modified}
      <p class="warn">{t("moduleReturn.unsaved")}</p>
    {/if}
    {#snippet actions()}
      <TipButton
        variant="primary"
        disabled={busy}
        tooltip={t("moduleReturn.return.tooltip", { path: back.project })}
        onclick={onreturn}>{t("moduleReturn.return.label")}</TipButton
      >
    {/snippet}
  </Notice>
</div>

<style>
  /* 置き場所は FileMenu の知らせの帯 (.bands)。地を敷いて、下のビューポートが透けないようにする。 */
  .return {
    border-radius: var(--ugoe-radius-lg);
    background: var(--ugoe-color-surface);
  }
  p {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .warn {
    color: var(--ugoe-color-warning);
  }
</style>
