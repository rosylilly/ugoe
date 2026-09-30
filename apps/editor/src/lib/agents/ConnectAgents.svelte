<!--
  設定の「Connect agents」(docs/editor.md#エージェントをつなぐ、docs/manual/mcp.md)。
  エディタの中の MCP サーバ (ブリッジ) の有効化・状態 (待ち受けの URL・つながったクライアント・エージェントが使う
  プロジェクトのパス)・ポートとトークン、クライアントごとの登録の手順 (同梱の ugoe-mcp のパスを埋めたもの) を出す。
  エージェントの適用・保存は、ここでの設定に関わらず確認のダイアログ (ApprovalDialog.svelte) を通る。
-->
<script lang="ts">
import { openExternal } from "../help";
import { ipc } from "../ipc";
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
/** 写した・写せなかったことの知らせ (手順の番号ごと)。 */
let copied = $state<string | null>(null);
let portInput = $state("");
let now = $state(Date.now());

$effect(() => {
  portInput = status?.port ? String(status.port) : "";
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
    agentBridge.error = String(e);
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
    agentBridge.error =
      "Enter a port between 1 and 65535, or leave it empty to pick one automatically.";
    return;
  }
  await agentBridge.setPort(port);
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
    Let coding agents (Claude Code, OpenCode, ChatGPT…) work on the project open in this editor. Their proposals
    appear in the AI tab, and applying, undoing or saving always asks you here first.
    <button class="link" onclick={openManual}>How to connect agents…</button>
    {#if copied === "manual:failed"}<span class="muted">{MCP_MANUAL_URL}</span>{/if}
  </p>
  {#if agentBridge.error}<p class="error">{agentBridge.error}</p>{/if}
  {#if status}
    <label class="check">
      <input
        type="checkbox"
        checked={status.enabled}
        disabled={agentBridge.busy}
        onchange={(e) => agentBridge.setEnabled(e.currentTarget.checked)}
      />
      Allow agents to connect to this editor
    </label>
    {#if status.error}
      <p class="error">Could not start: {status.error}</p>
    {:else if status.running}
      <p class="state" data-state="on">
        <span class="dot on"></span> Listening on <code>{status.url}</code> (this computer only, token required)
      </p>
    {:else}
      <p class="state" data-state="off"><span class="dot"></span> Off. Agents cannot connect.</p>
    {/if}

    {#if status.running}
      {#if status.project}
        <p>Agents work on <code class="path">{status.project}</code></p>
      {:else}
        <p class="warn">
          Save the project first (File → Save As…). Agents refer to the project by its path, so they cannot use an
          unsaved one.
        </p>
      {/if}
      <h4>Connected agents</h4>
      {#if status.clients.length === 0}
        <p class="muted">None yet. Register the editor in your agent below, then start it.</p>
      {:else}
        <ul class="clients">
          {#each status.clients as client (client.name)}
            <li>
              <strong>{client.name}</strong>{client.version ? ` ${client.version}` : ""}
              <span class="muted">· connected {connectedAgo(client.connected_at, now)}</span>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}

    <h4>Register the editor in your agent</h4>
    {#if !status.sidecar.path}
      <p class="warn note">
        No <code>ugoe-mcp</code> was found next to the editor or in the build folder (this is a development build;
        installers include it). Build it with <code>cargo build --release -p ugoe-mcp</code> and reopen this tab. Until then the
        commands use <code>ugoe-mcp</code> from the PATH.
      </p>
    {/if}
    <div class="choices" role="tablist" aria-label="Agent">
      {#each sections as s (s.id)}
        <button role="tab" aria-selected={section?.id === s.id} class:current={section?.id === s.id}
          onclick={() => ((chosen = s.id), (copied = null))}>{s.title}</button>
      {/each}
    </div>
    {#if section}
      <p class="muted note">{section.summary}</p>
      {#each section.steps as step, i (i)}
        {@const key = `${section.id}-${i}`}
        <div class="step">
          <div class="step-head">
            <span>{step.label}</span>
            <button onclick={() => copy(key, step.text)}>Copy</button>
            {#if copied === key}<span class="ok">Copied</span>{/if}
            {#if copied === `${key}:failed`}<span class="warn">Select the text and copy it</span>{/if}
          </div>
          <pre>{step.text}</pre>
          {#if step.note}<p class="muted note">{step.note}</p>{/if}
        </div>
      {/each}
      {#if section.id === "chatgpt"}
        <p class="muted note">
          In ChatGPT, enable developer mode, create an app and choose the tunnel as its connection. ChatGPT asks
          before tools that change the project; the editor asks you too.
        </p>
      {/if}
    {/if}

    <details>
      <summary>Port and token</summary>
      <form class="row" onsubmit={applyPort}>
        <label for="mcp-port">Port</label>
        <input id="mcp-port" bind:value={portInput} inputmode="numeric" placeholder="automatic" />
        <button type="submit" disabled={agentBridge.busy}>Apply</button>
      </form>
      <p class="muted note">
        The stdio relay (<code>ugoe-mcp --editor</code>) finds the port by itself. A fixed port is only needed for
        clients that connect over HTTP directly.
      </p>
      <p class="row">
        <button onclick={copyToken} disabled={agentBridge.busy}>Copy token</button>
        {#if copied === "token"}<span class="ok">Copied</span>{/if}
        <button onclick={() => agentBridge.regenerateToken()} disabled={agentBridge.busy}>Regenerate token</button>
      </p>
      <p class="muted note">
        The token is only needed by clients that connect over HTTP directly. Regenerate it if it was shown to
        someone else; the relay picks up the new one automatically.
        {#if status.discovery}The port and token are written to <code class="path">{status.discovery}</code> (readable
          only by you) while the bridge is on.{/if}
      </p>
    </details>
  {:else if !agentBridge.error}
    <p class="muted">Loading…</p>
  {/if}
</section>

<style>
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .state {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted);
  }
  .dot.on {
    background: #16a34a;
  }
  .path {
    word-break: break-all;
  }
  h4 {
    margin: 12px 0 4px;
    font-size: 12px;
  }
  .clients {
    margin: 4px 0;
    padding-left: 18px;
  }
  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 4px 0;
  }
  .choices button.current {
    color: var(--accent);
    border-color: var(--accent);
  }
  .step {
    margin: 6px 0;
  }
  .step-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .step-head span:first-child {
    flex: 1;
  }
  pre {
    margin: 4px 0;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    white-space: pre-wrap;
    word-break: break-all;
    font-size: 12px;
    user-select: text;
  }
  .note {
    font-size: 12px;
    margin: 2px 0 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row input {
    width: 8em;
  }
  .ok {
    color: #16a34a;
  }
  .warn {
    color: #b45309;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
  }
</style>
