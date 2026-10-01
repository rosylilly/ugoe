<!--
  グルーのパネル (docs/editor.md#グルーの編集)。右の列の「Glue」タブ。組み立ては glues.ts にある。

  - 一覧: グルーごとに両端のメッシュと組の数。押すと選び、下に編集の欄を出す。「Select meshes」でツリーで両端を選ぶ。
  - 作る: 2 つのメッシュと距離を選んで「+ Glue」(create_glue。頂点の組は editor-core が基準の姿の近さで作る)。
    ツリーで 2 つのメッシュを選んでいれば、それを既定にする。
  - 編集: 重み (どちらへ寄せるか。すべての組に同じ値)・強さ (キーフォームごと)・両端の入れ替え・組の作り直し・削除。
    どれも set_glue でグルーを丸ごと置き換える。
  - 強さの軸 (T0661): パラメータとキーを選んで「+ Axis」、軸ごとにキーを足す・外す (add_keyform_key・remove_keyform_key)。
  - 頂点の組はビューポートに重ね、ビューポートの「Add pairs」「Remove pairs」で組を足す・外す (viewport/tools/glue/GlueOverlay.svelte)。
  - モジュールのプロジェクトでは、本体のグルー (どちらかの端が本体のメッシュ) は読み取り専用 (BaseLock)。つなげるのは
    モジュールのメッシュだけ。描画用の複製のグルーは、インスペクタの「Sync」が作り直すので編集しない。
-->
<script lang="ts">
import BaseLock from "../../BaseLock.svelte";
import type { EditorDocument } from "../../document.svelte";
import type { Command, Glue, Model } from "../../ipc";
import { elementLabel, labelOf } from "../../labels";
import type { Selection } from "../../selection.svelte";
import {
  addGlueKeyCommand,
  createGlueCommand,
  DEFAULT_GLUE_DISTANCE,
  distanceValue,
  glueableMeshes,
  glueWeight,
  isBaseGlue,
  isDrawCloneGlue,
  keyformLabels,
  keyValue,
  removeGlueKeyCommand,
  selectedPair,
  setGlueCommand,
  swapped,
  unboundParameters,
  uniformWeight,
  withIntensity,
  withWeight,
} from "./glues";
import type { GlueView } from "./glueView.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  view: GlueView;
}

let { doc, selection, view }: Props = $props();

const model = $derived.by((): Model | null => {
  try {
    return doc.model ? (JSON.parse(doc.model.json) as Model) : null;
  } catch {
    return null;
  }
});
const module = $derived(doc.module);
const meshes = $derived(model?.meshes ?? []);
const glues = $derived(model?.glues ?? []);
const glueable = $derived(glueableMeshes(meshes, module));
const glue = $derived(glues.find((g) => g.id === view.glue) ?? null);

$effect(() => {
  if (view.glue !== null && !glue) view.glue = null;
});

/** ツリーで選んだメッシュのうち、つなげる 2 つ。 */
const fromSelection = $derived(
  selectedPair(
    selection.items.filter((r) => r.kind === "mesh").map((r) => r.id),
    glueable.map((m) => m.id),
  ),
);

let pickA = $state("");
let pickB = $state("");
let distance = $state(String(DEFAULT_GLUE_DISTANCE));
let repairDistance = $state(String(DEFAULT_GLUE_DISTANCE));
let busy = $state(false);
/** 強さの軸に足すパラメータと、そのキー (軸ごとのキーの入力はパラメータの ID ごと)。 */
let axisParameter = $state("");
let axisKey = $state("");
let newKeys = $state<Record<string, string>>({});

/** 選んでいなければ、ツリーの選択か一覧の先頭 2 つ。 */
const meshA = $derived(
  glueable.some((m) => m.id === pickA) ? pickA : (fromSelection?.[0] ?? glueable[0]?.id ?? ""),
);
const meshB = $derived(
  glueable.some((m) => m.id === pickB)
    ? pickB
    : (fromSelection?.[1] ?? glueable.find((m) => m.id !== meshA)?.id ?? ""),
);
const createDistance = $derived(distanceValue(String(distance)));
const canCreate = $derived(
  meshA !== "" && meshB !== "" && meshA !== meshB && createDistance !== null,
);

