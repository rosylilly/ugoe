<!--
  ヘッダーの「Settings」。「General」(画面の言語 (lib/LanguageSetting.svelte)・テーマ・Undo の上限 (lib/UndoLimitSetting.svelte))・「AI provider」と
  「Connect agents」(lib/agents/ConnectAgents.svelte。エディタの中の MCP サーバ) の 3 つのタブ。
  「AI provider」は AI の設定 (docs/editor.md#ai-の設定と送信の同意):
  プロバイダ (Claude / OpenAI 互換) とモデル・ベースの URL の選択 (エディタ全体の設定)、選んだプロバイダの
  API キーの保存・削除 (OS の資格情報の保存先。キーは画面に出さず、入力欄も保存したら空にする)、
  開いているプロジェクトの「外部の AI に送らない」、「今後は確かめない」の取り消し、接続の確認 (確かめている間は取り消せる)。
  1 回の提案の時間の上限 (分。プロバイダに関わらない。docs/ai-integration.md#取り消しと時間の上限) もプロバイダの設定と一緒に適用する。
  @ugoe/ui の Dialog。接続の確認では、外部の AI に送る前の確認 (ConsentDialog。AlertDialog) がこの上に重なって開く
  (Bits UI のフォーカスと Esc の層は重なった順に積まれるので、確認を閉じるとこのダイアログに戻る)。
-->
<script lang="ts">
import { RichText } from "@ugoe/i18n";
import { Button, Dialog, type Option, Select, ThemeSetting, Toggle } from "@ugoe/ui";
import ConnectAgents from "../agents/ConnectAgents.svelte";
import type { EditorDocument } from "../document.svelte";
import { errorCode, errorText } from "../errors";
import { t } from "../i18n";
import { type AiSettings, ipc, type OutputMode, type ProviderSettings } from "../ipc";
import LanguageSetting from "../LanguageSetting.svelte";
import UndoLimitSetting from "../UndoLimitSetting.svelte";
import TabStrip, { type TabItem } from "../ui/TabStrip.svelte";
import TipButton from "../ui/TipButton.svelte";
import { aiSettings, consentPrompt } from "./ai.svelte";
import { withConsent } from "./consent";
import { OPENAI_PRESETS, OUTPUT_MODES, presetFor } from "./providers";

interface Props {
  /** 開いているドキュメント (Undo の上限を当てる)。 */
  doc: EditorDocument;
  /** プロジェクトの設定を変えたとき (未保存の印を取り直すため)。 */
  onchange: () => void;
}

let { doc, onchange }: Props = $props();

let isOpen = $state(false);
let keyInput = $state<HTMLInputElement>();
/** 入力欄が空でないか (キーそのものは状態に持たない)。 */
let hasKeyInput = $state(false);
let busy = $state(false);
/**
 * 接続を確かめている間。このボタンだけは無効にしない (無効にするとフォーカスが外れ、重ねて開く確認のダイアログ (ConsentDialog) を
 * 閉じたときに Bits UI がフォーカスを戻す先を失う)。押し直しは testConnection で無視する。
 */
let testing = $state(false);
/** 接続の確認の結果。 */
let testResult = $state<{ ok: boolean; text: string } | null>(null);
/** 編集中のプロバイダの設定 (「Apply」で保存する)。 */
let draft = $state<ProviderSettings | null>(null);
/** 開いているタブ。 */
let tab = $state<SettingsTab>("general");
const id = $props.id();

type SettingsTab = "general" | "ai" | "agents";

const tabs = $derived<TabItem<SettingsTab>[]>([
  { value: "general", label: t("settings.general.tab") },
  { value: "ai", label: t("settings.ai.tab") },
  { value: "agents", label: t("settings.agents.tab") },
]);

const saved = $derived(aiSettings.value?.provider ?? null);
/** 時間の上限の入力が整数か (空や小数なら適用しない。範囲はバックエンドが確かめる)。 */
const timeLimitValid = $derived(!!draft && Number.isInteger(draft.time_limit_minutes));
const dirty = $derived(!!draft && !!saved && JSON.stringify(draft) !== JSON.stringify(saved));
const preset = $derived(draft ? presetFor(draft.openai.base_url) : undefined);
/** OpenAI 互換の送り先の選択肢 (値はベースの URL)。どれにも合わなければ「Other」。 */
const OTHER = "other";
const serviceOptions = $derived<Option[]>([
  ...OPENAI_PRESETS.map((p) => ({ value: p.base_url, label: p.label })),
  { value: OTHER, label: t("settings.ai.service.other"), disabled: true },
]);
const outputOptions = $derived<Option<OutputMode>[]>(
  OUTPUT_MODES.map((m) => ({ value: m.value, label: m.label })),
);

/** 開く。`ai` なら「AI provider」、`agents` なら「Connect agents」のタブを開く。 */
export async function open(which: "general" | "ai" | "agents" = "general"): Promise<void> {
  tab = which;
  testResult = null;
  isOpen = true;
  await aiSettings.refresh();
  resetDraft();
}

function resetDraft(): void {
  draft = aiSettings.value ? structuredClone($state.snapshot(aiSettings.value.provider)) : null;
}

function close(): void {
  clearKeyInput();
  isOpen = false;
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

function choosePreset(baseUrl: string): void {
  const p = OPENAI_PRESETS.find((x) => x.base_url === baseUrl);
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
  if (testing) return;
  testing = true;
  busy = true;
  testResult = null;
  try {
    const result = await withConsent(ipc.testAiConnection, consentPrompt.ask);
    testResult = result
      ? { ok: true, text: t("settings.ai.test.connected") }
      : { ok: false, text: t("settings.ai.test.notSent") };
  } catch (e) {
    testResult =
      errorCode(e) === "ai.cancelled"
        ? { ok: true, text: t("settings.ai.test.cancelled") }
        : { ok: false, text: errorText(e) };
  } finally {
    busy = false;
    testing = false;
    // 「今後は確かめない」を選んだかもしれない。
    await aiSettings.refresh();
  }
}

function keyStatus(s: AiSettings): string {
  switch (s.key) {
    case "stored":
      return t("settings.ai.key.stored", { service: s.service });
    case "not_stored":
      return s.key_required
        ? t("settings.ai.key.notStored", { service: s.service })
        : t("settings.ai.key.notStoredOptional", { service: s.service });
    case "unavailable":
      return s.key_env
        ? t("settings.ai.key.unavailableEnv", { env: s.key_env })
        : t("settings.ai.key.unavailable");
  }
}

function keyPlaceholder(s: AiSettings): string {
  if (s.key === "stored") return t("settings.ai.key.replace");
  return s.key_required
    ? t("settings.ai.key.placeholder")
    : t("settings.ai.key.placeholderOptional");
}
</script>

<Dialog bind:open={isOpen} title={t("settings.title")} size="lg" closeLabel={t("settings.close.label")} onclose={clearKeyInput}>
  <div class="tabs">
    <TabStrip {tabs} value={tab} ariaLabel={t("settings.tabs.aria")} onselect={(v) => (tab = v)} />
  </div>
  {#if tab === "general"}
    <section>
      <h3>{t("settings.general.tab")}</h3>
      <LanguageSetting />
      <div class="theme"><ThemeSetting /></div>
      <div class="undo-limit"><UndoLimitSetting {doc} /></div>
    </section>
  {:else if tab === "agents"}
    <ConnectAgents />
  {:else}
  <section>
    <h3>{t("settings.ai.tab")}</h3>
    {#if aiSettings.error}<p class="error ugoe-selectable">{aiSettings.error}</p>{/if}
    {#if draft}
      <form class="provider" onsubmit={applyProvider}>
        <div class="choices" role="radiogroup" aria-label={t("settings.ai.provider.aria")}>
          <label class="check">
            <input type="radio" name="provider" value="claude" bind:group={draft.provider} disabled={busy} />
            {t("settings.ai.provider.claude")}
          </label>
          <label class="check">
            <input
              type="radio"
              name="provider"
              value="openai_compatible"
              bind:group={draft.provider}
              disabled={busy}
            />
            {t("settings.ai.provider.openai")}
          </label>
        </div>
        {#if draft.provider === "claude"}
          <div class="ed-field">
            <label for={`${id}-claude-model`}>{t("settings.ai.model.label")}</label>
            <!-- i18n-ignore: モデルの ID の例 -->
            <input id={`${id}-claude-model`} type="text" bind:value={draft.claude.model} spellcheck="false" placeholder="claude-opus-5-5" />
          </div>
        {:else}
          <div class="ed-field">
            <span id={`${id}-service`}>{t("settings.ai.service.label")}</span>
            <Select
              aria-labelledby={`${id}-service`}
              value={preset?.base_url ?? OTHER}
              options={serviceOptions}
              onchange={choosePreset}
              disabled={busy}
            />
          </div>
          <div class="ed-field">
            <label for={`${id}-base-url`}>{t("settings.ai.baseUrl.label")}</label>
            <!-- i18n-ignore: URL の例 -->
            <input id={`${id}-base-url`} type="text" bind:value={draft.openai.base_url} spellcheck="false" placeholder="https://api.example.com/v1"
              aria-describedby={`${id}-base-url-note`} />
          </div>
          <p class="ed-hint note" id={`${id}-base-url-note`}>
            <RichText text={t("settings.ai.baseUrl.description")}>
              {#snippet code(text: string)}<code>{text}</code>{/snippet}
            </RichText>
          </p>
          <div class="ed-field">
            <label for={`${id}-openai-model`}>{t("settings.ai.model.label")}</label>
            <input
              id={`${id}-openai-model`}
              type="text"
              bind:value={draft.openai.model}
              spellcheck="false"
              placeholder={preset?.placeholder ?? t("settings.ai.model.placeholder")}
            />
          </div>
          <div class="ed-field">
            <span id={`${id}-output`}>{t("settings.ai.output.label")}</span>
            <Select
              aria-labelledby={`${id}-output`}
              value={draft.openai.output}
              options={outputOptions}
              onchange={(v) => draft && (draft.openai.output = v)}
              disabled={busy}
            />
          </div>
          <p class="ed-hint note">{t("settings.ai.output.description")}</p>
        {/if}
        <div class="ed-field">
          <label for={`${id}-time-limit`}>{t("settings.ai.timeLimit.label")}</label>
          <input
            id={`${id}-time-limit`}
            type="number"
            min="1"
            max="120"
            step="1"
            bind:value={draft.time_limit_minutes}
            aria-describedby={`${id}-time-limit-note`}
          />
        </div>
        <p class="ed-hint note" id={`${id}-time-limit-note`}>{t("settings.ai.timeLimit.description")}</p>
        <div class="ed-buttons">
          <Button type="submit" disabled={busy || !dirty || !timeLimitValid}>{t("settings.ai.apply.label")}</Button>
          {#if dirty}
            <Button disabled={busy} onclick={resetDraft}>{t("settings.ai.revert.label")}</Button>
            <span class="ed-hint">{t("settings.ai.notApplied")}</span>
          {/if}
        </div>
      </form>
    {:else if !aiSettings.error}
      <p class="ed-hint">{t("settings.ai.loading")}</p>
    {/if}

    {#if aiSettings.value}
      {@const s = aiSettings.value}
      <h3>{t("settings.ai.key.title", { service: s.service })}</h3>
      <p data-key-status={s.key}>{keyStatus(s)}</p>
      {#if s.env_key && s.key_env}
        <p class="ed-hint">
          {s.key === "stored"
            ? t("settings.ai.key.envAlso", { env: s.key_env })
            : t("settings.ai.key.envUsed", { env: s.key_env })}
        </p>
      {/if}
      {#if s.key !== "unavailable"}
        <form class="row key" onsubmit={saveKey}>
          <input
            bind:this={keyInput}
            type="password"
            autocomplete="off"
            spellcheck="false"
            placeholder={keyPlaceholder(s)}
            aria-label={t("settings.ai.key.aria", { service: s.service })}
            oninput={(e) => (hasKeyInput = e.currentTarget.value.trim() !== "")}
          />
          <Button type="submit" disabled={busy || !hasKeyInput || dirty}>{t("settings.ai.key.save.label")}</Button>
          {#if s.key === "stored"}
            <Button variant="danger" disabled={busy || dirty} onclick={() => run(ipc.deleteApiKey)}>
              {t("settings.ai.key.delete.label")}
            </Button>
          {/if}
        </form>
        {#if dirty}<p class="ed-hint note">{t("settings.ai.key.applyFirst")}</p>{/if}
      {/if}

      <h3>{t("settings.ai.project.title")}</h3>
      <Toggle
        checked={!s.send_externally}
        disabled={busy}
        label={t("settings.ai.project.dontSend.label")}
        onchange={(on) => setSendExternally(!on)}
      />
      <p class="ed-hint">
        {s.send_externally ? t("settings.ai.project.asked") : t("settings.ai.project.off")}
        {t("settings.ai.project.saved")}
      </p>
      {#if s.local}
        <p class="ed-hint" data-local>{t("settings.ai.project.local")}</p>
      {/if}
      {#if s.consent_remembered}
        <div class="ed-buttons">
          <span>{t("settings.ai.consent.remembered")}</span>
          <Button disabled={busy} onclick={() => run(ipc.forgetAiConsent)}>{t("settings.ai.consent.askAgain.label")}</Button>
        </div>
      {/if}
      <div class="ed-buttons test">
        <TipButton
          disabled={(busy && !testing) || dirty || !aiSettings.canSend}
          onclick={testConnection}
          tooltip={t("settings.ai.test.tooltip")}
        >{t("settings.ai.test.label")}</TipButton>
        {#if testing}
          <Button onclick={() => ipc.cancelAiRequests().catch(() => {})}>{t("settings.ai.test.cancel")}</Button>
        {/if}
        {#if testResult}
          <span class:error={!testResult.ok} role="status">{testResult.text}</span>
        {/if}
      </div>
    {/if}
  </section>
  {/if}
  {#snippet actions()}
    <Button onclick={close}>{t("settings.close.label")}</Button>
  {/snippet}
</Dialog>

<style>
  .tabs {
    margin-bottom: var(--ugoe-space-1);
  }
  section > :global(h3:first-child) {
    margin-top: 0;
  }
  p {
    margin: var(--ugoe-space-2) 0;
  }
  .check {
    display: flex;
    align-items: flex-start;
    gap: var(--ugoe-space-2);
  }
  .check input {
    accent-color: var(--ugoe-color-accent);
  }
  .choices {
    display: grid;
    gap: var(--ugoe-space-2);
    margin-bottom: var(--ugoe-space-3);
  }
  .ed-field input {
    min-width: 0;
  }
  .note {
    margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-3);
  }
  /* API キーの欄 (app.css の .ugoe-dialog input[type="text"] と同じ見た目)。 */
  .key input {
    flex: 1;
    min-width: 0;
    min-height: var(--ugoe-control-height);
    padding: 0 var(--ugoe-space-2);
    border: 1px solid var(--ugoe-color-control-border);
    border-radius: var(--ugoe-radius-md);
    background: var(--ugoe-color-surface);
    color: var(--ugoe-color-text);
    font: inherit;
  }
  .key input:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: 1px;
  }
  .test {
    margin-top: var(--ugoe-space-3);
  }
  .error {
    color: var(--ugoe-color-danger);
    overflow-wrap: anywhere;
  }
  .theme,
  .undo-limit {
    margin-top: var(--ugoe-space-4);
  }
</style>
