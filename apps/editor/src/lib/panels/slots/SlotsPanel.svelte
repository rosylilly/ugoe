<!--
  着せ替えのスロットのパネル (docs/editor.md#スロットの編集)。右の列の「Slots」タブ。
  操作はすべてスロットのコマンド (add_slot / set_slot / remove_slot) と、アプリに見せるスロットの
  update_avatar_profile を送るだけで、組み立ては slots.ts にある。

  - スロットの一覧と「+ Slot」。選んだスロットの各項目を、何のための設定かの説明と一緒に編集する。
  - 「Draw order」に本体のメッシュを描画順に並べ、スロットの範囲を帯で重ねる。行を押すとメッシュを選び、
    「Select meshes in range」で範囲に入るメッシュを選んでビューポートで確かめる。
-->
<script lang="ts">
import { formatList } from "@ugoe/i18n";
import { Button, NumberField, Select, Toggle, Tooltip } from "@ugoe/ui";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { Command } from "../../ipc";
import { elementLabel, elementOptions, labelOf } from "../../labels";
import type { Selection } from "../../selection.svelte";
import TipButton from "../../ui/TipButton.svelte";
import { indexModel, type ModelIndex } from "../tree/build";
import { treeModel } from "../tree/model";
import PickList from "./PickList.svelte";
import {
  DRAW_ORDER_MAX,
  DRAW_ORDER_MIN,
  drawOrderRows,
  exposeCommand,
  finite,
  fullSlot,
  isValidSlotId,
  meshesInRange,
  newSlot,
  overlapping,
  setSlotCommand,
} from "./slots";
import type { SlotsView } from "./slotsView.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  view: SlotsView;
}

let { doc, selection, view }: Props = $props();

const model = $derived(doc.parsedModel);
const index = $derived.by((): ModelIndex | null => {
  try {
    return model ? indexModel(treeModel(model)) : null;
  } catch {
    return null;
  }
});
const slots = $derived(model?.slots ?? []);
const current = $derived(slots.find((s) => s.id === view.slot) ?? null);
const slot = $derived(current ? fullSlot(current) : null);
const exposed = $derived(model?.avatar ? model.avatar.slots : null);
const rows = $derived(index ? drawOrderRows(index, slots) : []);
/** ポーズの要素のパーツ (リンクは含めない)。 */
const posePartIds = $derived(
  model ? model.pose.groups.flatMap((g) => g.entries.map((e) => e.part)) : [],
);
const colliders = $derived(doc.physics?.colliders ?? []);

// 何も選んでいないとき・選んでいたスロットが消えたとき (Undo など) は最初のスロットを選ぶ。
$effect(() => {
  if (model && !current) view.slot = slots[0]?.id ?? null;
});

let busy = $state(false);
let message = $state<string | null>(null);

async function send(command: Command | null): Promise<boolean> {
  if (!command || busy) return false;
  busy = true;
  try {
    // 失敗の理由は doc.error (パネルの上) に出る。
    const ok = (await doc.apply(command)) !== null;
    if (ok) message = null;
    return ok;
  } finally {
    busy = false;
  }
}

function patch(p: Parameters<typeof setSlotCommand>[1]): Promise<boolean> {
  return slot ? send(setSlotCommand(slot, p)) : Promise.resolve(false);
}

async function addSlot(): Promise<void> {
  const created = newSlot(slots);
  if (!created) {
    message = t("slots.noFreeRange");
    return;
  }
  if (await send({ op: "add_slot", slot: created })) view.slot = created.id;
}

async function rename(event: Event): Promise<void> {
  const input = event.currentTarget as HTMLInputElement;
  const id = input.value.trim();
  if (!slot || id === slot.id) return;
  if (!isValidSlotId(id)) {
    message = t("slots.name.invalid");
    input.value = slot.id;
    return;
  }
  if (await patch({ id })) view.slot = id;
  else input.value = slot.id;
}

/** 範囲の欄を今の値に戻すために作り直す (送れなかったとき)。 */
let rangeKey = $state(0);

