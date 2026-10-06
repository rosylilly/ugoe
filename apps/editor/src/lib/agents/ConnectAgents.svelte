<!--
  設定の「Connect agents」(docs/editor.md#エージェントをつなぐ、docs/manual/mcp.md)。
  エディタの中の MCP サーバ (ブリッジ) の有効化・状態 (待ち受けの URL・つながったクライアント・エージェントが使う
  プロジェクトのパス)・ポートとトークン、クライアントごとの登録の手順 (同梱の ugoe-mcp のパスを埋めたもの) を出す。
  Claude Desktop には、同梱の ugoe-mcp を包んだ .mcpb をその場で作り、開く (インストールの画面) か保存する。
  エージェントの適用・保存は、ここでの設定に関わらず確認のダイアログ (ApprovalDialog.svelte) を通る。
-->
<script lang="ts">
import { RichText } from "@ugoe/i18n";
import { Button, Section, TextField, Toggle } from "@ugoe/ui";
import { errorText } from "../errors";
import { fileDialogs } from "../fileDialogs";
import { joinPath } from "../filePaths";
import { openExternal } from "../help";
import { t } from "../i18n";
import { ipc } from "../ipc";
import TabStrip, { type TabItem } from "../ui/TabStrip.svelte";
import { agentBridge } from "./bridge.svelte";
import { connectedAgo, type SetupSection, setupSections } from "./setup";

const MCP_MANUAL_URL = "https://github.com/rosylilly/ugoe/blob/main/docs/manual/mcp.md";

$effect(() => {
  void agentBridge.refresh();
});

const status = $derived(agentBridge.status);
const sections = $derived(status ? setupSections(status) : []);
let chosen = $state<SetupSection["id"]>("claude");
const section = $derived(sections.find((s) => s.id === chosen) ?? sections[0]);
const choices = $derived<TabItem<SetupSection["id"]>[]>(
  sections.map((s) => ({ value: s.id, label: s.title })),
);
/** 写した・写せなかったことの知らせ (手順の番号ごと)。 */
let copied = $state<string | null>(null);
let portInput = $state("");
let now = $state(Date.now());

/** 設定に残したポート。変わったときだけ欄に入れ直す (つながったクライアントの知らせなどで、入力中の値を消さない)。 */
const savedPort = $derived(status?.port ?? null);
$effect(() => {
  portInput = savedPort ? String(savedPort) : "";
});

$effect(() => {
  const timer = setInterval(() => (now = Date.now()), 30_000);
  return () => clearInterval(timer);
});

async function copy(key: string, text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    copied = key;
  } catch {
    copied = `${key}:failed`;
  }
}

async function copyToken(): Promise<void> {
  try {
    await copy("token", await ipc.getMcpBridgeToken());
  } catch (e) {
    agentBridge.error = errorText(e);
  }
}

async function applyPort(e: SubmitEvent): Promise<void> {
  e.preventDefault();
  const text = portInput.trim();
  if (text === "") {
    await agentBridge.setPort(null);
    return;
  }
  const port = Number(text);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    agentBridge.error = t("agents.port.invalid");
    return;
  }
  await agentBridge.setPort(port);
}

/** Claude Desktop の .mcpb を作っている・作った結果。 */
let bundleBusy = $state(false);
let bundle = $state<
  { kind: "opened" | "saved"; path: string } | { kind: "error"; message: string } | null
>(null);

async function installInClaudeDesktop(): Promise<void> {
  bundleBusy = true;
  try {
    bundle = { kind: "opened", path: await ipc.openClaudeDesktopBundle() };
  } catch (e) {
    bundle = { kind: "error", message: errorText(e) };
  } finally {
    bundleBusy = false;
  }
}

async function saveBundle(): Promise<void> {
  const dialogs = fileDialogs();
  const dir = await dialogs.defaultDirectory();
  const path = await dialogs.saveFile({
    title: t("agents.desktop.save.title"),
    defaultPath: dir ? joinPath(dir, "ugoe-editor.mcpb") : "ugoe-editor.mcpb",
    filters: [{ name: t("agents.desktop.save.filter"), extensions: ["mcpb"] }],
  });
  if (!path) return;
  bundleBusy = true;
  try {
    bundle = { kind: "saved", path: await ipc.saveClaudeDesktopBundle(path) };
  } catch (e) {
    bundle = { kind: "error", message: errorText(e) };
  } finally {
    bundleBusy = false;
  }
}

