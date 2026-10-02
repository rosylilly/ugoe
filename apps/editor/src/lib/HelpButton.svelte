<!--
  ヘッダーの「Help」。操作マニュアルをブラウザで開く (lib/help.ts)。開けなかったときは、
  URL を選んで写せるように出す。マニュアルは日本語だけ (ADR 0029 の 6)。
-->
<script lang="ts">
import { Notice } from "@ugoe/ui";
import { errorText } from "./errors";
import { MANUAL_URL, openExternal } from "./help";
import { t } from "./i18n";
import TipButton from "./ui/TipButton.svelte";

/** 開けなかったときの理由 (null なら出さない)。 */
let failure = $state<string | null>(null);

async function open() {
  try {
    await openExternal(MANUAL_URL);
    failure = null;
  } catch (e) {
    failure = errorText(e);
  }
}
</script>

<span class="help">
  <TipButton variant="ghost" tooltip={t("help.open.tooltip")} onclick={open}>{t("help.open.label")}</TipButton>
  {#if failure !== null}
    <div class="popup ugoe-selectable">
      <Notice tone="danger" title={t("help.failed.title")} ondismiss={() => (failure = null)}>
        <p>{t("help.failed.description", { reason: failure })}</p>
        <p><a href={MANUAL_URL} target="_blank" rel="noreferrer">{MANUAL_URL}</a></p>
      </Notice>
    </div>
  {/if}
</span>

<style>
  .help {
    position: relative;
  }
  .popup {
    position: absolute;
    right: 0;
    top: 100%;
    z-index: var(--ugoe-z-popover);
    width: max-content;
    max-width: min(480px, calc(100vw - 32px));
    margin-top: var(--ugoe-space-2);
    white-space: normal;
  }
</style>
