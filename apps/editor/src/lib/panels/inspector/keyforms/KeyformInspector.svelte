<!--
  インスペクタのキーの格子 (docs/editor.md#キーフォームの編集)。主な選択の要素について、
  軸とキー・セルの由来 (authored / 推定の方法ごと)・今のパラメータ値がどのセルの上にあるかを示し、
  今のスライダーの値でキーを追加・削除する。セルを押すとスライダーをそのセルへ動かしてそのセルを選び、
  Shift / Ctrl で押すと選択に加える (選んだセルは推定の操作 inference/InferenceSection.svelte が使う)。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import type { Selection } from "../../../selection.svelte";
import type { ViewportController } from "../../../viewport/controller.svelte";
import { cellSelection } from "../inference/cellSelection.svelte";
import { originStyle, originTitle } from "../inference/inference";
import OriginLegend from "../inference/OriginLegend.svelte";
import {
  type Cell,
  cellIndex,
  cellOrigin,
  currentCell,
  elementGrid,
  keyAt,
  keyTolerance,
  type ParameterData,
  parameterValue,
  parseModel,
  snapToKeys,
} from "./keyforms";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
}

let { doc, selection, viewport }: Props = $props();

const model = $derived(doc.model ? parseModel(doc.model.json) : null);
const element = $derived(model ? elementGrid(model, selection.primary) : null);
const params = $derived<ParameterData[]>(model?.parameters ?? []);
const bindings = $derived(element?.grid.bindings ?? []);
const values = $derived(viewport.parameterValues);
const current = $derived(currentCell(bindings, values, params));
/** 表に出す 2 軸 (3 軸目以降は今の値に最も近いキーの断面を出す)。 */
const [colAxis, rowAxis] = $derived([bindings[0], bindings[1]]);
const slice = $derived(snapToKeys(bindings.slice(2), values, params));
const unbound = $derived(params.filter((p) => !bindings.some((b) => b.parameter === p.id)));
let newAxis = $state("");
/** メッシュの描画順 (今のセル、キーの間なら最初のセルの値)。 */
const drawOrder = $derived.by(() => {
  if (element?.kind !== "mesh") return null;
  const index = current ? cellIndex(bindings, current) : 0;
  const keyform = element.grid.keyforms[index] as { draw_order?: number } | undefined;
  return keyform?.draw_order ?? null;
});
let drawOrderInput = $state<number | null>(null);
$effect(() => {
  drawOrderInput = drawOrder;
});

/** 描画順をメッシュのすべてのセルで同じ値にする (set_mesh_draw_order)。 */
async function setDrawOrder() {
  if (element?.target.kind !== "mesh" || drawOrderInput === null) return;
  if (!Number.isFinite(drawOrderInput)) return;
  await doc.apply({ op: "set_mesh_draw_order", id: element.target.id, draw_order: drawOrderInput });
}

function nameOf(id: string): string {
  return params.find((p) => p.id === id)?.name ?? id;
}

function cellFor(col: number | undefined, row: number | undefined): Cell {
  const cell: Cell = { ...slice };
  if (colAxis && col !== undefined) cell[colAxis.parameter] = col;
  if (rowAxis && row !== undefined) cell[rowAxis.parameter] = row;
  return cell;
}

function isCurrent(cell: Cell): boolean {
  return current !== null && bindings.every((b) => current[b.parameter] === cell[b.parameter]);
}

function goTo(cell: Cell) {
  for (const [id, value] of Object.entries(cell)) viewport.setParameter(id, value);
}

/** セルを押したとき: 修飾キーなしなら移動して選び、Shift / Ctrl / Cmd なら選択に加える (外す)。 */
function onCell(event: MouseEvent, cell: Cell) {
  if (!element) return;
  if (event.shiftKey || event.ctrlKey || event.metaKey) {
    cellSelection.toggle(element.target, cell);
    return;
  }
  cellSelection.select(element.target, cell);
  goTo(cell);
}

function onKey(parameter: string): number | undefined {
  const keys = bindings.find((b) => b.parameter === parameter)?.keys ?? [];
  const param = params.find((p) => p.id === parameter);
  return keyAt(keys, parameterValue(parameter, values, params), keyTolerance(param));
}

async function addKey(parameter: string) {
  if (!element) return;
  const key = parameterValue(parameter, values, params);
  await doc.apply({ op: "add_keyform_key", target: element.target, parameter, key });
}

async function removeKey(parameter: string, key: number) {
  if (!element) return;
  await doc.apply({ op: "remove_keyform_key", target: element.target, parameter, key });
}
</script>

{#snippet cellButton(cell: Cell)}
  {@const index = element ? cellIndex(bindings, cell) : -1}
  {@const origin = element && index >= 0 ? cellOrigin(element.grid, index) : "authored"}
  <button
    class="cell"
    data-origin={origin}
    style="--origin: {originStyle(origin).color}"
    class:current={isCurrent(cell)}
    class:selected={element !== null && cellSelection.has(element.target, cell)}
    title="{Object.entries(cell)
      .map(([p, v]) => `${nameOf(p)} = ${v}`)
      .join(', ') || 'Only cell'} — {originTitle(
      origin,
      element?.grid.inference,
      element?.grid.mirror_source,
    )}"
    onclick={(event) => onCell(event, cell)}
  >
    {originStyle(origin).symbol}
  </button>
{/snippet}