async function openManual(): Promise<void> {
  try {
    await openExternal(MCP_MANUAL_URL);
  } catch {
    copied = "manual:failed";
  }
}
</script>

<section class="agents">
  <p class="muted">
    {t("agents.intro")}
    <button type="button" class="link" onclick={openManual}>{t("agents.manual.label")}</button>
    {#if copied === "manual:failed"}<span class="muted">{MCP_MANUAL_URL}</span>{/if}
  </p>
  {#if agentBridge.error}<p class="error ugoe-selectable">{agentBridge.error}</p>{/if}
  {#if status}
    <Toggle
      checked={status.enabled}
      disabled={agentBridge.busy}
      label={t("agents.enable.label")}
      onchange={(on) => agentBridge.setEnabled(on)}
    />
    {#if status.error}
      <p class="error ugoe-selectable">{t("agents.state.failed", { message: status.error.message })}</p>
    {:else if status.running}
      <p class="state" data-state="on">
        <span class="dot on"></span>
        <span>
          <RichText text={t("agents.state.listening", { url: status.url ?? "" })}>
            {#snippet code(text)}<code>{text}</code>{/snippet}
          </RichText>
        </span>
      </p>
    {:else}
      <p class="state" data-state="off"><span class="dot"></span> {t("agents.state.off")}</p>
    {/if}

    {#if status.running}
      {#if status.project}
        <p>
          <RichText text={t("agents.project.label", { path: status.project })}>
            {#snippet code(text)}<code class="path">{text}</code>{/snippet}
          </RichText>
        </p>
      {:else}
        <p class="warn">{t("agents.project.unsaved")}</p>
      {/if}
      <h4>{t("agents.clients.title")}</h4>
      {#if status.clients.length === 0}
        <p class="muted">{t("agents.clients.empty")}</p>
      {:else}
        <ul class="clients">
          {#each status.clients as client (client.name)}
            <li>
              <strong>{client.name}</strong>{client.version ? ` ${client.version}` : ""}
              <span class="muted">{t("agents.clients.connected", { ago: connectedAgo(client.connected_at, now) })}</span>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}

    <h4>{t("agents.register.title")}</h4>
    {#if !status.sidecar.path}
      <p class="warn note">
        <RichText text={t("agents.register.noSidecar")}>
          {#snippet code(text)}<code>{text}</code>{/snippet}
          {#snippet build(text)}<code>{text}</code>{/snippet}
        </RichText>
      </p>
    {/if}
    {#if section}
      <div class="choices">
        <TabStrip
          tabs={choices}
          value={section.id}
          aria-label={t("agents.register.choices.aria")}
          onselect={(id) => ((chosen = id), (copied = null))}
        />
      </div>
      <p class="muted note">{section.summary}</p>
      {#if section.id === "claude-desktop"}
        {#if !status.enabled}
          <p class="warn note">{t("agents.desktop.enableFirst")}</p>
        {/if}
        {#if status.sidecar.in_wsl}
          <p class="warn note">{t("agents.desktop.inWsl")}</p>
        {/if}
        <div class="ed-buttons bundle">
          <Button variant="primary" onclick={installInClaudeDesktop} disabled={bundleBusy || !status.sidecar.path}>
            {t("agents.desktop.install.label")}
          </Button>
          <Button onclick={saveBundle} disabled={bundleBusy || !status.sidecar.path}>
            {t("agents.desktop.save.label")}
          </Button>
          {#if bundleBusy}<span class="muted">{t("agents.desktop.packing")}</span>{/if}
        </div>
        {#if bundle?.kind === "opened"}
          <p class="ok note">{t("agents.desktop.opened")}</p>
          <p class="muted note">
            <RichText text={t("agents.desktop.openedFallback", { path: bundle.path })}>
              {#snippet code(text)}<code class="path">{text}</code>{/snippet}
            </RichText>
          </p>
        {:else if bundle?.kind === "saved"}
          <p class="ok note">
            <RichText text={t("agents.desktop.saved", { path: bundle.path })}>
              {#snippet code(text)}<code class="path">{text}</code>{/snippet}
            </RichText>
          </p>
        {:else if bundle?.kind === "error"}
          <p class="error ugoe-selectable">{bundle.message}</p>
        {/if}
        <ul class="muted note checks">
          <li>{t("agents.desktop.check")}</li>
          <li>{t("agents.desktop.update")}</li>
        </ul>
      {/if}
      {#each section.steps as step, i (i)}
        {@const key = `${section.id}-${i}`}
        <div class="step">
          <div class="step-head">
            <span>{step.label}</span>
            <Button small onclick={() => copy(key, step.text)}>{t("agents.copy.label")}</Button>
            {#if copied === key}<span class="ok" role="status">{t("agents.copy.done")}</span>{/if}
            {#if copied === `${key}:failed`}<span class="warn" role="status">{t("agents.copy.failed")}</span>{/if}
          </div>
          <pre class="ugoe-selectable">{step.text}</pre>
          {#if step.note}<p class="muted note">{step.note}</p>{/if}
        </div>
      {/each}
      {#if section.id === "chatgpt"}
        <p class="muted note">{t("agents.chatgptNote")}</p>
      {/if}
    {/if}

    <Section title={t("agents.port.title")} open={false} level={4}>
      <form class="row" onsubmit={applyPort}>
        <label for="mcp-port">{t("agents.port.label")}</label>
        <TextField
          id="mcp-port"
          size={8}
          bind:value={portInput}
          inputmode="numeric"
          placeholder={t("agents.port.placeholder")}
        />
        <Button type="submit" disabled={agentBridge.busy}>{t("agents.port.apply.label")}</Button>
      </form>
      <p class="muted note">
        <RichText text={t("agents.port.description")}>
          {#snippet code(text)}<code>{text}</code>{/snippet}
        </RichText>
      </p>
      <p class="ed-buttons">
        <Button onclick={copyToken} disabled={agentBridge.busy}>{t("agents.token.copy.label")}</Button>
        {#if copied === "token"}<span class="ok" role="status">{t("agents.copy.done")}</span>{/if}
        {#if copied === "token:failed"}<span class="warn" role="status">{t("agents.token.copy.failed")}</span>{/if}
        <Button onclick={() => agentBridge.regenerateToken()} disabled={agentBridge.busy}>
          {t("agents.token.regenerate.label")}
        </Button>
      </p>
      <p class="muted note">
        {t("agents.token.description")}
        {#if status.discovery}
          <RichText text={t("agents.token.discovery", { path: status.discovery })}>
            {#snippet code(text)}<code class="path">{text}</code>{/snippet}
          </RichText>
        {/if}
      </p>
    </Section>
  {:else if !agentBridge.error}
    <p class="muted">{t("agents.loading")}</p>
  {/if}
</section>

<style>
  .state {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
  }
  .dot {
    display: inline-block;
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: var(--ugoe-radius-full);
    background: var(--ugoe-color-text-muted);
  }
  .dot.on {
    background: var(--ugoe-color-success);
  }
  .path {
    word-break: break-all;
  }
  h4 {
    margin: var(--ugoe-space-4) 0 var(--ugoe-space-2);
    font-size: var(--ugoe-font-size-sm);
    font-weight: var(--ugoe-font-weight-strong);
  }
  .clients {
    margin: var(--ugoe-space-2) 0;
    padding-left: 1.5em;
  }
  .choices {
    margin: var(--ugoe-space-2) 0;
  }
  .step {
    margin: var(--ugoe-space-2) 0;
  }
  .step-head {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-3);
  }
  .step-head span:first-child {
    flex: 1;
  }
  pre {
    margin: var(--ugoe-space-2) 0;
    padding: var(--ugoe-space-2) var(--ugoe-space-3);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-md);
    background: var(--ugoe-color-bg);
    white-space: pre-wrap;
    word-break: break-all;
    font-family: var(--ugoe-font-mono);
    font-size: var(--ugoe-font-size-sm);
  }
  .note {
    font-size: var(--ugoe-font-size-sm);
    margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-3);
  }
  .muted {
    color: var(--ugoe-color-text-muted);
  }
  .ok {
    color: var(--ugoe-color-success);
  }
  .bundle {
    margin: var(--ugoe-space-2) 0;
  }
  .checks {
    margin: var(--ugoe-space-2) 0 var(--ugoe-space-3);
    padding-left: 1.5em;
  }
  .warn {
    color: var(--ugoe-color-warning);
  }
  .error {
    color: var(--ugoe-color-danger);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--ugoe-color-accent);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .link:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: 2px;
  }
</style>
