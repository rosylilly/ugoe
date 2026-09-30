<!--
  エディタにつないだエージェントの求め (提案の適用・Undo・Redo・保存) の確認 (docs/editor.md#エージェントをつなぐ)。
  答えを待っている求めの先頭を、画面の下に出す。画面の操作をふさがない (モーダルにしない) ので、「Show in AI tab」で提案を
  プレビューしながら決められる。答えずに時間が過ぎると、エージェントには時間切れのエラーが返り、ここから消える。
-->
<script lang="ts">
import type { ApprovalRequest, ProposalView } from "../ipc";
import { agentBridge } from "./bridge.svelte";

interface Props {
  /** 提案の一覧 (適用の求めで、題名とコマンドの数を出す)。 */
  proposals: ProposalView[];
  /** 提案を AI タブでプレビューする (エージェントが外すコマンドを外して)。 */
  onshow: (id: number, excluded: number[]) => void;
}

let { proposals, onshow }: Props = $props();

const request = $derived(agentBridge.approvals[0] ?? null);
const waiting = $derived(agentBridge.approvals.length);
const proposal = $derived(
  request?.proposal != null ? (proposals.find((p) => p.id === request.proposal) ?? null) : null,
);

/** 求めが出た時刻 (残りの秒数を出す)。 */
let shownAt = $state(0);
let now = $state(Date.now());
$effect(() => {
  if (request) {
    shownAt = Date.now();
    now = shownAt;
  }
});
$effect(() => {
  if (!request) return;
  const timer = setInterval(() => (now = Date.now()), 1000);
  return () => clearInterval(timer);
});
const remaining = $derived(
  request ? Math.max(0, request.timeout_secs - Math.floor((now - shownAt) / 1000)) : 0,
);

function who(r: ApprovalRequest): string {
  return r.client ?? "An agent";
}

function heading(r: ApprovalRequest): string {
  switch (r.action) {
    case "apply":
      return `${who(r)} wants to apply a proposal`;
    case "undo":
      return `${who(r)} wants to undo the last edit`;
    case "redo":
      return `${who(r)} wants to redo the last undone edit`;
    case "save":
      return `${who(r)} wants to save the project`;
  }
}

function approveLabel(r: ApprovalRequest): string {
  switch (r.action) {
    case "apply":
      return "Apply";
    case "undo":
      return "Undo";
    case "redo":
      return "Redo";
    case "save":
      return "Save";
  }
}
</script>

{#if request}
  <div class="approval" role="alertdialog" aria-labelledby="approval-title" aria-describedby="approval-body">
    <h2 id="approval-title">{heading(request)}</h2>
    <div id="approval-body">
      {#if request.action === "apply"}
        <p><strong>{request.title ?? `Proposal ${request.proposal}`}</strong></p>
        {#if proposal}
          <p class="muted">
            {proposal.commands.length - request.excluded.length} of {proposal.commands.length} commands, applied as
            one edit (one Undo reverts it).
          </p>
        {/if}
        {#if request.excluded.length > 0}
          <p class="muted">Leaves out commands #{request.excluded.join(", #")}.</p>
        {/if}
      {:else if request.action === "undo" || request.action === "redo"}
        <p class="muted">
          This {request.action === "undo" ? "undoes" : "redoes"} the most recent edit in the editor, whoever made
          it.
        </p>
      {:else}
        <p class="muted">Writes the project to its folder, as File → Save does.</p>
        {#if request.bundle}<p>Also exports <code class="path">{request.bundle}</code></p>{/if}
      {/if}
      <p class="muted small">
        Answer within {remaining} s, or the agent is told you did not answer.
        {#if waiting > 1}{waiting - 1} more waiting.{/if}
      </p>
    </div>
    <div class="actions">
      {#if request.action === "apply" && request.proposal != null && proposal}
        <button onclick={() => request.proposal != null && onshow(request.proposal, request.excluded)}
          >Show in AI tab</button>
      {/if}
      <span class="spacer"></span>
      <button onclick={() => agentBridge.answer(request.id, false)}>Decline</button>
      <button class="primary" onclick={() => agentBridge.answer(request.id, true)}>{approveLabel(request)}</button>
    </div>
  </div>
{/if}

<style>
  .approval {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 50;
    width: min(460px, calc(100vw - 32px));
    padding: 12px 16px;
    background: var(--panel);
    color: var(--text);
    border: 2px solid var(--accent);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 35%);
  }
  h2 {
    margin: 0 0 6px;
    font-size: 14px;
    text-transform: none;
    letter-spacing: normal;
    color: var(--text);
  }
  p {
    margin: 4px 0;
  }
  .small {
    font-size: 12px;
  }
  .path {
    word-break: break-all;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
  .spacer {
    flex: 1;
  }
  .primary {
    color: var(--accent);
    border-color: var(--accent);
  }
</style>
