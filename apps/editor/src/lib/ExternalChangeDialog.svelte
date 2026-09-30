<!--
  開いているプロジェクトがエディタの外で書き換えられたときの確認 (docs/editor.md#外での書き換え)。
  FileMenu が `change` を渡すと開く。選べること:
  - 未保存の変更がない: 開き直す・今のまま (後で)
  - 未保存の変更がある (または保存しようとした): 開き直す (自分の変更を捨てる)・自分の版で上書き・別名で保存・後で
  - プロジェクトが消された: その場所に保存し直す・別名で保存・後で
  Esc は「後で」。
-->
<script lang="ts">
import { describeFiles } from "./externalChange";
import type { ExternalChange } from "./ipc";

interface Props {
  change: ExternalChange | null;
  /** 未保存の変更があるか。 */
  modified: boolean;
  /** 保存しようとして見つかったか (上書きを選べるようにする)。 */
  saving: boolean;
  /** 未保存の変更がなければ自動で開き直す設定。 */
  autoReload: boolean;
  onautoreload: (enabled: boolean) => void;
  onreload: () => void;
  onoverwrite: () => void;
  onsaveas: () => void;
  ondismiss: () => void;
}

let {
  change,
  modified,
  saving,
  autoReload,
  onautoreload,
  onreload,
  onoverwrite,
  onsaveas,
  ondismiss,
}: Props = $props();

let dialog = $state<HTMLDialogElement>();

$effect(() => {
  if (!dialog) return;
  if (change && !dialog.open) dialog.showModal();
  else if (!change && dialog.open) dialog.close();
});

const canOverwrite = $derived(modified || saving);
</script>

<dialog
  bind:this={dialog}
  class="external"
  aria-labelledby="external-title"
  oncancel={(e) => {
    e.preventDefault();
    ondismiss();
  }}
>
  {#if change}
    {#if change.missing}
      <h2 id="external-title">The project folder is gone</h2>
      <p>The project was moved or deleted outside the editor. Your work is still open here.</p>
    {:else}
      <h2 id="external-title">The project was changed outside the editor</h2>
      <p>
        Another program (for example an AI agent through <code>ugoe-mcp</code>, or another editor) changed
        <code>{describeFiles(change.files)}</code> after you opened or saved it.
      </p>
      {#if modified}
        <p class="warn">You have unsaved changes. Reloading discards them; overwriting discards the other changes.</p>
      {:else if saving}
        <p class="warn">Saving now overwrites the other changes.</p>
      {/if}
    {/if}
    {#if !change.missing}
      <label class="check">
        <input type="checkbox" checked={autoReload}
          onchange={(e) => onautoreload((e.currentTarget as HTMLInputElement).checked)} />
        Reload automatically when there are no unsaved changes
      </label>
    {/if}
    <div class="actions">
      <button onclick={ondismiss} title="Keep working on the open version; you will be asked again before saving">Later</button>
      {#if canOverwrite || change.missing}
        <button onclick={onsaveas}>Save As…</button>
      {/if}
      {#if change.missing}
        <button class="primary" onclick={onoverwrite}>Save here again</button>
      {:else}
        {#if canOverwrite}
          <button class="danger" onclick={onoverwrite}>Overwrite with my version</button>
        {/if}
        <button class={modified ? "danger" : "primary"} onclick={onreload}
          >{modified ? "Reload and discard my changes" : "Reload"}</button>
      {/if}
    </div>
  {/if}
</dialog>

<style>
  .external {
    max-width: 520px;
    padding: 16px 20px;
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border);
  }
  .external::backdrop {
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
  .warn {
    color: #b45309;
  }
  .check {
    display: flex;
    align-items: flex-start;
    gap: 6px;
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
  .danger {
    color: #dc2626;
    border-color: #dc2626;
  }
</style>
