<!--
  インスペクタの推定の操作 (docs/editor.md#推定とミラー)。主な選択の要素の格子について、推定の設定
  (加算合成 / 深度からの回転) の変更、推定・ミラーの実行、選んだセルの由来の変更、ミラー元の表示、
  左右の対の設定を行う。操作はすべて Command を送るだけ (inference.ts で組み立てる)。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import type { Command } from "../../../ipc";
import type { Selection } from "../../../selection.svelte";
import type { ViewportController } from "../../../viewport/controller.svelte";
import { currentCell, editablePoints, elementGrid, parseModel } from "../keyforms/keyforms";
import { cellSelection } from "./cellSelection.svelte";
import {
  describeMirrorSource,
  formError,
  type InferenceForm,
  inferCommand,
  inferenceForm,
  inferenceFromForm,
  markCommand,
  methodName,
  mirrorChoices,
  mirrorCommand,
  mirrorPair,
  partnerCandidates,
  sameInference,
  setInferenceCommand,
  setMirrorPairCommand,
} from "./inference";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
}

let { doc, selection, viewport }: Props = $props();

const model = $derived(doc.model ? parseModel(doc.model.json) : null);
const element = $derived(model ? elementGrid(model, selection.primary) : null);
const grid = $derived(element?.grid);
const bindings = $derived(grid?.bindings ?? []);
const params = $derived(model?.parameters ?? []);
const inference = $derived(grid?.inference ?? null);
/** 深度からの回転を持てる格子 (メッシュとワープ)。 */
const canRotate = $derived(element?.kind === "mesh" || element?.kind === "warp");

/** 回転の中心の既定値: 基準のセル (先頭) の点の外接矩形の中心。 */
const defaultPivot = $derived.by((): [number, number] => {
  if (!element || !grid) return [0, 0];
  const points = editablePoints(element.kind, grid.keyforms[0]);
  if (points.length === 0) return [0, 0];
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  return [(Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2];
});

let form = $state<InferenceForm>(inferenceForm(null, []));
$effect(() => {
  form = inferenceForm(inference, bindings, defaultPivot);
});
const error = $derived(formError(form));
const edited = $derived(!sameInference(inferenceFromForm(form), inference));

// ミラー
const choices = $derived(model && element ? mirrorChoices(model, element) : []);
let mirrorFrom = $state(0);
const chosen = $derived(choices[mirrorFrom]);
let mirrorAxis = $state<number | null>(null);
let mirrorTolerance = $state<number | null>(null);
let overwriteAuthored = $state(false);
$effect(() => {
  // 要素が変わったら先頭 (対の相手) を選び直す。
  void element?.target.id;
  mirrorFrom = 0;
});

// 左右の対
const pair = $derived(model && element ? mirrorPair(model, element.target) : null);
const partners = $derived(model && element ? partnerCandidates(model, element.target) : []);
/** 今の対の相手 (自分自身と対にしたものは対なしと同じ)。 */
const currentPartner = $derived(
  pair?.mirror && pair.mirror !== element?.target.id ? pair.mirror : "",
);
let partnerInput = $state("");
let pairAxisInput = $state<number | null>(null);
$effect(() => {
  partnerInput = currentPartner;
  pairAxisInput = pair?.axis ?? null;
});
const pairEdited = $derived(
  partnerInput !== currentPartner || finiteOrNull(pairAxisInput) !== (pair?.axis ?? null),
);

// 由来を変えるセル: 選んだセル、なければ今のセル
const selected = $derived(element ? cellSelection.of(element.target) : []);
const current = $derived(currentCell(bindings, viewport.parameterValues, params));
const markCells = $derived(selected.length > 0 ? selected : current ? [current] : []);

function nameOf(id: string): string {
  return params.find((p) => p.id === id)?.name ?? id;
}

async function send(command: Command) {
  await doc.apply(command);
}

function finiteOrNull(value: number | null): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}
</script>

