<!--
  ポーズのパネル (docs/editor.md#ポーズの編集)。右の列の「Poses」タブ。
  操作はすべて、ポーズを丸ごと置き換える set_pose を送るだけで、組み立ては poses.ts にある。

  - グループの一覧。グループの要素のうち表示されるのは 1 つで、先頭の要素が最初に表示される (▲▼ で並べ替える)。
    要素の `links` (要素と一緒に切り替わるパーツ。腕の差分に付く袖など) は要素ごとに選ぶ。
  - 「+ Group」「+ Entry」で、ポーズでまだ使っていないパーツを足す。「+ Group from selection」は、ツリーで選んだパーツで
    グループを作る。要素の名前を押すとそのパーツを選ぶ。
  - モジュールのプロジェクトでは、本体のグループ (先頭の `base_pose_groups` 個) は読み取り専用。本体の要素のうちスロットが
    公開するもの (`pose_parts`) にだけ、モジュールのパーツをリンクに加えられる。モジュールのグループは後ろに足す
    (docs/spec/modules.md#ポーズ)。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import type { Model, Pose } from "../../ipc";
import { elementLabel, labelOf } from "../../labels";
import type { Selection } from "../../selection.svelte";
import PickList from "../slots/PickList.svelte";
import {
  addEntry,
  addGroup,
  canLinkTo,
  editableLinks,
  fadeSeconds,
  freeParts,
  isBaseGroup,
  isBasePart,
  moveEntry,
  removeEntry,
  removeGroup,
  setLinks,
  setPoseCommand,
} from "./poses";

interface Props {
  doc: EditorDocument;
  selection: Selection;
}

let { doc, selection }: Props = $props();

const model = $derived.by((): Model | null => {
  try {
    return doc.model ? (JSON.parse(doc.model.json) as Model) : null;
  } catch {
    return null;
  }
});
const module = $derived(doc.module);
const pose = $derived(model?.pose ?? null);
const parts = $derived(model?.parts ?? []);
const free = $derived(pose ? freeParts(parts, pose, module) : []);
/** ツリーで選んだパーツのうち、グループにできるもの。 */
const selectedFree = $derived(
  free.filter((p) => selection.has({ kind: "part", id: p.id })).map((p) => p.id),
);

let busy = $state(false);
/** 「+ Group」「+ Entry」で選んでいるパーツ (グループの番号ごと。-1 は新しいグループ)。 */
let picks = $state<Record<number, string>>({});

async function send(next: Pose): Promise<boolean> {
  if (busy) return false;
  busy = true;
  try {
    // 失敗の理由は doc.error (パネルの上) に出る。
    return (await doc.apply(setPoseCommand(next))) !== null;
  } finally {
    busy = false;
  }
}

function pickOf(group: number): string {
  const current = picks[group];
  return current && free.some((p) => p.id === current) ? current : (free[0]?.id ?? "");
}

function setFade(event: Event): void {
  const input = event.currentTarget as HTMLInputElement;
  const value = fadeSeconds(input.value);
  if (!pose || value === null) {
    if (pose) input.value = String(pose.fade_duration);
    return;
  }
  void send({ ...pose, fade_duration: value });
}

function select(part: string): void {
  selection.set([{ kind: "part", id: part }]);
}

/** リンクに選べるパーツ: 使っていないものと、今のリンク (外せるように)。 */
function linkOptions(current: readonly string[]): { id: string; label: string }[] {
  const ids = [...free.map((p) => p.id), ...current.filter((id) => !free.some((p) => p.id === id))];
  return ids.map((id) => ({ id, label: labelOf(parts, id) }));
}
</script>

<h2>Poses</h2>
<p class="help intro">
  A pose group switches between variants of a part (for example arms A / B): only one entry of a group shows at a
  time, and the first entry shows at start. Links fade in and out with their entry (for example sleeves with arms).
  Motions choose the entry by animating part opacity; apps can also pick one.
</p>
{#if module}
  <p class="base-note" role="note">
    Module project: the base's pose groups are read-only. You can add groups of the module's parts, and link the
    module's parts to the base entries that the slot exposes (Pose parts in the base's Slots tab).
  </p>
{/if}

{#if model && pose}
  <div class="field">
    <label for="pose-fade">Fade time (s)</label>
    <input
      id="pose-fade"
      type="number"
      min="0"
      step="0.1"
      value={pose.fade_duration}
      disabled={busy || module !== null}
      title={module ? "The fade time belongs to the base model." : "Cross-fade time when the shown entry changes"}
      onchange={setFade}
    />
  </div>

  {#if pose.groups.length === 0}
    <p class="muted">No pose groups yet.</p>
  {/if}
  {#each pose.groups as group, gi (gi)}
    {@const base = isBaseGroup(module, gi)}
    <section class="group" class:base aria-label="Pose group {gi + 1}">
      <header>
        <h3>Group {gi + 1}</h3>
        {#if base}<span class="tag" title="Belongs to the base model">base</span>{/if}
        <button
          class="remove"
          disabled={busy || base}
          title={base ? "Base groups can't be removed here." : "Remove this group (the parts stay)"}
          onclick={() => send(removeGroup(pose, gi))}>Remove group</button>
      </header>
      <ol class="entries">
        {#each group.entries as entry, ei (entry.part)}
          {@const linkable = !base || canLinkTo(module, entry.part)}
          <li>
            <div class="entry">
              <button
                class="name"
                class:selected={selection.has({ kind: "part", id: entry.part })}
                title="Select the part"
                onclick={() => select(entry.part)}>{labelOf(parts, entry.part)}</button>
              {#if ei === 0}<span class="tag" title="Shown at start">first</span>{/if}
              <span class="buttons">
                <button
                  disabled={busy || base || ei === 0}
                  aria-label="Move up"
                  title="Move up (the first entry shows at start)"
                  onclick={() => send(moveEntry(pose, gi, ei, -1))}>▲</button>
                <button
                  disabled={busy || base || ei === group.entries.length - 1}
                  aria-label="Move down"
                  title="Move down"
                  onclick={() => send(moveEntry(pose, gi, ei, 1))}>▼</button>
                <button
                  disabled={busy || base}
                  aria-label="Remove entry"
                  title={base ? "Base entries can't be removed here." : "Remove this entry from the group"}
                  onclick={() => send(removeEntry(pose, gi, ei))}>✕</button>
              </span>
            </div>
            {#if base && entry.links.some((l) => isBasePart(module, l))}
              <p class="help">
                Base links: {entry.links
                  .filter((l) => isBasePart(module, l))
                  .map((l) => labelOf(parts, l))
                  .join(", ")}
              </p>
            {/if}
            {#if linkable}
              <PickList
                title="Links"
                help={base
                  ? "Module parts that switch with this base entry (for example sleeves A / B with arms A / B)."
                  : "Parts that switch together with this entry (for example sleeves with arms)."}
                options={linkOptions(editableLinks(entry.links, module))}
                selected={editableLinks(entry.links, module)}
                empty="No free parts. A part can be in only one place in the pose."
                disabled={busy}
                onchange={(links) => send(setLinks(pose, gi, ei, links, module))}
              />
            {:else}
              <p class="help">The slot doesn't expose this entry, so module parts can't follow it.</p>
            {/if}
          </li>
        {/each}
      </ol>
      {#if !base}
        <div class="row">
          <select
            aria-label="Part to add to group {gi + 1}"
            value={pickOf(gi)}
            disabled={busy || free.length === 0}
            onchange={(e) => (picks[gi] = e.currentTarget.value)}
          >
            {#each free as p (p.id)}<option value={p.id}>{elementLabel(p.name, p.id)}</option>{/each}
          </select>
          <button
            disabled={busy || free.length === 0}
            title="Add the part as another variant in this group"
            onclick={() => send(addEntry(pose, gi, pickOf(gi)))}>+ Entry</button>
        </div>
      {/if}
    </section>
  {/each}

  <section class="add">
    <h3>New group</h3>
    {#if free.length === 0}
      <p class="muted">
        {module ? "Every module part is already in the pose, or the module has no parts." : "Every part is already in the pose."}
      </p>
    {:else}
      <div class="row">
        <select
          aria-label="First part of the new group"
          value={pickOf(-1)}
          disabled={busy}
          onchange={(e) => (picks[-1] = e.currentTarget.value)}
        >
          {#each free as p (p.id)}<option value={p.id}>{elementLabel(p.name, p.id)}</option>{/each}
        </select>
        <button disabled={busy} onclick={() => send(addGroup(pose, pickOf(-1)))}>+ Group</button>
      </div>
      <div class="row">
        <button
          disabled={busy || selectedFree.length < 2}
          title={selectedFree.length < 2
            ? "Select two or more parts that aren't in the pose yet (in the tree)"
            : `Make a group of ${selectedFree.length} selected parts`}
          onclick={() => {
            const [first, ...rest] = selectedFree;
            if (first === undefined) return;
            send(rest.reduce((p, id) => addEntry(p, p.groups.length - 1, id), addGroup(pose, first)));
          }}>+ Group from selection</button>
      </div>
    {/if}
  </section>
{/if}

<style>
h3 {
  margin: 0;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.help {
  margin: 2px 0 6px;
  color: var(--muted);
  font-size: 11px;
  overflow-wrap: anywhere;
}

.intro {
  margin-bottom: 8px;
}

.base-note {
  margin: 4px 0 8px;
  padding: 4px 8px;
  border-left: 3px solid var(--border);
  font-size: 12px;
  opacity: 0.85;
}

.field {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 6px 0;
}

.field input {
  width: 6em;
  min-width: 0;
}

.group,
.add {
  margin: 8px 0;
  padding: 4px 8px 8px;
  border: 1px solid var(--border);
  border-radius: 4px;
}

.group.base {
  border-style: dashed;
}

.group header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 2px 0 4px;
}

.remove {
  margin-left: auto;
}

.entries {
  margin: 0;
  padding-left: 18px;
}

.entries li {
  margin: 4px 0;
}

.entry {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
}

.entry .name {
  min-width: 0;
  border: 1px solid transparent;
  background: none;
  padding: 0 2px;
  text-align: left;
  overflow-wrap: anywhere;
}

.entry .name.selected {
  border-color: var(--accent);
}

.buttons {
  display: flex;
  flex: none;
  gap: 2px;
  margin-left: auto;
}

.tag {
  flex: none;
  padding: 0 4px;
  border: 1px solid var(--accent);
  border-radius: 3px;
  color: var(--accent);
  font-size: 10px;
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
  margin: 4px 0;
}

.row select {
  flex: 1;
  min-width: 0;
}
</style>
