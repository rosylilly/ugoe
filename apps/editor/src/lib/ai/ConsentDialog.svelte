<!--
  外部の AI サービスに送る前の確認 (docs/editor.md#ai-の設定と送信の同意)。App に 1 つ置き、
  `consentPrompt.ask` (lib/ai/ai.svelte.ts) の問い合わせがあれば開く。送り先・モデル・画像の枚数と
  バイト数・テキストのバイト数 (利用者の注文の文字数と、答えが検証を通らなければ送り直す回数の上限も) と、送り先の規約へのリンク
  (知っている送り先だけ。lib/ai/consent.ts の `termsFor`) を示す。
  送ると取り消せないので @ugoe/ui の AlertDialog (開くとフォーカスは「Cancel」)。Esc と「Cancel」は送らない。
  設定のダイアログ (SettingsDialog の「Test connection」) の上に重なって開くこともある。Bits UI のフォーカスと Esc の層は
  重なった順に積まれるので、Esc で閉じるのはこの確認だけで、閉じるとフォーカスは設定のダイアログに戻る。
-->
<script lang="ts">
import { AlertDialog, Checkbox } from "@ugoe/ui";
import { errorText } from "../errors";
import { openExternal } from "../help";
import { t } from "../i18n";
import type { ConsentPrompt } from "./ai.svelte";
import { formatBytes, termsFor } from "./consent";

interface Props {
  prompt: ConsentPrompt;
}

let { prompt }: Props = $props();

let remember = $state(false);
let linkError = $state<string | null>(null);

const disclosure = $derived(prompt.pending?.disclosure ?? null);
const terms = $derived(disclosure ? termsFor(disclosure.endpoint) : null);

// 問い合わせが来るたびに選択を戻す。
$effect(() => {
  if (prompt.pending) {
    remember = false;
    linkError = null;
  }
});

function answer(send: boolean): void {
  prompt.answer(send, send && remember);
}

async function openTerms(url: string): Promise<void> {
  try {
    await openExternal(url);
    linkError = null;
  } catch (e) {
    linkError = t("consent.linkError", { error: errorText(e), url });
  }
}
</script>

<AlertDialog
  open={prompt.pending !== null}
  title={disclosure ? t("consent.title", { service: disclosure.service }) : ""}
  description={t("consent.description")}
  confirmLabel={t("consent.send.label")}
  cancelLabel={t("consent.cancel.label")}
  danger={false}
  onconfirm={() => answer(true)}
  oncancel={() => answer(false)}
>
  {#if disclosure}
    {@const d = disclosure}
    <dl>
      <dt>{t("consent.destination")}</dt>
      <dd><code>{d.endpoint}</code></dd>
      <dt>{t("consent.model")}</dt>
      <dd><code>{d.model}</code></dd>
      <dt>{t("consent.images.label")}</dt>
      <dd>{t("consent.images.value", { count: d.image_count, size: formatBytes(d.image_bytes) })}</dd>
      <dt>{t("consent.text")}</dt>
      <dd>{formatBytes(d.text_bytes)}</dd>
      {#if d.instruction_chars > 0}
        <dt>{t("consent.instructions.label")}</dt>
        <dd>{t("consent.instructions.value", { count: d.instruction_chars })}</dd>
      {/if}
    </dl>
    {#if d.retries > 0}
      <p class="ed-hint">{t("consent.retries", { count: d.retries })}</p>
    {/if}
    <p class="ed-hint">
      {#if terms}
        {t("consent.terms.intro", { org: terms.org })}
        {#each terms.links as term, i (term.url)}
          {#if i > 0}{" · "}{/if}
          <a href={term.url} onclick={(e) => (e.preventDefault(), openTerms(term.url))}>{term.label}</a>
        {/each}
      {:else}
        {t("consent.terms.unknown")}
      {/if}
    </p>
    {#if linkError}<p class="error ugoe-selectable" role="alert">{linkError}</p>{/if}
    <Checkbox bind:checked={remember} label={t("consent.remember.label")} />
  {/if}
</AlertDialog>

<style>
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--ugoe-space-2) var(--ugoe-space-4);
    margin: 0 0 var(--ugoe-space-3);
  }
  dt {
    color: var(--ugoe-color-text-muted);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  p {
    margin: 0 0 var(--ugoe-space-3);
  }
  a {
    color: var(--ugoe-color-accent);
  }
  .error {
    overflow-wrap: anywhere;
  }
</style>