function setRange(edge: "min" | "max", value: number): void {
  if (!slot || finite(String(value)) === null) {
    rangeKey++;
    return;
  }
  void patch({ draw_order: { ...slot.draw_order, [edge]: value } }).then((ok) => {
    if (!ok) rangeKey++;
  });
}

async function removeSlot(): Promise<void> {
  if (!slot) return;
  const i = slots.findIndex((s) => s.id === slot.id);
  if (await send({ op: "remove_slot", id: slot.id })) {
    view.slot = slots[i + 1]?.id ?? slots[i - 1]?.id ?? null;
  }
}

function selectInRange(): void {
  if (!slot || !index) return;
  selection.set(meshesInRange(index, slot.draw_order).map((id) => ({ kind: "mesh", id })));
}

/** Select の「なし」(Canvas・None) の値。要素の ID には使えない文字にする。 */
const NONE = "\u0000";

function selectValue(v: string): string | null {
  return v === NONE ? null : v;
}

const anchorOptions = $derived([
  { value: NONE, label: t("slots.anchor.canvas") },
  ...elementOptions(model?.deformers ?? []).map((d) => ({ value: d.id, label: d.label })),
]);
const partOptions = $derived([
  { value: NONE, label: t("slots.part.none") },
  ...elementOptions(model?.parts ?? []).map((p) => ({ value: p.id, label: p.label })),
]);
const ids = $props.id();
</script>

<h2>{t("slots.title")}</h2>
<p class="help intro">{t("slots.intro")}</p>

