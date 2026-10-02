<!--
  モジュールのプロジェクトの設定 (docs/editor.md#モジュールの設定): 名前・付けるスロット・本体に求める条件 (`requires`)。
  ファイルメニューの `Module: <スロット>` の印を押すと開く。「Apply」で set_module_settings を 1 つ送る (1 回の Undo で戻る)。
  本体がスロットや条件を満たさなければ、バックエンドが拒否し、理由をここに出す。Esc で閉じる。
  取り付け先の違うスロットに移すときは、見た目を保って移すか (`keep_look`、既定) 局所座標のまま移すかを選ぶ (T0651)。
-->
<script lang="ts">
import { Button, Checkbox, Dialog, Select } from "@ugoe/ui";
import { untrack } from "svelte";
import { errorText } from "./errors";
import { t } from "./i18n";
import type { Command, ModuleStatus } from "./ipc";
import {
  anchorName,
  isValidModuleName,
  requirementsOf,
  slotSummary,
  toggled,
} from "./moduleProject";

interface Props {
  open: boolean;
  module: ModuleStatus | null;
  /** コマンドを適用する (失敗すると例外。メッセージを出してダイアログは開いたまま)。 */
  onapply: (command: Command) => Promise<void>;
  onclose: () => void;
}

let { open, module, onapply, onclose }: Props = $props();

let name = $state("");
let slot = $state("");
let avatarLevel = $state<number | null>(null);
let slots = $state<string[]>([]);
let parameters = $state<string[]>([]);
/** 取り付け先の違うスロットに移すとき、見た目 (基準の姿) を保つ。 */
let keepLook = $state(true);
let error = $state<string | null>(null);
let busy = $state(false);
const nameId = $props.id();

// 開くたびに今の設定を欄に入れる。
$effect(() => {
  if (open) untrack(reset);
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
  keepLook = true;
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
const levelOptions = $derived([
  { value: "none", label: t("moduleSettings.avatarLevel.none") },
  ...Array.from({ length: module?.base_avatar_level ?? 0 }, (_, i) => ({
    value: String(i + 1),
    label: t("moduleSettings.avatarLevel.atLeast", { level: i + 1 }),
  })),
]);

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
      keep_look: anchorChanges && keepLook,
    });
    error = null;
  } catch (e) {
    error = errorText(e);
  } finally {
    busy = false;
  }
}
</script>

<Dialog {open} title={t("moduleSettings.title")} size="md" {onclose}>
  {#if module}
    <div class="field">
      <label for={nameId}>{t("moduleSettings.name.label")}</label>
      <input id={nameId} type="text" bind:value={name} />
    </div>
    {#if name.length > 0 && !isValidModuleName(name)}
      <p class="warn">{t("moduleSettings.name.invalid")}</p>
    {:else if renamed}
      <p class="muted">{t("moduleSettings.name.renameHint", { name: name.trim() })}</p>
    {/if}

    <fieldset>
      <legend>{t("moduleSettings.slot.label")}</legend>
      {#each module.base_slots as s (s.id)}
        <label class="slot">
          <input type="radio" name="module-slot" value={s.id} checked={slot === s.id}
            onchange={() => (slot = s.id)} />
          <strong>{s.id}</strong>
          <span class="muted">{slotSummary(s)}</span>
        </label>
      {/each}
      {#if anchorChanges}
        <p class="warn">
          {t("moduleSettings.slot.anchorChanges", {
            to: anchorName(current?.anchor ?? null),
            from: anchorName(module.slot.anchor),
          })}
        </p>
        <Checkbox bind:checked={keepLook} label={t("moduleSettings.keepLook.label")} />
        {#if !keepLook}
          <p class="muted">{t("moduleSettings.keepLook.offHint")}</p>
        {/if}
      {/if}
    </fieldset>

    <fieldset>
      <legend>{t("moduleSettings.requires.label")}</legend>
      <Select
        label={t("moduleSettings.avatarLevel.label")}
        value={avatarLevel === null ? "none" : String(avatarLevel)}
        options={levelOptions}
        onchange={(v) => (avatarLevel = v === "none" ? null : Number(v))}
      />
      {#if baseSlotIds.length > 1}
        <p class="muted">{t("moduleSettings.requires.slots")}</p>
        <div class="checks">
          {#each baseSlotIds.filter((s) => s !== slot) as id (id)}
            <Checkbox
              checked={slots.includes(id)}
              label={id}
              onchange={(on) => (slots = toggled(slots, id, on, baseSlotIds))}
            />
          {/each}
        </div>
      {/if}
      <p class="muted">{t("moduleSettings.requires.parameters")}</p>
      <div class="checks">
        {#each module.base_parameters as id (id)}
          <Checkbox
            checked={parameters.includes(id)}
            label={id}
            onchange={(on) => (parameters = toggled(parameters, id, on, module?.base_parameters ?? []))}
          />
        {/each}
      </div>
    </fieldset>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#snippet actions()}
    <Button onclick={onclose}>{t("moduleSettings.cancel.label")}</Button>
    <Button variant="primary" onclick={apply} disabled={busy || !isValidModuleName(name) || !slot}>
      {t("moduleSettings.apply.label")}
    </Button>
  {/snippet}
</Dialog>

<style>
  p {
    margin: 0;
  }
  .field {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-3);
  }
  .field label {
    display: block;
  }
  .field input {
    flex: 1;
  }
  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
    margin: 0;
    padding: var(--ugoe-space-2) var(--ugoe-space-3) var(--ugoe-space-3);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-md);
  }
  .slot {
    display: flex;
    align-items: baseline;
    gap: var(--ugoe-space-2);
  }
  .checks {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-2) var(--ugoe-space-4);
  }
  .muted {
    color: var(--ugoe-color-text-muted);
  }
  .warn {
    color: var(--ugoe-color-warning);
  }
  .error {
    color: var(--ugoe-color-danger);
    overflow-wrap: anywhere;
  }
</style>