async function send(command: Command): Promise<boolean> {
  if (busy) return false;
  busy = true;
  try {
    // 失敗の理由は doc.error (パネルの上) に出る。
    return (await doc.apply(command)) !== null;
  } finally {
    busy = false;
  }
}

async function create(): Promise<void> {
  if (!canCreate || createDistance === null) return;
  const before = new Set(glues.map((g) => g.id));
  if (await send(createGlueCommand(meshA, meshB, createDistance))) {
    // 新しいグルーを選ぶ (ID は editor-core が決める)。
    const next = (model?.glues ?? []).find((g) => !before.has(g.id));
    if (next) view.glue = next.id;
  }
}

function update(next: Glue, pairWithin?: number): void {
  void send(setGlueCommand(next, pairWithin));
}

function number(event: Event): number | null {
  const value = Number((event.currentTarget as HTMLInputElement).value);
  return Number.isFinite(value) ? value : null;
}

function meshLabel(id: string): string {
  return labelOf(meshes, id);
}

function parameterName(id: string): string {
  return doc.parameters.find((p) => p.id === id)?.name || id;
}

/** 強さの軸に足せるパラメータ (選んでいなければ先頭)。 */
const unbound = $derived(glue ? unboundParameters(glue, doc.parameters) : []);
const axisChoice = $derived(unbound.find((p) => p.id === axisParameter) ?? unbound[0] ?? null);
const axisKeyValue = $derived(
  axisChoice ? keyValue(axisKey === "" ? String(axisChoice.default) : axisKey, axisChoice) : null,
);
</script>

<h2>Glue</h2>
<p class="help intro">
  A glue pulls the vertices of two meshes toward each other so seams stay closed when they move (for example the
  edges of a sleeve and an arm). Vertices that are close in the rest pose are paired.
