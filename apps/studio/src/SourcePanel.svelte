<!--
  ブラウザソース (docs/studio.md#ブラウザソース、T0808): OBS のブラウザソースに入れる URL を出すスイッチ・URL のコピー・
  作り直し・OBS での設定の手順 (大きさと透過)。既定は切で、操作 API と同じ待ち受け (ループバックだけ) を使う。
  URL はトークンを含むので、画面ではトークンを伏せ、コピーするときだけバックエンドから取る。
-->
<script lang="ts">
import { AlertDialog, Button, Notice, Toggle } from "@ugoe/ui";
import { type ApiView, maskSourceUrl } from "./lib/api";
import { t } from "./lib/i18n";

interface Props {
  api: ApiView;
  /** 案内する大きさ (出力ウィンドウの大きさのプリセット。自由なら `null`)。 */
  size: { width: number; height: number } | null;
  disabled?: boolean;
  onenabled: (enabled: boolean) => void;
  /** URL を取る (コピーするため)。 */
  url: () => Promise<string>;
  /** トークンを作り直し、新しい URL を返す。 */
  onregenerate: () => Promise<string | null>;
}

let { api, size, disabled = false, onenabled, url, onregenerate }: Props = $props();

let copied = $state<"copied" | "failed" | null>(null);
let confirmRegenerate = $state(false);
/** 画面に出す (トークンを伏せた) URL。 */
let shown = $state<string | null>(null);

const serving = $derived(api.source.enabled && api.source.listening);

$effect(() => {
  // 出し始めたとき・ポートが変わったときに URL を取り直す。
  void api.port;
  if (!serving) {
    shown = null;
    return;
  }
  void url()
    .then((u) => (shown = maskSourceUrl(u)))
    .catch(() => (shown = null));
});

async function copy() {
  try {
    await navigator.clipboard.writeText(await url());
    copied = "copied";
  } catch {
    copied = "failed";
  }
}
</script>

<div class="source">
  <Toggle label={t("source.enable")} checked={api.source.enabled} {disabled} onchange={onenabled} />
  <p class="source__hint">{t("source.hint")}</p>

  {#if api.source.enabled && api.error !== null}
    <Notice tone="danger">{t("source.error", { message: api.error })}</Notice>
  {/if}

  {#if serving}
    <div class="source__row">
      <span class="source__label">{t("source.url")}</span>
      <code class="source__value">{shown ?? ""}</code>
    </div>
    <div class="source__row">
      <Button small {disabled} onclick={copy}>{t("source.copy")}</Button>
      <Button small {disabled} onclick={() => (confirmRegenerate = true)}>{t("source.regenerate")}</Button>
    </div>
    {#if copied !== null}
      <p class="source__hint" role="status">
        {copied === "copied" ? t("source.copied") : t("source.copyFailed")}
      </p>
    {/if}
    <p class="source__hint">{t("source.urlHint")}</p>
    <p class="source__hint">{t("source.connections", { count: api.source.connections })}</p>
    <p class="source__hint">{t("source.port", { port: api.port })}</p>

    <h3 class="source__heading">{t("source.stepsHeading")}</h3>
    <ol class="source__steps">
      <li>{t("source.step1")}</li>
      <li>{t("source.step2")}</li>
      <li>
        {size === null
          ? t("source.step3Free")
          : t("source.step3", { width: size.width, height: size.height })}
      </li>
      <li>{t("source.step4")}</li>
      <li>{t("source.step5")}</li>
    </ol>
  {/if}
</div>

<AlertDialog
  open={confirmRegenerate}
  title={t("source.regenerateTitle")}
  description={t("source.regenerateBody")}
  confirmLabel={t("source.regenerate")}
  onconfirm={() => {
    confirmRegenerate = false;
    copied = null;
    void onregenerate().then((u) => {
      if (u !== null) shown = maskSourceUrl(u);
    });
  }}
  oncancel={() => (confirmRegenerate = false)}
/>

<style>
  .source {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .source__row {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
    flex-wrap: wrap;
  }

  .source__label {
    color: var(--ugoe-color-text-muted);
  }

  .source__value {
    font-family: var(--ugoe-font-mono);
    font-size: var(--ugoe-font-size-sm);
    overflow-wrap: anywhere;
  }

  .source__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }

  .source__heading {
    margin: var(--ugoe-space-2) 0 0;
    font-size: var(--ugoe-font-size-md);
    font-weight: var(--ugoe-font-weight-strong);
  }

  .source__steps {
    margin: 0;
    padding-inline-start: var(--ugoe-space-5);
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-1);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
