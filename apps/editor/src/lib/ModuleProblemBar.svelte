<!--
  モジュールのプロジェクトが本体の問題で開けない・本体を読み直せないときの、問題と直し方 (docs/editor.md#本体の問題の直し方)。
  FileMenu が出す。直し方はバックエンドの問題の種類 (`ModuleProblem.kind`) から選ぶ (moduleProject.ts の moduleProblemFixes):
  本体を選び直す (「Locate base project…」「Locate base .ugoe…」)、別のスロットで開く、本体のプロジェクトを開いて直す。
  開いているモジュールの本体を読み直せなかったときは、開き直さずに付け替える (保存していない編集は残る。T0651)。
-->
<script lang="ts">
import type { ModuleProblem } from "./ipc";
import { describeModuleProblem, moduleProblemFixes } from "./moduleProject";

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
  ondismiss: () => void;
}

let { problem, canReopen, busy, onlocate, onchooseslot, onopenbase, ondismiss }: Props = $props();

const fixes = $derived(moduleProblemFixes(problem));
let slot = $state("");

$effect(() => {
  slot = problem.base_slots[0] ?? "";
});
</script>

<div class="problem" role="alert">
  <p class="summary">{describeModuleProblem(problem)}</p>
  <details>
    <summary>Details</summary>
    <code>{problem.message}</code>
  </details>
  <div class="actions">
    {#if fixes.includes("choose_slot")}
      <span class="slot">
        <select bind:value={slot} aria-label="Slot to use" disabled={busy || !canReopen}>
          {#each problem.base_slots as s (s)}<option value={s}>{s}</option>{/each}
        </select>
        <button disabled={busy || !canReopen || !slot} onclick={() => onchooseslot(slot)}
          title="Open this module on another slot of the base (saving remembers it)">Use this slot</button>
      </span>
    {/if}
    {#if fixes.includes("open_base")}
      <button disabled={busy} onclick={onopenbase} title="Open the base project to fix it; open the module again afterwards"
        >Open base project</button
      >
    {/if}
    <button disabled={busy || !canReopen} onclick={() => onlocate("project")}>Locate base project…</button>
    <button disabled={busy || !canReopen} onclick={() => onlocate("bundle")}>Locate base .ugoe…</button>
    <button class="close" onclick={ondismiss} aria-label="Close" title="Close">✕</button>
  </div>
  {#if !canReopen}
    <p class="muted">Save the module project first to choose another base or slot.</p>
  {/if}
</div>

<style>
  .problem {
    position: fixed;
    z-index: 20;
    top: 44px;
    left: 50%;
    transform: translateX(-50%);
    width: min(620px, calc(100vw - 32px));
    padding: 8px 12px;
    background: var(--panel);
    border: 1px solid #dc2626;
    border-radius: 4px;
    box-shadow: 0 4px 16px rgb(0 0 0 / 25%);
  }
  .summary {
    margin: 0 0 4px;
    color: #dc2626;
  }
  code {
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
  }
  .slot {
    display: inline-flex;
    gap: 4px;
  }
  .close {
    margin-left: auto;
  }
  .muted {
    margin: 4px 0 0;
    opacity: 0.7;
  }
</style>