{#if element}
  <section>
    <h3>Keyforms</h3>
    {#if current}
      <p class="status">
        On cell
        {bindings.map((b) => `${nameOf(b.parameter)} = ${current[b.parameter]}`).join(", ") ||
          "(no axes)"}
      </p>
    {:else}
      <p class="status muted">
        Between keys.
        <button onclick={() => goTo(snapToKeys(bindings, values, params))}>Move to nearest key</button>
      </p>
    {/if}

    <ul class="axes">
      {#each bindings as b (b.parameter)}
        {@const on = onKey(b.parameter)}
        <li>
          <span class="axis">{nameOf(b.parameter)}</span>
          <span class="keys">
            {#each b.keys as key (key)}
              <span class="chip" class:current={key === on}>
                <button class="go" onclick={() => viewport.setParameter(b.parameter, key)}>{key}</button>
                <button
                  class="remove"
                  title="Remove key {key}{b.keys.length === 1 ? ' (removes the axis)' : ''}"
                  onclick={() => removeKey(b.parameter, key)}>×</button
                >
              </span>
            {/each}
            {#if on === undefined}
              <button
                title="Add a key at the current value"
                onclick={() => addKey(b.parameter)}
              >+ {parameterValue(b.parameter, values, params).toFixed(2)}</button>
            {/if}
          </span>
        </li>
      {/each}
    </ul>

    {#if unbound.length > 0}
      <span class="add-axis">
        <select bind:value={newAxis} aria-label="Parameter for a new axis">
          <option value="" disabled>Add axis…</option>
          {#each unbound as p (p.id)}
            <option value={p.id}>{p.name}</option>
          {/each}
        </select>
        <button
          disabled={!newAxis}
          onclick={async () => {
            await addKey(newAxis);
            newAxis = "";
          }}>Add key at current value</button
        >
      </span>
    {/if}

    <table class="grid">
      {#if rowAxis && colAxis}
        <thead>
          <tr>
            <th class="muted">{nameOf(rowAxis.parameter)} ↓ / {nameOf(colAxis.parameter)} →</th>
            {#each colAxis.keys as col (col)}<th>{col}</th>{/each}
          </tr>
        </thead>
        <tbody>
          {#each rowAxis.keys as row (row)}
            <tr>
              <th>{row}</th>
              {#each colAxis.keys as col (col)}
                <td>{@render cellButton(cellFor(col, row))}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      {:else if colAxis}
        <tbody>
          <tr>
            <th class="muted">{nameOf(colAxis.parameter)}</th>
            {#each colAxis.keys as col (col)}<th>{col}</th>{/each}
          </tr>
          <tr>
            <th></th>
            {#each colAxis.keys as col (col)}<td>{@render cellButton(cellFor(col, undefined))}</td>{/each}
          </tr>
        </tbody>
      {:else}
        <tbody>
          <tr><td>{@render cellButton({})}</td></tr>
        </tbody>
      {/if}
    </table>
    {#if bindings.length > 2}
      <p class="muted">
        Showing the slice at {bindings
          .slice(2)
          .map((b) => `${nameOf(b.parameter)} = ${slice[b.parameter]}`)
          .join(", ")}.
      </p>
    {/if}
    {#if drawOrder !== null}
      <span class="draw-order">
        <label>Draw order <input type="number" step="any" bind:value={drawOrderInput} /></label>
        <button
          disabled={drawOrderInput === drawOrder}
          onclick={setDrawOrder}
          title="Set the draw order in every cell of this mesh">Set for all cells</button
        >
      </span>
    {/if}
    <OriginLegend />
    <p class="hint muted">Click a cell to go there; Shift/Ctrl-click to select several.</p>
  </section>
{/if}

<style>
h3 {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.status {
  margin: 4px 0;
}

.axes li {
  display: grid;
  gap: 2px;
  margin-bottom: 6px;
}

.keys,
.add-axis {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
}

.chip {
  display: inline-flex;
  border: 1px solid var(--border);
  border-radius: 4px;
}

.chip.current {
  border-color: var(--accent);
}

.chip button {
  border: none;
  background: none;
  color: inherit;
  padding: 0 4px;
  cursor: pointer;
}

.grid {
  border-collapse: collapse;
  margin-top: 8px;
}

.grid th,
.grid td {
  padding: 2px;
  text-align: center;
  font-weight: normal;
}

.cell {
  width: 24px;
  height: 24px;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--panel);
  color: inherit;
  cursor: pointer;
}

/* 推定で作ったセル: 方法の色 (--origin) の点線と薄い地。 */
.cell:not([data-origin="authored"]) {
  border: 1px dashed var(--origin);
  background: color-mix(in srgb, var(--origin) 14%, var(--panel));
  color: var(--origin);
}

.cell.current {
  border: 2px solid var(--accent);
}

.cell.selected {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}

.draw-order {
  display: flex;
  gap: 4px;
  align-items: end;
  margin-top: 8px;
}

.draw-order input {
  width: 6em;
}

.hint {
  margin: 4px 0 0;
  font-size: 11px;
}
</style>
