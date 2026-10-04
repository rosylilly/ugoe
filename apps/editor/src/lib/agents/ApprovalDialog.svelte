<!--
  エディタにつないだエージェントの求め (提案の適用・Undo・Redo・保存) の確認 (docs/editor.md#エージェントをつなぐ)。
  答えを待っている求めの先頭を、画面の下に出す。画面の操作をふさがない (モーダルにしない) ので、「Show in AI tab」で提案を
  プレビューしながら決められる。答えずに時間が過ぎると、エージェントには時間切れのエラーが返り、ここから消える。
  エージェントが求めを取り消したときも消え、`AgentBridge.handle` が取り消されたことを Toast で知らせる。
-->
<script lang="ts">
import { formatList, RichText } from "@ugoe/i18n";
import { Button } from "@ugoe/ui";
import { t } from "../i18n";
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

/** 求めを受け取った時刻 (残りの秒数を出す。前の求めに答えている間も、後ろの求めの時間は過ぎている)。 */
let receivedAt = $state(0);
let now = $state(Date.now());
$effect(() => {
  if (request) {
    now = Date.now();
    receivedAt = agentBridge.receivedAt(request.id) ?? now;
  }
});
$effect(() => {
  if (!request) return;
  const timer = setInterval(() => (now = Date.now()), 1000);
  return () => clearInterval(timer);
});
const remaining = $derived(
  request ? Math.max(0, request.timeout_secs - Math.floor((now - receivedAt) / 1000)) : 0,
);

function who(r: ApprovalRequest): string {
  return r.client ?? t("agents.approval.agent");
}

function heading(r: ApprovalRequest): string {
  switch (r.action) {
    case "apply":
      return t("agents.approval.heading.apply", { who: who(r) });
    case "undo":
      return t("agents.approval.heading.undo", { who: who(r) });
    case "redo":
      return t("agents.approval.heading.redo", { who: who(r) });
    case "save":
      return t("agents.approval.heading.save", { who: who(r) });
  }
}

function approveLabel(r: ApprovalRequest): string {
  switch (r.action) {
    case "apply":
      return t("agents.approval.approve.apply");
    case "undo":
      return t("agents.approval.approve.undo");
    case "redo":
      return t("agents.approval.approve.redo");
    case "save":
      return t("agents.approval.approve.save");
  }
}
</script>

{#if request}
  <div class="approval" role="alertdialog" aria-labelledby="approval-title" aria-describedby="approval-body">
    <h2 id="approval-title">{heading(request)}</h2>
    <div id="approval-body">
      {#if request.action === "apply"}
        <p><strong>{request.title ?? t("agents.approval.proposal", { id: request.proposal })}</strong></p>
        {#if proposal}
          <p class="muted">
            {t("agents.approval.commands", {
              count: proposal.commands.length - request.excluded.length,
              total: proposal.commands.length,
            })}
          </p>
        {/if}
        {#if request.excluded.length > 0}
          <p class="muted">
            {t("agents.approval.excluded", { list: formatList(request.excluded.map((n) => `#${n}`)) })}
          </p>
        {/if}
      {:else if request.action === "undo"}
        <p class="muted">{t("agents.approval.undo")}</p>
      {:else if request.action === "redo"}
        <p class="muted">{t("agents.approval.redo")}</p>
      {:else}
        <p class="muted">{t("agents.approval.save")}</p>
        {#if request.bundle}
          <p>
            <RichText text={t("agents.approval.bundle", { path: request.bundle })}>
              {#snippet code(text)}<code class="path">{text}</code>{/snippet}
            </RichText>
          </p>
        {/if}
      {/if}
      <p class="muted small">
        {t("agents.approval.remaining", { count: remaining })}
        {#if waiting > 1}{t("agents.approval.waiting", { count: waiting - 1 })}{/if}
      </p>
    </div>
    <div class="actions">
      {#if request.action === "apply" && request.proposal != null && proposal}
        <Button onclick={() => request.proposal != null && onshow(request.proposal, request.excluded)}
          >{t("agents.approval.show.label")}</Button>
      {/if}
      <span class="spacer"></span>
      <Button onclick={() => agentBridge.answer(request.id, false)}>{t("agents.approval.decline.label")}</Button>
      <Button variant="primary" onclick={() => agentBridge.answer(request.id, true)}>{approveLabel(request)}</Button>
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
    padding: var(--ugoe-space-4) var(--ugoe-space-5);
    background: var(--ugoe-color-surface);
    color: var(--ugoe-color-text);
    border: 2px solid var(--ugoe-color-accent);
    border-radius: var(--ugoe-radius-lg);
    box-shadow: var(--ugoe-shadow-overlay);
  }
  h2 {
    margin: 0 0 var(--ugoe-space-2);
    font-size: var(--ugoe-font-size-lg);
    color: var(--ugoe-color-text);
  }
  p {
    margin: var(--ugoe-space-2) 0;
  }
  .muted {
    color: var(--ugoe-color-text-muted);
  }
  .small {
    font-size: var(--ugoe-font-size-sm);
  }
  .path {
    word-break: break-all;
  }
  .actions {
    display: flex;
    gap: var(--ugoe-space-2);
    margin-top: var(--ugoe-space-3);
  }
  .spacer {
    flex: 1;
  }
</style>
