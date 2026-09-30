<!--
  モジュールのプロジェクトの設定 (docs/editor.md#モジュールの設定): 名前・付けるスロット・本体に求める条件 (`requires`)。
  ファイルメニューの `Module: <スロット>` の印を押すと開く。「Apply」で set_module_settings を 1 つ送る (1 回の Undo で戻る)。
  本体がスロットや条件を満たさなければ、バックエンドが拒否し、理由をここに出す。Esc で閉じる。
-->
<script lang="ts">
import type { Command, ModuleStatus } from "./ipc";
import { isValidModuleName, requirementsOf, toggled } from "./moduleProject";

interface Props {
  open: boolean;
  module: ModuleStatus | null;
  /** コマンドを適用する (失敗すると例外。メッセージを出してダイアログは開いたまま)。 */
  onapply: (command: Command) => Promise<void>;
  onclose: () => void;
}

let { open, module, onapply, onclose }: Props = $props();

let dialog = $state<HTMLDialogElement>();
let name = $state("");
let slot = $state("");
let avatarLevel = $state<number | null>(null);
let slots = $state<string[]>([]);
let parameters = $state<string[]>([]);
let error = $state<string | null>(null);
let busy = $state(false);

$effect(() => {
  if (!dialog) return;
  if (open && !dialog.open) {
    reset();
    dialog.showModal();
  } else if (!open && dialog.open) dialog.close();
});

/** 今の設定を欄に入れる (開くたび)。 */
function reset(): void {
  if (!module) return;
  const r = requirementsOf(module);
  name = module.name;
  slot = module.slot.id;
  avatarLevel = r.avatar_level;
  slots = r.slots;
  parameters = r.parameters;
  error = null;
}

const baseSlotIds = $derived(module?.base_slots.map((s) => s.id) ?? []);
const current = $derived(module?.base_slots.find((s) => s.id === slot) ?? null);
/** 取り付け先が今のスロットと違う (見た目の位置が変わりうる)。 */
const anchorChanges = $derived(
  module !== null && current !== null && current.anchor !== module.slot.anchor,
);
/** 名前を変える (ID の頭が変わる)。 */
const renamed = $derived(module !== null && name.trim() !== module.name);
/** 求められるアバタープロファイルのレベル (本体が名乗るレベルまで)。 */
const levels = $derived(Array.from({ length: module?.base_avatar_level ?? 0 }, (_, i) => i + 1));

async function apply(): Promise<void> {
  if (!module || !isValidModuleName(name) || !slot) return;
  busy = true;
  try {
    await onapply({
      op: "set_module_settings",
      name: name.trim(),
      slot,
      requires: {
        avatar_level: avatarLevel,
        // 付けるスロットは条件に書かなくてよい
        slots: slots.filter((s) => s !== slot),
        parameters,
      },
    });
    error = null;
  } catch (e) {
    error = String(e);
  } finally {
    busy = false;
  }
}
</script>

<dialog
  bind:this={dialog}
  class="module-settings"
  aria-labelledby="module-settings-title"
  oncancel={(e) => {
    e.preventDefault();
    onclose();
  }}
>
  <h2 id="module-settings-title">Module settings</h2>
  {#if module}
    <label class="row">
      <span class="label">Module name</span>
      <input type="text" bind:value={name} />
    </label>
    {#if name.length > 0 && !isValidModuleName(name)}
      <p class="warn">The name must not contain "/" or start with "#".</p>
    {:else if renamed}
      <p class="muted">
        The name is the prefix of the module's IDs in a model (<code>{name.trim()}/…</code>). Modules that go on the
        same model at once need different names.
      </p>
    {/if}

    <fieldset>
      <legend>Slot</legend>
      {#each module.base_slots as s (s.id)}
        <label class="slot">
          <input type="radio" name="module-slot" value={s.id} checked={slot === s.id}
            onchange={() => (slot = s.id)} />
          <strong>{s.id}</strong>
          <span class="muted">on {s.anchor ?? "canvas"} · draw order {s.draw_order_min}–{s.draw_order_max}</span>
        </label>
      {/each}
      {#if anchorChanges}
        <p class="warn">
          This slot attaches to {current?.anchor ?? "the canvas"}, not {module.slot.anchor ?? "the canvas"}. The module
          keeps its coordinates relative to the attachment, so it may appear somewhere else. Check it after applying
          (Undo goes back).
        </p>
      {/if}
    </fieldset>

    <fieldset>
      <legend>Requires (checked when an app puts the module on)</legend>
      <label class="row">
        <span class="label">Avatar profile level</span>
        <select
          value={avatarLevel === null ? "" : String(avatarLevel)}
          onchange={(e) => {
            const v = e.currentTarget.value;
            avatarLevel = v === "" ? null : Number(v);
          }}
        >
          <option value="">(none)</option>
          {#each levels as level (level)}
            <option value={String(level)}>{level} or higher</option>
          {/each}
        </select>
      </label>
      {#if baseSlotIds.length > 1}
        <p class="muted">Other slots the model must have:</p>
        <div class="checks">
          {#each baseSlotIds.filter((s) => s !== slot) as id (id)}
            <label>
              <input type="checkbox" checked={slots.includes(id)}
                onchange={(e) => (slots = toggled(slots, id, e.currentTarget.checked, baseSlotIds))} />
              {id}
            </label>
          {/each}
        </div>
      {/if}
      <p class="muted">Parameters the model must have:</p>
      <div class="checks">
        {#each module.base_parameters as id (id)}
          <label>
            <input type="checkbox" checked={parameters.includes(id)}
              onchange={(e) =>
                (parameters = toggled(parameters, id, e.currentTarget.checked, module?.base_parameters ?? []))} />
            {id}
          </label>
        {/each}
      </div>
    </fieldset>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <div class="actions">
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={apply} disabled={busy || !isValidModuleName(name) || !slot}>Apply</button>
  </div>
</dialog>

<style>
  .module-settings {
    width: min(560px, 90vw);
    max-height: 90vh;
    padding: 16px 20px;
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border);
  }
  .module-settings::backdrop {
    background: rgb(0 0 0 / 40%);
  }
  h2 {
    margin: 0 0 8px;
    font-size: 15px;
    text-transform: none;
    letter-spacing: normal;
    color: var(--text);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 0;
  }
  .label {
    min-width: 136px;
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
  .checks {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
  }
  .checks label {
    display: inline-flex;
    align-items: center;
    gap: 4px;
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