{#if element}
  <section>
    <h3>Inference</h3>
    <p class="muted">Derived cells are re-derived by {methodName(inference)}.</p>

    <div class="settings">
      <label>
        Method
        <select bind:value={form.method}>
          <option value="additive">Additive blending</option>
          <option value="depth_rotation" disabled={!canRotate && form.method !== "depth_rotation"}>
            Depth rotation
          </option>
        </select>
      </label>
      {#if form.method === "depth_rotation"}
        <label>Angle scale <input type="number" step="any" bind:value={form.angleScale} /></label>
        <label>Pivot x <input type="number" step="any" bind:value={form.pivotX} /></label>
        <label>Pivot y <input type="number" step="any" bind:value={form.pivotY} /></label>
        <label>Pivot depth <input type="number" step="any" bind:value={form.pivotDepth} /></label>
        <label>
          Yaw
          <select bind:value={form.yaw}>
            <option value="">(none)</option>
            {#each bindings as b (b.parameter)}<option value={b.parameter}>{nameOf(b.parameter)}</option>{/each}
          </select>
        </label>
        <label>
          Pitch
          <select bind:value={form.pitch}>
            <option value="">(none)</option>
            {#each bindings as b (b.parameter)}<option value={b.parameter}>{nameOf(b.parameter)}</option>{/each}
          </select>
        </label>
      {/if}
      {#if error && edited}<p class="error">{error}</p>{/if}
      <span class="row">
        <button
          disabled={!edited || error !== null}
          title="Set this grid's inference settings and re-derive its derived cells"
          onclick={() => send(setInferenceCommand(element.target, inferenceFromForm(form)))}
          >Apply settings</button
        >
        {#if edited}
          <button onclick={() => (form = inferenceForm(inference, bindings, defaultPivot))}>Reset</button>
        {/if}
      </span>
    </div>

    <span class="row">
      <button
        title="Re-derive the cells made by the grid's method (authored cells are kept)"
        onclick={() => send(inferCommand(element.target, false))}>Infer</button
      >
      <button
        title="Re-derive every derived cell with the grid's method, including cells made by other methods"
        onclick={() => send(inferCommand(element.target, true))}>Infer (overwrite derived)</button
      >
    </span>

    <span class="row">
      <span class="muted">
        {selected.length > 0
          ? `${selected.length} selected cell${selected.length === 1 ? "" : "s"}`
          : current
            ? "Current cell"
            : "No cell (select cells in the grid)"}
      </span>
      <button
        disabled={markCells.length === 0}
        title="Keep these cells as they are; inference will not overwrite them"
        onclick={() => send(markCommand(element.target, markCells, "authored"))}>Mark authored</button
      >
      <button
        disabled={markCells.length === 0}
        title="Let inference re-derive these cells with the grid's method"
        onclick={() => send(markCommand(element.target, markCells, "derived"))}>Mark derived</button
      >
    </span>

    {#if choices.length > 0}
      <h4>Mirror</h4>
      {#if grid?.mirror_source}
        <p class="source">{describeMirrorSource(grid.mirror_source, element.target.id)}</p>
      {/if}
      <div class="settings">
        <label>
          Mirror from
          <select bind:value={mirrorFrom}>
            {#each choices as choice, i (choice.label)}<option value={i}>{choice.label}</option>{/each}
          </select>
        </label>
        <label>Axis <input type="number" step="any" placeholder="default" bind:value={mirrorAxis} /></label>
        <label>
          Tolerance
          <input type="number" step="any" min="0" placeholder="1 px" bind:value={mirrorTolerance} />
        </label>
        <label class="check"><input type="checkbox" bind:checked={overwriteAuthored} /> Overwrite authored cells</label>
        <span class="row">
          <button
            disabled={!chosen}
            title="Flip the source's keyforms into this grid's derived cells"
            onclick={() =>
              chosen &&
              send(
                mirrorCommand(chosen.source, element.target, {
                  axis: finiteOrNull(mirrorAxis),
                  tolerance: finiteOrNull(mirrorTolerance),
                  overwriteAuthored,
                }),
              )}>Mirror</button
          >
        </span>
      </div>

      <h4>Mirror partner</h4>
      <div class="settings">
        <label>
          Partner
          <select bind:value={partnerInput}>
            <option value="">(none, symmetric)</option>
            {#each partners as id (id)}<option value={id}>{id}</option>{/each}
          </select>
        </label>
        <label>
          Mirror axis
          <input type="number" step="any" placeholder="default" bind:value={pairAxisInput} />
        </label>
        <span class="row">
          <button
            disabled={!pairEdited}
            title="Pair both elements (their previous partners are unpaired) and set this element's mirror axis"
            onclick={() =>
              send(setMirrorPairCommand(element.target, partnerInput || null, finiteOrNull(pairAxisInput)))}
            >Set partner</button
          >
        </span>
      </div>
    {:else if grid?.mirror_source}
      <p class="source">{describeMirrorSource(grid.mirror_source, element.target.id)}</p>
    {/if}
  </section>
{/if}

<style>
h3,
h4 {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

h4 {
  margin: 12px 0 4px;
}

p {
  margin: 4px 0;
}

.settings {
  display: grid;
  gap: 4px;
  margin: 6px 0;
}

.settings label {
  display: grid;
  grid-template-columns: 7em 1fr;
  align-items: center;
  gap: 4px;
}

.settings label.check {
  display: flex;
}

.settings input[type="number"] {
  width: 100%;
  min-width: 0;
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
  margin: 4px 0;
}

.source {
  font-size: 12px;
  color: #2563eb;
}
</style>
