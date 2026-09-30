<!--
  モジュールのプロジェクトの本体のファイルが、開いた (読み直した) 後で変わったときの確認 (docs/editor.md#本体の変化)。
  FileMenu が `change` を渡すと開く。
  - 本体が変わった: 本体を読み直す (保存していないモジュールの編集は保つ。Undo の履歴は消える)・後で
  - 本体が見つからない: 後で (保存はできる。本体を選び直すのは保存してから「Locate base…」)
  Esc は「後で」。
-->
<script lang="ts">
import { describeFiles } from "./externalChange";
import type { ExternalChange } from "./ipc";
import { baseFileName } from "./moduleProject";

interface Props {
  change: ExternalChange | null;
  /** 本体のファイルの場所。 */
  base: string;
  onreload: () => void;
  ondismiss: () => void;
}

let { change, base, onreload, ondismiss }: Props = $props();

let dialog = $state<HTMLDialogElement>();

$effect(() => {
  if (!dialog) return;
  if (change && !dialog.open) dialog.showModal();
  else if (!change && dialog.open) dialog.close();
});
</script>

<dialog
  bind:this={dialog}
  class="base-change"
  aria-labelledby="base-change-title"
  oncancel={(e) => {
    e.preventDefault();
    ondismiss();
  }}
>
  {#if change}
    {#if change.missing}
      <h2 id="base-change-title">The base model is gone</h2>
      <p>
        <code>{base}</code> was moved or deleted. Your module is still open and can be saved. To use the base from its
        new place, save, then open this project again and choose “Locate base…”.
      </p>
    {:else}
      <h2 id="base-change-title">The base model changed</h2>
      <p>
        {baseFileName(base)} was changed after you opened this module
        {#if change.files.length > 0}(<code>{describeFiles(change.files)}</code>){/if}. The model underneath is out of date.
      </p>
      <p class="muted">
        Reloading puts the new base underneath. Your module edits, saved or not, are kept; the undo history is cleared.
        If the module no longer fits the new base, nothing changes and you are told how to fix it.
      </p>
    {/if}
    <div class="actions">
      <button onclick={ondismiss} title="Keep the old base underneath for now">Later</button>
      {#if !change.missing}
        <button class="primary" onclick={onreload}>Reload the base</button>
      {/if}
    </div>
  {/if}
</dialog>

<style>
  .base-change {
    max-width: 520px;
    padding: 16px 20px;
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border);
  }
  .base-change::backdrop {
    background: rgb(0 0 0 / 40%);
  }
  h2 {
    margin: 0 0 8px;
    font-size: 15px;
    text-transform: none;
    letter-spacing: normal;
    color: var(--text);
  }
  code {
    overflow-wrap: anywhere;
  }
  .muted {
    opacity: 0.75;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  .primary {
    color: var(--accent);
    border-color: var(--accent);
  }
</style>
