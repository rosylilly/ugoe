<!--
  外部の AI サービスに送る前の確認 (docs/editor.md#ai-の設定と送信の同意)。App に 1 つ置き、
  `consentPrompt.ask` (lib/ai/ai.svelte.ts) の問い合わせがあれば開く。送り先・モデル・画像の枚数と
  バイト数・テキストのバイト数 (利用者の注文の文字数と、答えが検証を通らなければ送り直す回数の上限も) と、送り先の規約へのリンク
  (知っている送り先だけ。lib/ai/consent.ts の `termsFor`) を示す。
  Esc と「Cancel」は送らない。
-->
<script lang="ts">
import { openExternal } from "../help";
import type { ConsentPrompt } from "./ai.svelte";
import { formatBytes, termsFor } from "./consent";

interface Props {
  prompt: ConsentPrompt;
}

let { prompt }: Props = $props();

let dialog = $state<HTMLDialogElement>();
let remember = $state(false);
let linkError = $state<string | null>(null);

$effect(() => {
  if (!dialog) return;
  if (prompt.pending && !dialog.open) {
    remember = false;
    linkError = null;
    dialog.showModal();
  } else if (!prompt.pending && dialog.open) {
    dialog.close();
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
    linkError = `Could not open the browser (${e}): ${url}`;
  }
}
</script>

<dialog
  bind:this={dialog}
  class="consent"
  aria-labelledby="consent-title"
  oncancel={(e) => {
    e.preventDefault();
    answer(false);
  }}
>
  {#if prompt.pending}
    {@const d = prompt.pending.disclosure}
    {@const terms = termsFor(d.endpoint)}
    <h2 id="consent-title">Send to {d.service}?</h2>
    <p>This project's data leaves your computer and is sent to an external AI service.</p>
    <dl>
      <dt>Destination</dt>
      <dd><code>{d.endpoint}</code></dd>
      <dt>Model</dt>
      <dd><code>{d.model}</code></dd>
      <dt>Images</dt>
      <dd>{d.image_count} ({formatBytes(d.image_bytes)})</dd>
      <dt>Text</dt>
      <dd>{formatBytes(d.text_bytes)}</dd>
      {#if d.instruction_chars > 0}
        <dt>Your instructions</dt>
        <dd>{d.instruction_chars} {d.instruction_chars === 1 ? "character" : "characters"} (included in the text)</dd>
      {/if}
    </dl>
    {#if d.retries > 0}
      <p class="muted">
        If an answer fails validation, the same data is sent again with the previous answer and the reason, up to
        {d.retries} more {d.retries === 1 ? "time" : "times"}.
      </p>
    {/if}
    <p class="muted">
      {#if terms}
        {terms.org} handles the data under its terms:
        {#each terms.links as term, i}
          {#if i > 0}{" · "}{/if}
          <a href={term.url} onclick={(e) => (e.preventDefault(), openTerms(term.url))}>{term.label}</a>
        {/each}
      {:else}
        Check how this service handles your data before sending.
      {/if}
    </p>
    {#if linkError}<p class="error">{linkError}</p>{/if}
    <label class="check">
      <input type="checkbox" bind:checked={remember} />
      Don't ask again for this project until the editor is closed (same service and model)
    </label>
    <div class="actions">
      <button onclick={() => answer(false)}>Cancel</button>
      <button class="primary" onclick={() => answer(true)}>Send</button>
    </div>
  {/if}
</dialog>

<style>
  .consent {
    max-width: 480px;
    padding: 16px 20px;
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border);
  }
  .consent::backdrop {
    background: rgb(0 0 0 / 40%);
  }
  h2 {
    margin: 0 0 8px;
    font-size: 15px;
    text-transform: none;
    letter-spacing: normal;
    color: var(--text);
  }
  .check {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 4px 12px;
    margin: 12px 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  .primary {
    color: var(--accent);
    border-color: var(--accent);
  }
</style>