{#if model}
  {#if slots.length > 0}
    <ul class="list">
      {#each slots as s (s.id)}
        {@const overlaps = overlapping(s, slots)}
        <li>
          <button class="item" class:selected={s.id === view.slot} onclick={() => (view.slot = s.id)}>
            {s.id}
            <span class="muted">· {s.draw_order.min}–{s.draw_order.max}{s.anchor ? ` · ${s.anchor}` : ""}</span>
            {#if exposed?.includes(s.id)}
              <Tooltip text={t("slots.tag.app.tooltip")}>
                {#snippet trigger(attrs)}<span {...attrs} class="tag">{t("slots.tag.app.label")}</span>{/snippet}
              </Tooltip>
            {/if}
            {#if overlaps.length > 0}
              <Tooltip text={t("slots.tag.overlap.tooltip", { slots: formatList(overlaps) })}>
                {#snippet trigger(attrs)}<span {...attrs} class="tag bad"
                    >{t("slots.tag.overlap.label")}</span
                  >{/snippet}
              </Tooltip>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="ed-hint">{t("slots.empty")}</p>
  {/if}
  <div class="ed-buttons row">
    <TipButton disabled={busy} onclick={addSlot} tooltip={t("slots.add.tooltip")}>{t("slots.add.label")}</TipButton>
  </div>
  {#if message}<p class="error ugoe-selectable">{message}</p>{/if}

  {#if slot}
    {#key slot.id}
      <section class="editor">
        <h3>{t("slots.slot.title")}</h3>
        <div class="field">
          <label for="{ids}-name">{t("slots.name.label")}</label>
          <input id="{ids}-name" value={slot.id} disabled={busy} onchange={rename} />
        </div>
        <p class="help">{t("slots.name.help")}</p>

        <div class="field">
          <span id="{ids}-anchor">{t("slots.anchor.label")}</span>
          <span class="control">
            <Select
              aria-labelledby="{ids}-anchor"
              value={slot.anchor ?? NONE}
              options={anchorOptions}
              disabled={busy}
              onchange={(v) => patch({ anchor: selectValue(v) })}
            />
          </span>
        </div>
        <p class="help">{t("slots.anchor.help")}</p>

        <div class="field">
          <span id="{ids}-part">{t("slots.part.label")}</span>
          <span class="control">
            <Select
              aria-labelledby="{ids}-part"
              value={slot.part ?? NONE}
              options={partOptions}
              disabled={busy}
              onchange={(v) => patch({ part: selectValue(v) })}
            />
          </span>
        </div>
        <p class="help">{t("slots.part.help")}</p>

        <div class="field">
          <span>{t("slots.drawOrder.label")}</span>
          {#key rangeKey}
            <span class="range">
              <span class="ugoe-visually-hidden" id="{ids}-from">{t("slots.drawOrder.from.aria")}</span>
              <span class="ugoe-visually-hidden" id="{ids}-to">{t("slots.drawOrder.to.aria")}</span>
              <NumberField
                aria-labelledby="{ids}-from"
                step={1}
                size={5}
                value={slot.draw_order.min}
                disabled={busy}
                onchange={(v) => setRange("min", v)}
              />
              –
              <NumberField
                aria-labelledby="{ids}-to"
                step={1}
                size={5}
                value={slot.draw_order.max}
                disabled={busy}
                onchange={(v) => setRange("max", v)}
              />
            </span>
          {/key}
        </div>
        <p class="help">{t("slots.drawOrder.help", { min: DRAW_ORDER_MIN, max: DRAW_ORDER_MAX })}</p>

        <PickList
          title={t("slots.replaces.title")}
          help={t("slots.replaces.help")}
          options={elementOptions(model.parts)}
          selected={slot.replaces}
          empty={t("slots.replaces.empty")}
          disabled={busy}
          onchange={(replaces) => patch({ replaces })}
        />
        <PickList
          title={t("slots.parameters.title")}
          help={t("slots.parameters.help")}
          options={elementOptions(model.parameters)}
          selected={slot.parameters}
          empty={t("slots.parameters.empty")}
          disabled={busy}
          onchange={(parameters) => patch({ parameters })}
        />
        <PickList
          title={t("slots.maskSources.title")}
          help={t("slots.maskSources.help")}
          options={elementOptions(model.meshes)}
          selected={slot.mask_sources}
          empty={t("slots.maskSources.empty")}
          disabled={busy}
          onchange={(mask_sources) => patch({ mask_sources })}
        />
        <PickList
          title={t("slots.colliders.title")}
          help={t("slots.colliders.help")}
          options={colliders.map((c) => ({ id: c.id, label: elementLabel(c.name, c.id) }))}
          selected={slot.colliders}
          empty={t("slots.colliders.empty")}
          disabled={busy}
          onchange={(colliders) => patch({ colliders })}
        />
        <PickList
          title={t("slots.poseParts.title")}
          help={t("slots.poseParts.help")}
          options={posePartIds.map((id) => ({ id, label: labelOf(model.parts, id) }))}
          selected={slot.pose_parts}
          empty={t("slots.poseParts.empty")}
          disabled={busy}
          onchange={(pose_parts) => patch({ pose_parts })}
        />

        <div class="check">
          <Toggle
            label={t("slots.showToApps.label")}
            disabled={busy || exposed === null}
            checked={exposed?.includes(slot.id) ?? false}
            onchange={(checked) => send(exposeCommand(exposed, slot.id, checked))}
          />
        </div>
        <p class="help">
          {exposed === null ? t("slots.showToApps.noProfile") : t("slots.showToApps.help")}
        </p>

        <div class="ed-buttons row">
          <TipButton disabled={busy || !index} onclick={selectInRange} tooltip={t("slots.selectInRange.tooltip")}
            >{t("slots.selectInRange.label")}</TipButton
          >
          <span class="remove">
            <Button disabled={busy} onclick={removeSlot}>{t("slots.remove.label")}</Button>
          </span>
        </div>
      </section>
    {/key}
  {/if}

  <section>
    <h3>{t("slots.drawOrderList.title")}</h3>
    <p class="help">{t("slots.drawOrderList.description")}</p>
    {#if rows.length === 0}
      <p class="ed-hint">{t("slots.drawOrderList.empty")}</p>
    {:else}
      <ul class="orders">
        {#each rows as row (row.kind === "mesh" ? `m:${row.id}` : `s:${row.id}:${row.edge}`)}
          {#if row.kind === "slot"}
            <li class="edge" class:current={row.id === view.slot}>
              <button class="edge-button" onclick={() => (view.slot = row.id)}>
                <span class="order">{row.drawOrder}</span>
                <span
                  >{row.edge === "top"
                    ? t("slots.drawOrderList.frontEnd", { slot: row.id })
                    : t("slots.drawOrderList.backEnd", { slot: row.id })}</span
                >
              </button>
            </li>
          {:else}
            <li class="mesh" class:inside={row.slots.length > 0} class:current={row.slots.includes(view.slot ?? "")}>
              <button
                class="mesh-button"
                class:selected={selection.has({ kind: "mesh", id: row.id })}
                onclick={() => selection.set([{ kind: "mesh", id: row.id }])}
              >
                <span class="order">{row.drawOrder}</span>
                <span class="name">{labelOf(model.meshes, row.id)}</span>
              </button>
            </li>
          {/if}
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
.help {
  margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-sm);
}

.intro {
  margin-bottom: var(--ugoe-space-3);
}

.row {
  margin: var(--ugoe-space-2) 0;
}

.list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.item {
  width: 100%;
  text-align: left;
  border: 1px solid transparent;
  background: none;
  overflow-wrap: anywhere;
}

.item.selected {
  border-color: var(--ugoe-color-accent);
}

.tag {
  margin-left: var(--ugoe-space-2);
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-accent);
  border-radius: var(--ugoe-radius-sm);
  color: var(--ugoe-color-accent);
  font-size: var(--ugoe-font-size-xs);
}

.tag.bad {
  border-color: var(--ugoe-color-danger);
  color: var(--ugoe-color-danger);
}

.editor {
  margin: var(--ugoe-space-3) 0;
  padding: var(--ugoe-space-2) var(--ugoe-space-3) var(--ugoe-space-3);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
}

.editor h3 {
  margin-top: var(--ugoe-space-2);
}

.field {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ugoe-space-3);
  margin: var(--ugoe-space-2) 0 0;
}

.field input,
.control {
  width: 12em;
  min-width: 0;
}

.control {
  display: grid;
}

.range {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
}

.check {
  margin-top: var(--ugoe-space-3);
}

.remove {
  margin-left: auto;
}

.error {
  color: var(--ugoe-color-danger);
  overflow-wrap: anywhere;
}

.orders {
  list-style: none;
  margin: 0;
  padding: 0;
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
}

.orders li {
  margin: 0;
}

.orders button {
  display: flex;
  gap: 8px;
  width: 100%;
  padding: 1px 6px;
  border: none;
  border-radius: 0;
  background: none;
  text-align: left;
}

.mesh.inside {
  background: color-mix(in srgb, var(--ugoe-color-accent) 8%, transparent);
  box-shadow: inset 3px 0 0 color-mix(in srgb, var(--ugoe-color-accent) 40%, transparent);
}

.mesh.current {
  background: color-mix(in srgb, var(--ugoe-color-accent) 18%, transparent);
  box-shadow: inset 3px 0 0 var(--ugoe-color-accent);
}

.mesh-button.selected {
  outline: 1px solid var(--ugoe-color-accent);
  outline-offset: -1px;
}

.orders .order {
  flex: none;
  width: 3.5em;
  color: var(--ugoe-color-text-muted);
  font-variant-numeric: tabular-nums;
}

.mesh .name {
  min-width: 0;
  overflow-wrap: anywhere;
}

.edge {
  border-block: 1px dashed color-mix(in srgb, var(--ugoe-color-accent) 50%, transparent);
}

.edge-button {
  color: var(--ugoe-color-accent);
  font-size: var(--ugoe-font-size-xs);
}

.edge.current {
  border-block-style: solid;
  border-color: var(--ugoe-color-accent);
}

.edge.current .edge-button {
  font-weight: var(--ugoe-font-weight-strong);
}
</style>