</p>
{#if module}
  <p class="base-note" role="note">
    Module project: glues of the base are read-only. You can glue the module's meshes to each other.
  </p>
{/if}

{#if model}
  {#if glues.length === 0}
    <p class="muted">No glues yet.</p>
  {:else}
    <ul class="list" aria-label="Glues">
      {#each glues as g (g.id)}
        {@const base = isBaseGlue(g, module)}
        <li>
          <button
            class="row-button"
            class:selected={view.glue === g.id}
            aria-pressed={view.glue === g.id}
            onclick={() => (view.glue = view.glue === g.id ? null : g.id)}
          >
            <span class="name">{g.id}</span>
            <span class="muted">{meshLabel(g.mesh_a)} + {meshLabel(g.mesh_b)} · {g.pairs.length} pair{g.pairs
                .length === 1
                ? ""
                : "s"}</span>
          </button>
          {#if base}<span class="tag" title="Belongs to the base model">base</span>{/if}
          {#if isDrawCloneGlue(g)}<span class="tag" title="Keeps a draw clone on its original">clone</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if glue}
    {@const base = isBaseGlue(glue, module)}
    {@const clone = isDrawCloneGlue(glue)}
    <section class="editor" aria-label="Glue {glue.id}">
      <header>
        <h3>{glue.id}</h3>
        <button
          title="Select both meshes in the tree"
          onclick={() =>
            selection.set([
              { kind: "mesh", id: glue.mesh_a },
              { kind: "mesh", id: glue.mesh_b },
            ])}>Select meshes</button
        >
      </header>
      <BaseLock
        locked={base || clone}
        hint={clone
          ? "This glue keeps a draw clone on its original. Use Sync on the clone in the inspector to rebuild it."
          : undefined}
      >
        <div class="field">
          <span>Meshes</span>
          <span class="value">{meshLabel(glue.mesh_a)} + {meshLabel(glue.mesh_b)}</span>
          <button disabled={busy} title="Swap the two meshes (the look stays the same)" onclick={() => update(swapped(glue))}
            >Swap</button
          >
        </div>
        <div class="field">
          <label for="glue-weight" title="0 pulls both toward {meshLabel(glue.mesh_a)}, 1 toward {meshLabel(glue.mesh_b)}"
            >Weight</label
          >
          <input
            id="glue-weight"
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={glueWeight(glue)}
            disabled={busy || glue.pairs.length === 0}
            onchange={(e) => {
              const w = number(e);
              if (w !== null) update(withWeight(glue, w));
            }}
          />
          <span class="value">{glueWeight(glue).toFixed(2)}</span>
        </div>
        <p class="help">
          0 pulls both meshes to {meshLabel(glue.mesh_a)}, 1 to {meshLabel(glue.mesh_b)}, 0.5 meets in the middle.
          {#if !uniformWeight(glue)}The pairs have different weights; changing it sets them all.{/if}
        </p>
        {#each keyformLabels(glue) as label, i (i)}
          <div class="field">
            <label for="glue-intensity-{i}" title="How strongly the vertices are pulled (1 = all the way)">{label}</label>
            <input
              id="glue-intensity-{i}"
              type="number"
              step="0.1"
              value={glue.keyforms.keyforms[i]?.intensity ?? 0}
              disabled={busy}
              onchange={(e) => {
                const v = number(e);
                if (v !== null) update(withIntensity(glue, i, v));
              }}
            />
          </div>
        {/each}
        {#if glue.keyforms.bindings.length > 0}
          <p class="help">The intensity follows the parameters of the keyforms above.</p>
        {/if}
        <div class="axes" aria-label="Intensity axes">
          {#each glue.keyforms.bindings as b (b.parameter)}
            {@const param = doc.parameters.find((p) => p.id === b.parameter)}
            {@const text = newKeys[b.parameter] ?? ""}
            {@const key = param ? keyValue(text, param, b.keys) : null}
            <div class="axis">
              <span class="axis-name">{parameterName(b.parameter)}</span>
              <span class="keys">
                {#each b.keys as k (k)}
                  <span class="chip"
                    >{k}<button
                      class="remove-key"
                      disabled={busy}
                      title="Remove key {k}{b.keys.length === 1 ? ' (removes the axis)' : ''}"
                      onclick={() => send(removeGlueKeyCommand(glue.id, b.parameter, k))}>×</button
                    ></span
                  >
                {/each}
                <input
                  type="number"
                  step="any"
                  placeholder="key"
                  aria-label="New key of {parameterName(b.parameter)}"
                  value={text}
                  disabled={busy}
                  oninput={(e) => (newKeys = { ...newKeys, [b.parameter]: e.currentTarget.value })}
                />
                <button
                  disabled={busy || key === null}
                  title={param ? `Add a key between ${param.min} and ${param.max}` : ""}
                  onclick={async () => {
                    if (key !== null && (await send(addGlueKeyCommand(glue.id, b.parameter, key)))) {
                      newKeys = { ...newKeys, [b.parameter]: "" };
                    }
                  }}>+ Key</button
                >
              </span>
            </div>
          {/each}
          {#if unbound.length > 0}
            <div class="row">
              <select
                aria-label="Parameter for a new intensity axis"
                value={axisChoice?.id ?? ""}
                disabled={busy}
                onchange={(e) => {
                  axisParameter = e.currentTarget.value;
                  axisKey = "";
                }}
              >
                {#each unbound as p (p.id)}<option value={p.id}>{p.name || p.id}</option>{/each}
              </select>
              <input
                type="number"
                step="any"
                aria-label="First key of the new axis"
                value={axisKey === "" ? String(axisChoice?.default ?? 0) : axisKey}
                disabled={busy}
                oninput={(e) => (axisKey = e.currentTarget.value)}
              />
              <button
                disabled={busy || axisChoice === null || axisKeyValue === null}
                title="Make the intensity follow this parameter (the current intensity is kept at this key)"
                onclick={async () => {
                  if (axisChoice && axisKeyValue !== null && (await send(addGlueKeyCommand(glue.id, axisChoice.id, axisKeyValue)))) {
                    axisKey = "";
                  }
                }}>+ Axis</button
              >
            </div>
          {/if}
          <p class="help">
            Add an axis to change the intensity with a parameter (for example 0 when the arm is lowered), then add
            keys and set the intensity of each.
          </p>
        </div>
        <p class="help">
          The pairs are shown in the viewport. Use Add pairs / Remove pairs there to pair vertices by hand.
        </p>
        <div class="row">
          <label for="glue-repair">Pair again within</label>
          <input id="glue-repair" type="number" min="0" step="1" bind:value={repairDistance} disabled={busy} />
          <span>px</span>
          <button
            disabled={busy || distanceValue(String(repairDistance)) === null}
            title="Pair the vertices again by distance in the rest pose (keeps the weight and intensity)"
            onclick={() => {
              const d = distanceValue(String(repairDistance));
              if (d !== null) update(glue, d);
            }}>Pair</button
          >
        </div>
        <div class="row">
          <button
            class="remove"
            disabled={busy}
            onclick={async () => {
              if (await send({ op: "remove_glue", id: glue.id })) view.glue = null;
            }}>Remove glue</button
          >
        </div>
      </BaseLock>
    </section>
  {/if}

  <section class="add">
    <h3>New glue</h3>
    {#if glueable.length < 2}
      <p class="muted">
        {module ? "The module needs two or more meshes to glue." : "The model needs two or more meshes to glue."}
      </p>
    {:else}
      <div class="field">
        <label for="glue-mesh-a">Mesh A</label>
        <select id="glue-mesh-a" value={meshA} disabled={busy} onchange={(e) => (pickA = e.currentTarget.value)}>
          {#each glueable as m (m.id)}<option value={m.id}>{elementLabel(m.name, m.id)}</option>{/each}
        </select>
      </div>
      <div class="field">
        <label for="glue-mesh-b">Mesh B</label>
        <select id="glue-mesh-b" value={meshB} disabled={busy} onchange={(e) => (pickB = e.currentTarget.value)}>
          {#each glueable as m (m.id)}<option value={m.id}>{elementLabel(m.name, m.id)}</option>{/each}
        </select>
      </div>
      <div class="field">
        <label for="glue-distance" title="Pair vertices that are this close in the rest pose (canvas pixels)"
          >Distance (px)</label
        >
        <input id="glue-distance" type="number" min="0" step="1" bind:value={distance} disabled={busy} />
      </div>
      <div class="row">
        <button
          disabled={busy || !canCreate}
          title={meshA === meshB ? "Choose two different meshes" : "Glue the two meshes where their vertices are close"}
          onclick={create}>+ Glue</button
        >
        {#if fromSelection}<span class="muted">From the selected meshes</span>{/if}
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
  overflow-wrap: anywhere;
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

.list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.list li {
  display: flex;
  align-items: center;
  gap: 4px;
  margin: 2px 0;
}

.row-button {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: flex-start;
  min-width: 0;
  border: 1px solid transparent;
  background: none;
  padding: 2px 4px;
  text-align: left;
  overflow-wrap: anywhere;
}

.row-button.selected {
  border-color: var(--accent);
}

.editor,
.add {
  margin: 8px 0;
  padding: 4px 8px 8px;
  border: 1px solid var(--border);
  border-radius: 4px;
}

.editor header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 2px 0 4px;
}

.editor header button {
  margin-left: auto;
}

.field {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 6px 0;
}

.field > :first-child {
  flex: none;
  min-width: 5em;
}

.field input[type="number"],
.field select {
  flex: 1;
  min-width: 0;
}

.field input[type="range"] {
  flex: 1;
  min-width: 0;
}

.value {
  min-width: 0;
  overflow-wrap: anywhere;
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
  margin: 4px 0;
}

.row input {
  width: 5em;
  min-width: 0;
}

.axes {
  margin: 6px 0;
}

.axis {
  display: grid;
  gap: 2px;
  margin-bottom: 6px;
}

.axis-name {
  color: var(--muted);
  font-size: 11px;
  overflow-wrap: anywhere;
}

.keys {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
}

.keys input {
  width: 5em;
  min-width: 0;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding-left: 4px;
  border: 1px solid var(--border);
  border-radius: 4px;
}

.remove-key {
  border: none;
  background: none;
  color: inherit;
  padding: 0 4px;
}

.row select {
  flex: 1;
  min-width: 0;
}

.tag {
  flex: none;
  padding: 0 4px;
  border: 1px solid var(--accent);
  border-radius: 3px;
  color: var(--accent);
  font-size: 10px;
}

.remove {
  margin-left: auto;
}
</style>
