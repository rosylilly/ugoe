<!--
  着せ替えのスロットのパネル (docs/editor.md#スロットの編集)。右の列の「Slots」タブ。
  操作はすべてスロットのコマンド (add_slot / set_slot / remove_slot) と、アプリに見せるスロットの
  update_avatar_profile を送るだけで、組み立ては slots.ts にある。

  - スロットの一覧と「+ Slot」。選んだスロットの各項目を、何のための設定かの説明と一緒に編集する。
  - 「Draw order」に本体のメッシュを描画順に並べ、スロットの範囲を帯で重ねる。行を押すとメッシュを選び、
    「Select meshes in range」で範囲に入るメッシュを選んでビューポートで確かめる。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import type { Command, Model } from "../../ipc";
import { elementLabel, elementOptions, labelOf } from "../../labels";
import type { Selection } from "../../selection.svelte";
import { indexModel, type ModelIndex } from "../tree/build";
import { parseTreeModel } from "../tree/model";
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

const model = $derived.by((): Model | null => {
  try {
    return doc.model ? (JSON.parse(doc.model.json) as Model) : null;
  } catch {
    return null;
  }
});
const index = $derived.by((): ModelIndex | null => {
  try {
    return doc.model ? indexModel(parseTreeModel(doc.model.json)) : null;
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
    message = "No free draw order range between 0 and 1000. Narrow another slot first.";
    return;
  }
  if (await send({ op: "add_slot", slot: created })) view.slot = created.id;
}

async function rename(event: Event): Promise<void> {
  const input = event.currentTarget as HTMLInputElement;
  const id = input.value.trim();
  if (!slot || id === slot.id) return;
  if (!isValidSlotId(id)) {
    message =
      "Slot names may use only letters, digits and _ (modules refer to the slot by this name).";
    input.value = slot.id;
    return;
  }
  if (await patch({ id })) view.slot = id;
  else input.value = slot.id;
}

function setRange(edge: "min" | "max", event: Event): void {
  const input = event.currentTarget as HTMLInputElement;
  const value = finite(input.value);
  if (!slot || value === null) {
    if (slot) input.value = String(slot.draw_order[edge]);
    return;
  }
  void patch({ draw_order: { ...slot.draw_order, [edge]: value } }).then((ok) => {
    if (!ok && slot) input.value = String(slot.draw_order[edge]);
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

function selectValue(event: Event): string | null {
  const v = (event.currentTarget as HTMLSelectElement).value;
  return v === "" ? null : v;
}
</script>

<h2>Slots</h2>
<p class="help intro">
  Slots are the places where dress-up modules (outfits, hairstyles, accessories made as separate module
  projects) attach to this model. A module names only its slot, so define here where it attaches, where
  it draws, and which parts of this model it may use.
</p>

{#if model}
  {#if slots.length > 0}
    <ul class="list">
      {#each slots as s (s.id)}
        {@const overlaps = overlapping(s, slots)}
        <li>
          <button class="item" class:selected={s.id === view.slot} onclick={() => (view.slot = s.id)}>
            {s.id}
            <span class="muted">· {s.draw_order.min}–{s.draw_order.max}{s.anchor ? ` · ${s.anchor}` : ""}</span>
            {#if exposed?.includes(s.id)}<span class="tag" title="Listed in the avatar profile">app</span>{/if}
            {#if overlaps.length > 0}
              <span class="tag bad" title="Its draw order range overlaps {overlaps.join(', ')}">overlap</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted">No slots yet.</p>
  {/if}
  <div class="row">
    <button disabled={busy} onclick={addSlot} title="Add a slot in a free draw order range">+ Slot</button>
  </div>
  {#if message}<p class="error">{message}</p>{/if}

  {#if slot}
    {#key slot.id}
      <section class="editor">
        <h3>Slot</h3>
        <label class="field">
          <span>Name</span>
          <input value={slot.id} disabled={busy} onchange={rename} aria-label="Slot name" />
        </label>
        <p class="help">Modules refer to the slot by this name (e.g. Hair, Outfit, HeadAccessory).</p>

        <label class="field">
          <span>Attach to</span>
          <select value={slot.anchor ?? ""} disabled={busy} onchange={(e) => patch({ anchor: selectValue(e) })}>
            <option value="">Canvas</option>
            {#each elementOptions(model.deformers) as d (d.id)}
              <option value={d.id}>{d.label}</option>
            {/each}
          </select>
        </label>
        <p class="help">
          The deformer the module moves with (e.g. the head for hair). The module's top-level meshes and
          deformers are placed in this deformer's space.
        </p>

        <label class="field">
          <span>Parent part</span>
          <select value={slot.part ?? ""} disabled={busy} onchange={(e) => patch({ part: selectValue(e) })}>
            <option value="">None</option>
            {#each elementOptions(model.parts) as p (p.id)}
              <option value={p.id}>{p.label}</option>
            {/each}
          </select>
        </label>
        <p class="help">
          The module's parts become children of this part, so they fade out with it (poses, motions).
        </p>

        <div class="field">
          <span>Draw order</span>
          <span class="range">
            <input
              type="number"
              step="1"
              value={slot.draw_order.min}
              disabled={busy}
              aria-label="Draw order from"
              onchange={(e) => setRange("min", e)}
            />
            –
            <input
              type="number"
              step="1"
              value={slot.draw_order.max}
              disabled={busy}
              aria-label="Draw order to"
              onchange={(e) => setRange("max", e)}
            />
          </span>
        </div>
        <p class="help">
          The module's meshes (draw order {DRAW_ORDER_MIN}–{DRAW_ORDER_MAX}) are squeezed into this range, so they
          draw in front of this model's meshes below it and behind those above it. Ranges of slots must not
          overlap. See “Draw order” below.
        </p>

        <PickList
          title="Replaces"
          help="Parts of this model hidden while a module is attached (e.g. the default hair)."
          options={elementOptions(model.parts)}
          selected={slot.replaces}
          empty="The model has no parts."
          disabled={busy}
          onchange={(replaces) => patch({ replaces })}
        />
        <PickList
          title="Parameters"
          help="This model's own parameters the module may animate with. Standard parameters (AngleX, EyeLOpen, …) are always available and need not be ticked."
          options={elementOptions(model.parameters)}
          selected={slot.parameters}
          empty="The model has no parameters."
          disabled={busy}
          onchange={(parameters) => patch({ parameters })}
        />
        <PickList
          title="Mask sources"
          help="Meshes of this model the module may clip to (e.g. the face, to keep bangs inside it)."
          options={elementOptions(model.meshes)}
          selected={slot.mask_sources}
          empty="The model has no meshes."
          disabled={busy}
          onchange={(mask_sources) => patch({ mask_sources })}
        />
        <PickList
          title="Colliders"
          help="Physics colliders of this model the module's physics may hit (e.g. the neck, so long hair drapes over it)."
          options={colliders.map((c) => ({ id: c.id, label: elementLabel(c.name, c.id) }))}
          selected={slot.colliders}
          empty="The model has no colliders. Add them in the Physics tab."
          disabled={busy}
          onchange={(colliders) => patch({ colliders })}
        />
        <PickList
          title="Pose parts"
          help="Parts in this model's pose groups the module's parts may follow (e.g. sleeves A / B follow arms A / B)."
          options={posePartIds.map((id) => ({ id, label: labelOf(model.parts, id) }))}
          selected={slot.pose_parts}
          empty="The model has no pose groups."
          disabled={busy}
          onchange={(pose_parts) => patch({ pose_parts })}
        />

        <label class="check">
          <input
            type="checkbox"
            disabled={busy || exposed === null}
            checked={exposed?.includes(slot.id) ?? false}
            onchange={(e) => send(exposeCommand(exposed, slot.id, e.currentTarget.checked))}
          />
          Show to apps
        </label>
        <p class="help">
          {exposed === null
            ? "Lists the slot in the avatar profile so apps offer it to users. Create an avatar profile first (Inspector with nothing selected)."
            : "Lists the slot in the avatar profile so apps offer it to users. Unlisted slots are for authors only."}
        </p>

        <div class="row">
          <button disabled={busy || !index} onclick={selectInRange}
            title="Select this model's meshes whose draw order is in the range, to see them in the viewport"
            >Select meshes in range</button>
          <button class="remove" disabled={busy} onclick={removeSlot}>Remove slot</button>
        </div>
      </section>
    {/key}
  {/if}

  <section>
    <h3>Draw order</h3>
    <p class="help">
      This model's meshes from front to back (at default parameters), with the slots' ranges. Meshes inside
      a range interleave with the module; the module draws between the meshes above and below the range.
    </p>
    {#if rows.length === 0}
      <p class="muted">The model has no meshes.</p>
    {:else}
      <ul class="orders">
        {#each rows as row (row.kind === "mesh" ? `m:${row.id}` : `s:${row.id}:${row.edge}`)}
          {#if row.kind === "slot"}
            <li class="edge" class:current={row.id === view.slot}>
              <button class="edge-button" onclick={() => (view.slot = row.id)}>
                <span class="order">{row.drawOrder}</span>
                <span>{row.id} · {row.edge === "top" ? "front end ▼" : "back end ▲"}</span>
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
h3 {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.help {
  margin: 2px 0 6px;
  color: var(--muted);
  font-size: 11px;
}

.intro {
  margin-bottom: 8px;
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
  margin: 4px 0;
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
  border-color: var(--accent);
}

.tag {
  margin-left: 4px;
  padding: 0 4px;
  border: 1px solid var(--accent);
  border-radius: 3px;
  color: var(--accent);
  font-size: 10px;
}

.tag.bad {
  border-color: #dc2626;
  color: #dc2626;
}

.editor {
  margin: 8px 0;
  padding: 4px 8px 8px;
  border: 1px solid var(--border);
  border-radius: 4px;
}

.field {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 6px 0 0;
}

.field select,
.field input:not([type="number"]) {
  width: 12em;
  min-width: 0;
}

.range {
  display: flex;
  align-items: center;
  gap: 4px;
}

.range input {
  width: 5em;
  min-width: 0;
}

.check {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
}

.remove {
  margin-left: auto;
}

.error {
  color: #dc2626;
  overflow-wrap: anywhere;
}

.orders {
  list-style: none;
  margin: 0;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 4px;
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
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  box-shadow: inset 3px 0 0 color-mix(in srgb, var(--accent) 40%, transparent);
}

.mesh.current {
  background: color-mix(in srgb, var(--accent) 18%, transparent);
  box-shadow: inset 3px 0 0 var(--accent);
}

.mesh-button.selected {
  outline: 1px solid var(--accent);
  outline-offset: -1px;
}

.orders .order {
  flex: none;
  width: 3.5em;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}

.mesh .name {
  min-width: 0;
  overflow-wrap: anywhere;
}

.edge {
  border-block: 1px dashed color-mix(in srgb, var(--accent) 50%, transparent);
}

.edge-button {
  color: var(--accent);
  font-size: 11px;
}

.edge.current {
  border-block-style: solid;
  border-color: var(--accent);
}

.edge.current .edge-button {
  font-weight: 600;
}
</style>
