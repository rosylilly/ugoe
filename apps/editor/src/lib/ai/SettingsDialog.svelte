<!--
  ヘッダーの「Settings」。「General」(画面の言語。lib/LanguageSetting.svelte)・「AI provider」と
  「Connect agents」(lib/agents/ConnectAgents.svelte。エディタの中の MCP サーバ) の 3 つのタブ。
  「AI provider」は AI の設定 (docs/editor.md#ai-の設定と送信の同意):
  プロバイダ (Claude / OpenAI 互換) とモデル・ベースの URL の選択 (エディタ全体の設定)、選んだプロバイダの
  API キーの保存・削除 (OS の資格情報の保存先。キーは画面に出さず、入力欄も保存したら空にする)、
  開いているプロジェクトの「外部の AI に送らない」、「今後は確かめない」の取り消し、接続の確認。
-->
<script lang="ts">
import { Button } from "@ugoe/ui";
import ConnectAgents from "../agents/ConnectAgents.svelte";
import { errorText } from "../errors";
import { t } from "../i18n";
import { type AiSettings, ipc, type ProviderSettings } from "../ipc";
import LanguageSetting from "../LanguageSetting.svelte";
import TabStrip, { type TabItem } from "../ui/TabStrip.svelte";
import { aiSettings, consentPrompt } from "./ai.svelte";
import { withConsent } from "./consent";
import { OPENAI_PRESETS, OUTPUT_MODES, presetFor } from "./providers";

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
/** 編集中のプロバイダの設定 (「Apply」で保存する)。 */
let draft = $state<ProviderSettings | null>(null);
/** 開いているタブ。 */
let tab = $state<SettingsTab>("general");

type SettingsTab = "general" | "ai" | "agents";

const tabs = $derived<TabItem<SettingsTab>[]>([
  { value: "general", label: t("settings.general.tab") },
  { value: "ai", label: t("settings.ai.tab") },
  { value: "agents", label: t("settings.agents.tab") },
]);

const saved = $derived(aiSettings.value?.provider ?? null);
const dirty = $derived(!!draft && !!saved && JSON.stringify(draft) !== JSON.stringify(saved));
const preset = $derived(draft ? presetFor(draft.openai.base_url) : undefined);

/** 開く。`ai` なら「AI provider」、`agents` なら「Connect agents」のタブを開く。 */
export async function open(which: "general" | "ai" | "agents" = "general"): Promise<void> {
  tab = which;
  testResult = null;
  dialog?.showModal();
  await aiSettings.refresh();
  resetDraft();
}

function resetDraft(): void {
  draft = aiSettings.value ? structuredClone($state.snapshot(aiSettings.value.provider)) : null;
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

async function applyProvider(e: SubmitEvent): Promise<void> {
  e.preventDefault();
  if (!draft) return;
  const next = $state.snapshot(draft);
  testResult = null;
  clearKeyInput();
  if (await run(() => ipc.setAiProvider(next))) resetDraft();
}

function choosePreset(label: string): void {
  const p = OPENAI_PRESETS.find((x) => x.label === label);
  if (!draft || !p) return;
  draft.openai.base_url = p.base_url;
  draft.openai.model = p.model;
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
      ? { ok: true, text: "Connected. The server answered." }
      : { ok: false, text: "Not sent." };
  } catch (e) {
    testResult = { ok: false, text: errorText(e) };
  } finally {
    busy = false;
    // 「今後は確かめない」を選んだかもしれない。
    await aiSettings.refresh();
  }
}

function keyStatus(s: AiSettings): string {
  switch (s.key) {
    case "stored":
      return `An API key for ${s.service} is saved in the OS credential store.`;
    case "not_stored":
      return s.key_required
        ? `No API key for ${s.service} is saved.`
        : `No API key for ${s.service} is saved (not needed if the server has no authentication).`;
    case "unavailable":
      return (
        "The OS credential store is not available here (for example WSL or Linux without a keyring). " +
        (s.key_env ? `Set the ${s.key_env} environment variable instead.` : "Keys cannot be saved.")
      );
  }
}
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
  <h2 id="settings-title">{t("settings.title")}</h2>
  <div class="tabs">
    <TabStrip {tabs} value={tab} ariaLabel={t("settings.tabs.aria")} onselect={(v) => (tab = v)} />
  </div>
  {#if tab === "general"}
    <section>
      <h3>{t("settings.general.tab")}</h3>
      <LanguageSetting />
    </section>
  {:else if tab === "agents"}
    <ConnectAgents />
  {:else}
  <section>
    <h3>AI provider</h3>
    {#if aiSettings.error}<p class="error">{aiSettings.error}</p>{/if}
    {#if draft}
      <form class="provider" onsubmit={applyProvider}>
        <div class="choices" role="radiogroup" aria-label="AI provider">
          <label class="check">
            <input type="radio" name="provider" value="claude" bind:group={draft.provider} disabled={busy} />
            Anthropic Claude API
          </label>
          <label class="check">
            <input
              type="radio"
              name="provider"
              value="openai_compatible"
              bind:group={draft.provider}
              disabled={busy}
            />
            OpenAI-compatible API (OpenAI, OpenRouter, Gemini, Ollama, LM Studio, vLLM…)
          </label>
        </div>
        {#if draft.provider === "claude"}
          <label class="field">
            <span>Model</span>
            <input
              bind:value={draft.claude.model}
              spellcheck="false"
              placeholder="claude-opus-5-5"
              aria-label="Model"
            />
          </label>
        {:else}
          <label class="field">
            <span>Service</span>
            <select
              aria-label="Service"
              value={preset?.label ?? ""}
              onchange={(e) => choosePreset(e.currentTarget.value)}
              disabled={busy}
            >
              {#each OPENAI_PRESETS as p (p.label)}
                <option value={p.label}>{p.label}</option>
              {/each}
              <option value="" disabled>Other (enter the URL)</option>
            </select>
          </label>
          <label class="field">
            <span>Base URL</span>
            <input
              bind:value={draft.openai.base_url}
              spellcheck="false"
              placeholder="https://api.example.com/v1"
              aria-label="Base URL"
              aria-describedby="base-url-note"
            />
          </label>
          <p class="muted note" id="base-url-note">The address up to <code>/chat/completions</code>.</p>
          <label class="field">
            <span>Model</span>
            <input
              bind:value={draft.openai.model}
              spellcheck="false"
              placeholder={preset?.placeholder ?? "model id (a vision model)"}
              aria-label="Model"
            />
          </label>
          <label class="field">
            <span>JSON output</span>
            <select bind:value={draft.openai.output} disabled={busy} aria-label="JSON output">
              {#each OUTPUT_MODES as mode (mode.value)}
                <option value={mode.value}>{mode.label}</option>
              {/each}
            </select>
          </label>
          <p class="muted note">
            Use structured output when the server supports it. Otherwise answers are checked against the schema
            and asked again, which sends the same data more than once.
          </p>
        {/if}
        <p class="row">
          <button type="submit" disabled={busy || !dirty}>Apply</button>
          {#if dirty}
            <button type="button" disabled={busy} onclick={resetDraft}>Revert</button>
            <span class="muted">Not applied yet.</span>
          {/if}
        </p>
      </form>
    {:else if !aiSettings.error}
      <p class="muted">Loading…</p>
    {/if}

    {#if aiSettings.value}
      {@const s = aiSettings.value}
      <h3>API key for {s.service}</h3>
      <p data-key-status={s.key}>{keyStatus(s)}</p>
      {#if s.env_key && s.key_env}
        <p class="muted">
          {s.key === "stored"
            ? `${s.key_env} is also set; the saved key is used first.`
            : `Using the API key from ${s.key_env}.`}
        </p>
      {/if}
      {#if s.key !== "unavailable"}
        <form class="row" onsubmit={saveKey}>
          <input
            bind:this={keyInput}
            type="password"
            autocomplete="off"
            spellcheck="false"
            placeholder={s.key === "stored" ? "Replace the saved key" : s.key_required ? "API key" : "API key (optional)"}
            aria-label={`API key for ${s.service}`}
            oninput={(e) => (hasKeyInput = e.currentTarget.value.trim() !== "")}
          />
          <button type="submit" disabled={busy || !hasKeyInput || dirty}>Save</button>
          {#if s.key === "stored"}
            <button type="button" disabled={busy || dirty} onclick={() => run(ipc.deleteApiKey)}>Delete</button>
          {/if}
        </form>
        {#if dirty}<p class="muted note">Apply the provider first; keys are saved per provider.</p>{/if}
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
          ? "You are asked before anything is sent to an external service."
          : "AI features that send data to external services are turned off for this project."}
        Saved with the project.
      </p>
      {#if s.local}
        <p class="muted" data-local>
          The selected server is on this computer, so nothing leaves it and you are not asked before sending.
        </p>
      {/if}
      {#if s.consent_remembered}
        <p class="row">
          <span>You chose not to be asked again for this project.</span>
          <button disabled={busy} onclick={() => run(ipc.forgetAiConsent)}>Ask again</button>
        </p>
      {/if}
      <p class="row">
        <button
          disabled={busy || dirty || !aiSettings.canSend}
          onclick={testConnection}
          title="Send a small request without images to check the provider and the API key"
        >Test connection</button>
        {#if testResult}
          <span class:error={!testResult.ok}>{testResult.text}</span>
        {/if}
      </p>
    {/if}
  </section>
  {/if}
  <div class="actions">
    <Button onclick={close}>{t("settings.close.label")}</Button>
  </div>
</dialog>

<style>
  .tabs {
    margin-bottom: var(--ugoe-space-3);
  }
  /* 外枠は @ugoe/ui の Dialog と同じ見た目にそろえる (中で外部の AI に送る前の確認の <dialog> を開くので、T0665 で
     中身と一緒に Dialog へ移すまでは <dialog> のまま)。 */
  .settings {
    width: min(640px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    padding: var(--ugoe-space-5);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-lg);
    background: var(--ugoe-color-surface);
    box-shadow: var(--ugoe-shadow-overlay);
    color: var(--ugoe-color-text);
  }
  .settings::backdrop {
    background: var(--ugoe-color-backdrop);
  }
  h2 {
    margin: 0 0 var(--ugoe-space-3);
    font-size: var(--ugoe-font-size-lg);
    font-weight: var(--ugoe-font-weight-strong);
    color: var(--ugoe-color-text);
  }
  .check {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }
  .choices {
    display: grid;
    gap: 4px;
    margin-bottom: 8px;
  }
  .field {
    display: grid;
    grid-template-columns: 7em 1fr;
    align-items: center;
    gap: 8px;
    margin: 4px 0;
  }
  .field input,
  .field select {
    min-width: 0;
  }
  .note {
    margin: 2px 0 6px;
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
