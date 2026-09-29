<!--
  ヘッダーの「Settings」。今は AI の設定だけ (docs/editor.md#ai-の設定と送信の同意):
  API キーの保存・削除 (OS の資格情報の保存先。キーは画面に出さず、入力欄も保存したら空にする)、
  開いているプロジェクトの「外部の AI に送らない」、「今後は確かめない」の取り消し、接続の確認。
-->
<script lang="ts">
import { type AiSettings, ipc } from "../ipc";
import { aiSettings, consentPrompt } from "./ai.svelte";
import { withConsent } from "./consent";

interface Props {
  /** プロジェクトの設定を変えたとき (未保存の印を取り直すため)。 */
  onchange: () => void;
}

let { onchange }: Props = $props();

let dialog = $state<HTMLDialogElement>();
let keyInput = $state<HTMLInputElement>();
/** 入力欄が空でないか (キーそのものは状態に持たない)。 */
let hasKeyInput = $state(false);
let busy = $state(false);
/** 接続の確認の結果。 */
let testResult = $state<{ ok: boolean; text: string } | null>(null);

export function open(): void {
  testResult = null;
  aiSettings.refresh();
  dialog?.showModal();
}

function close(): void {
  clearKeyInput();
  dialog?.close();
}

function clearKeyInput(): void {
  if (keyInput) keyInput.value = "";
  hasKeyInput = false;
}

async function run(task: () => Promise<AiSettings>): Promise<boolean> {
  busy = true;
  try {
    return await aiSettings.run(task);
  } finally {
    busy = false;
  }
}

async function saveKey(e: SubmitEvent): Promise<void> {
  e.preventDefault();
  const key = keyInput?.value ?? "";
  clearKeyInput();
  if (key.trim()) await run(() => ipc.storeApiKey(key));
}

async function setSendExternally(allowed: boolean): Promise<void> {
  testResult = null;
  if (await run(() => ipc.setAiSendExternally(allowed))) onchange();
}

async function testConnection(): Promise<void> {
  busy = true;
  testResult = null;
  try {
    const result = await withConsent(ipc.testAiConnection, consentPrompt.ask);
    testResult = result
      ? { ok: true, text: "Connected. The API key works." }
      : { ok: false, text: "Not sent." };
  } catch (e) {
    testResult = { ok: false, text: String(e) };
  } finally {
    busy = false;
    // 「今後は確かめない」を選んだかもしれない。
    await aiSettings.refresh();
  }
}

const KEY_STATUS = {
  stored: "An API key is saved in the OS credential store.",
  not_stored: "No API key is saved.",
  unavailable:
    "The OS credential store is not available here (for example WSL or Linux without a keyring). " +
    "Set the ANTHROPIC_API_KEY environment variable instead.",
} as const;
</script>

<dialog
  bind:this={dialog}
  class="settings"
  aria-labelledby="settings-title"
  oncancel={(e) => {
    e.preventDefault();
    close();
  }}
>
  <h2 id="settings-title">Settings</h2>
  <section>
    <h3>AI (Claude API)</h3>
    {#if aiSettings.error}<p class="error">{aiSettings.error}</p>{/if}
    {#if aiSettings.value}
      {@const s = aiSettings.value}
      <p data-key-status={s.key}>{KEY_STATUS[s.key]}</p>
      {#if s.env_key}
        <p class="muted">
          {s.key === "stored"
            ? "ANTHROPIC_API_KEY is also set; the saved key is used first."
            : "Using the API key from ANTHROPIC_API_KEY."}
        </p>
      {/if}
      {#if s.key !== "unavailable"}
        <form class="row" onsubmit={saveKey}>
          <input
            bind:this={keyInput}
            type="password"
            autocomplete="off"
            spellcheck="false"
            placeholder={s.key === "stored" ? "Replace the saved key" : "sk-ant-…"}
            aria-label="Anthropic API key"
            oninput={(e) => (hasKeyInput = e.currentTarget.value.trim() !== "")}
          />
          <button type="submit" disabled={busy || !hasKeyInput}>Save</button>
          {#if s.key === "stored"}
            <button type="button" disabled={busy} onclick={() => run(ipc.deleteApiKey)}>Delete</button>
          {/if}
        </form>
      {/if}

      <h3>This project</h3>
      <label class="check">
        <input
          type="checkbox"
          checked={!s.send_externally}
          disabled={busy}
          onchange={(e) => setSendExternally(!e.currentTarget.checked)}
        />
        Don't send this project's data to external AI services
      </label>
      <p class="muted">
        {s.send_externally
          ? "You are asked before anything is sent."
          : "AI features that send data are turned off for this project."}
        Saved with the project.
      </p>
      {#if s.consent_remembered}
        <p class="row">
          <span>You chose not to be asked again for this project.</span>
          <button disabled={busy} onclick={() => run(ipc.forgetAiConsent)}>Ask again</button>
        </p>
      {/if}
      <p class="row">
        <button
          disabled={busy || !aiSettings.canSend}
          onclick={testConnection}
          title="Send a small request without images to check the API key"
        >Test connection</button>
        {#if testResult}
          <span class:error={!testResult.ok}>{testResult.text}</span>
        {/if}
      </p>
    {:else if !aiSettings.error}
      <p class="muted">Loading…</p>
    {/if}
  </section>
  <div class="actions">
    <button onclick={close}>Close</button>
  </div>
</dialog>

<style>
  .settings {
    width: min(520px, calc(100vw - 32px));
    padding: 16px 20px;
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border);
  }
  .settings::backdrop {
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
  h3 {
    margin: 16px 0 4px;
    font-size: 13px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row input {
    flex: 1;
    min-width: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 16px;
  }
</style>
