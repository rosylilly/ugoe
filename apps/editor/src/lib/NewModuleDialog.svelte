<!--
  着せ替えのモジュールのプロジェクトの新規作成 (File > New Module…、docs/editor.md#モジュールのプロジェクト)。
  本体 (.ugoeproj のフォルダーか .ugoe) を選ぶとスロットの一覧を読み、スロットとモジュールの名前を決めて作る。
  スロットのない本体は、本体のプロジェクトの「Slots」タブでスロットを足してから選ぶ。Esc で閉じる。
-->
<script lang="ts">
import { fileDialogs } from "./fileDialogs";
import { BUNDLE_FILTER } from "./filePaths";
import { type BaseSummary, ipc } from "./ipc";
import { isValidModuleName, MODULE_CAN, MODULE_CANNOT, suggestModuleName } from "./moduleProject";

interface Props {
  open: boolean;
  /** 最初に開く場所。 */
  startDir: string | null;
  /** 作る (失敗すると例外。メッセージを出してダイアログは開いたまま)。 */
  oncreate: (base: string, slot: string, name: string) => Promise<void>;
  onclose: () => void;
}

let { open, startDir, oncreate, onclose }: Props = $props();

let dialog = $state<HTMLDialogElement>();
let basePath = $state<string | null>(null);
let base = $state<BaseSummary | null>(null);
let slot = $state<string | null>(null);
let name = $state("");
/** 名前を手で変えたか (変えていなければスロットを選ぶたびに候補を入れる)。 */
let nameEdited = false;
let error = $state<string | null>(null);
let busy = $state(false);

$effect(() => {
  if (!dialog) return;
  if (open && !dialog.open) dialog.showModal();
  else if (!open && dialog.open) dialog.close();
});

async function chooseBase(kind: "project" | "bundle") {
  const dialogs = fileDialogs();
  const defaultPath = startDir ?? undefined;
  const path =
    kind === "project"
      ? await dialogs.openDirectory({
          title: "Choose the base project (.ugoeproj folder)",
          defaultPath,
        })
      : await dialogs.openFile({
          title: "Choose the base bundle",
          defaultPath,
          filters: [BUNDLE_FILTER],
        });
  if (!path) return;
  busy = true;
  try {
    base = await ipc.readModuleBase(path);
    basePath = path;
    error = null;
    selectSlot(base.slots[0]?.id ?? null);
  } catch (e) {
    error = String(e);
  } finally {
    busy = false;
  }
}

function selectSlot(id: string | null) {
  slot = id;
  if (id && !nameEdited) name = suggestModuleName(id);
}

async function create() {
  if (!basePath || !slot || !isValidModuleName(name)) return;
  busy = true;
  try {
    await oncreate(basePath, slot, name.trim());
    error = null;
  } catch (e) {
    error = String(e);
  } finally {
    busy = false;
  }
}

const ready = $derived(basePath !== null && slot !== null && isValidModuleName(name) && !busy);
</script>

<dialog
  bind:this={dialog}
  class="new-module"
  aria-labelledby="new-module-title"
  oncancel={(e) => {
    e.preventDefault();
    onclose();
  }}
>
  <h2 id="new-module-title">New dress-up module</h2>
  <p class="muted">
    A module is made in its own project that refers to a base model. The base is shown underneath and cannot be
    edited here.
  </p>
  <details class="features">
    <summary>What you can do in a module project</summary>
    <div class="columns">
      <div>
        <strong>You can</strong>
        <ul>{#each MODULE_CAN as line (line)}<li>{line}</li>{/each}</ul>
      </div>
      <div>
        <strong>Edit in the base project instead</strong>
        <ul>{#each MODULE_CANNOT as line (line)}<li>{line}</li>{/each}</ul>
      </div>
    </div>
    <p class="muted">The name, slot and requirements can be changed later by clicking the “Module:” badge.</p>
  </details>
  <div class="row">
    <span class="label">Base</span>
    <button onclick={() => chooseBase("project")} disabled={busy}>Project folder…</button>
    <button onclick={() => chooseBase("bundle")} disabled={busy}>.ugoe file…</button>
  </div>
  {#if base && basePath}
    <p class="base" title={basePath}><strong>{base.name}</strong> <code>{basePath}</code></p>
    {#if base.slots.length === 0}
      <p class="warn">
        This base has no slots. Open the base project and add a slot in the Slots tab first.
      </p>
    {:else}
      <fieldset>
        <legend>Slot</legend>
        {#each base.slots as s (s.id)}
          <label class="slot">
            <input
              type="radio"
              name="slot"
              value={s.id}
              checked={slot === s.id}
              onchange={() => selectSlot(s.id)}
            />
            <strong>{s.id}</strong>
            <span class="muted">
              on {s.anchor ?? "canvas"} · draw order {s.draw_order_min}–{s.draw_order_max}{#if s.replaces.length > 0}
                · replaces {s.replaces.join(", ")}{/if}
            </span>
          </label>
        {/each}
      </fieldset>
      <label class="row">
        <span class="label">Module name</span>
        <input
          type="text"
          value={name}
          oninput={(e) => {
            nameEdited = true;
            name = e.currentTarget.value;
          }}
        />
      </label>
      {#if name.length > 0 && !isValidModuleName(name)}
        <p class="warn">The name must not contain "/" or start with "#".</p>
      {/if}
    {/if}
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <div class="actions">
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={create} disabled={!ready}>Create</button>
  </div>
</dialog>

<style>
  .new-module {
    width: min(560px, 90vw);
    padding: 16px 20px;
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border);
  }
  .new-module::backdrop {
    background: rgb(0 0 0 / 40%);
  }
  h2 {
    margin: 0 0 8px;
    font-size: 15px;
    text-transform: none;
    letter-spacing: normal;
    color: var(--text);
  }
  .features {
    margin: 4px 0 8px;
  }
  .features ul {
    margin: 4px 0;
    padding-left: 18px;
  }
  .columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 0;
  }
  .label {
    min-width: 96px;
  }
  .base code {
    overflow-wrap: anywhere;
    opacity: 0.7;
  }
  fieldset {
    border: 1px solid var(--border);
    margin: 8px 0;
    padding: 4px 8px;
  }
  .slot {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 2px 0;
  }
  .muted {
    opacity: 0.7;
  }
  .warn {
    color: #b45309;
  }
  .error {
    color: #dc2626;
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  .primary {
    color: var(--accent);
    border-color: var(--accent);
  }
</style>
