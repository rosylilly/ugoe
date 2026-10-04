<!--
  モジュールのプロジェクトが本体の問題で開けない・本体を読み直せないときの、問題と直し方 (docs/editor.md#本体の問題の直し方)。
  FileMenu が出す。直し方はバックエンドの問題の種類 (`ModuleProblem.kind`) から選ぶ (moduleProject.ts の moduleProblemFixes):
  本体を選び直す (「Locate base project…」「Locate base .ugoe…」)、別のスロットで開く、本体のプロジェクトを開いて直す。
  開いているモジュールの本体を読み直せなかったときは、開き直さずに付け替える (保存していない編集は残る。T0651)。
  物理の無い本体で開けないモジュールは、衝突体の参照を外して開ける (T0661)。外すものを並べて確かめてから開く
  (T0655 の「黙って消さない」)。知らせの帯 (Notice)。FileMenu がヘッダーの下に重ねる。
-->
<script lang="ts">
import { Button, Notice, Select } from "@ugoe/ui";
import { messageText } from "./errors";
import { t } from "./i18n";
import type { ModuleProblem } from "./ipc";
import { describeModuleProblem, dropCollidersPrompt, moduleProblemFixes } from "./moduleProject";
import TipButton from "./ui/TipButton.svelte";

interface Props {
  problem: ModuleProblem;
  /**
   * 本体・スロットの付け替えができるか (開けなかったプロジェクトは保存した場所から開き直す。開いているプロジェクトは
   * いつでも付け替えられる)。
   */
  canReopen: boolean;
  busy: boolean;
  onlocate: (kind: "project" | "bundle") => void;
  onchooseslot: (slot: string) => void;
  onopenbase: () => void;
  /** 衝突体の参照 (`problem.collider_references`) を外して開く (利用者が確かめた後に呼ぶ)。 */
  ondropcolliders: () => void;
  ondismiss: () => void;
}

let {
  problem,
  canReopen,
  busy,
  onlocate,
  onchooseslot,
  onopenbase,
  ondropcolliders,
  ondismiss,
}: Props = $props();

const fixes = $derived(moduleProblemFixes(problem));
const slotOptions = $derived(problem.base_slots.map((s) => ({ value: s, label: s })));
let slot = $state("");
/** 衝突体の参照を外す確認を出しているか。 */
let confirmDrop = $state(false);

$effect(() => {
  slot = problem.base_slots[0] ?? "";
  confirmDrop = false;
});
</script>

<div class="problem">
  <Notice tone="danger" title={describeModuleProblem(problem)} {ondismiss}>
    <details>
      <summary>{t("moduleProblem.details.label")}</summary>
      <code class="ugoe-selectable">{messageText(problem)}</code>
    </details>
    {#if confirmDrop}
      <div class="confirm" role="group" aria-label={t("moduleProblem.dropColliders.aria")}>
        <p>{dropCollidersPrompt(problem.collider_references)}</p>
        <div class="actions">
          <Button variant="danger" disabled={busy} onclick={ondropcolliders}>
            {t("moduleProblem.dropColliders.confirm")}
          </Button>
          <Button disabled={busy} onclick={() => (confirmDrop = false)}>
            {t("moduleProblem.dropColliders.cancel")}
          </Button>
        </div>
      </div>
    {/if}
    {#if !canReopen}
      <p class="muted">{t("moduleProblem.saveFirst")}</p>
    {/if}
    {#snippet actions()}
      {#if fixes.includes("choose_slot")}
        <span class="slot">
          <Select
            aria-label={t("moduleProblem.slot.aria")}
            bind:value={slot}
            options={slotOptions}
            disabled={busy || !canReopen}
          />
          <TipButton
            disabled={busy || !canReopen || !slot}
            tooltip={t("moduleProblem.useSlot.tooltip")}
            onclick={() => onchooseslot(slot)}>{t("moduleProblem.useSlot.label")}</TipButton
          >
        </span>
      {/if}
      {#if fixes.includes("open_base")}
        <TipButton disabled={busy} tooltip={t("moduleProblem.openBase.tooltip")} onclick={onopenbase}>
          {t("moduleProblem.openBase.label")}
        </TipButton>
      {/if}
      {#if fixes.includes("drop_colliders")}
        <TipButton
          disabled={busy || !canReopen || confirmDrop}
          tooltip={t("moduleProblem.dropColliders.tooltip")}
          onclick={() => (confirmDrop = true)}>{t("moduleProblem.dropColliders.label")}</TipButton
        >
      {/if}
      <Button disabled={busy || !canReopen} onclick={() => onlocate("project")}>
        {t("moduleProblem.locateProject.label")}
      </Button>
      <Button disabled={busy || !canReopen} onclick={() => onlocate("bundle")}>
        {t("moduleProblem.locateBundle.label")}
      </Button>
    {/snippet}
  </Notice>
</div>

<style>
  /* 置き場所は FileMenu の知らせの帯 (.bands)。地を敷いて、下のビューポートが透けないようにする。 */
  .problem {
    border-radius: var(--ugoe-radius-lg);
    background: var(--ugoe-color-surface);
  }
  p {
    margin: 0;
  }
  code {
    overflow-wrap: anywhere;
  }
  .confirm {
    margin-top: var(--ugoe-space-2);
    padding: var(--ugoe-space-2) var(--ugoe-space-3);
    border: 1px dashed var(--ugoe-color-danger);
    border-radius: var(--ugoe-radius-md);
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-2);
    margin-top: var(--ugoe-space-2);
  }
  .slot {
    display: inline-flex;
    gap: var(--ugoe-space-2);
  }
  .muted {
    color: var(--ugoe-color-text-muted);
  }
</style>
