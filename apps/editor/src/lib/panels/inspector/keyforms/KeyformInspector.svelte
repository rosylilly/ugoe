<!--
  インスペクタのキーの格子 (docs/editor.md#キーフォームの編集)。主な選択の要素について、
  軸とキー・セルの由来 (authored / 推定の方法ごと)・今のパラメータ値がどのセルの上にあるかを示し、
  今のスライダーの値でキーを追加・削除する。セルを押すとスライダーをそのセルへ動かしてそのセルを選び、
  Shift / Ctrl で押すと選択に加える (選んだセルは推定の操作 inference/InferenceSection.svelte が使う)。
-->
<script lang="ts">
import { formatList } from "@ugoe/i18n";
import { Button, Checkbox, NumberField, Select, Tooltip } from "@ugoe/ui";
import type { EditorDocument } from "../../../document.svelte";
import { t } from "../../../i18n";
import type { Selection } from "../../../selection.svelte";
import TipButton from "../../../ui/TipButton.svelte";
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
  gridInterpolation,
  keyAt,
  keyTolerance,
  modelData,
  type ParameterData,
  parameterValue,
  snapToKeys,
  supportsSmooth,
} from "./keyforms";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
}

let { doc, selection, viewport }: Props = $props();

const model = $derived(modelData(doc.parsedModel));
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

const interpolation = $derived(element ? gridInterpolation(element.grid) : "linear");

/** キーの間を曲線で補間するか (set_keyform_interpolation、docs/spec/evaluation.md#曲線補間)。 */
async function setSmooth(smooth: boolean) {
  if (!element) return;
  await doc.apply({
    op: "set_keyform_interpolation",
    target: element.target,
    interpolation: smooth ? "smooth" : "linear",
  });
}

/** 描画順をメッシュのすべてのセルで同じ値にする (set_mesh_draw_order)。 */
async function setDrawOrder() {
  if (element?.target.kind !== "mesh" || drawOrderInput === null) return;
  if (!Number.isFinite(drawOrderInput)) return;
  await doc.apply({ op: "set_mesh_draw_order", id: element.target.id, draw_order: drawOrderInput });
}

function nameOf(id: string): string {
  return params.find((p) => p.id === id)?.name ?? id;
}

/** セルの表記 (`AngleX = 30、AngleY = 0`)。軸が無ければ空。 */
function cellText(cell: Cell, axes: readonly string[] = Object.keys(cell)): string {
  return formatList(
    axes.map((p) => `${nameOf(p)} = ${cell[p]}`),
    { type: "unit" },
  );
}

const ids = $props.id();
const axisOptions = $derived(unbound.map((p) => ({ value: p.id, label: p.name })));

/** Tooltip のトリガーの属性と自前の onclick を重ねる (トリガーの onclick も呼ぶ)。 */
function withClick(
  attrs: Record<string, unknown>,
  onclick: (event: MouseEvent) => void,
): Record<string, unknown> {
  return {
    ...attrs,
    onclick: (event: MouseEvent) => {
      (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(event);
      onclick(event);
    },
  };
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
  {@const tip = t("inspector.keyforms.cellTooltip", {
    cell: cellText(cell) || t("inspector.keyforms.onlyCell"),
    origin: originTitle(origin, element?.grid.inference, element?.grid.mirror_source),
  })}
  <Tooltip text={tip}>
    {#snippet children(attrs)}
      <button
        {...withClick(attrs, (event) => onCell(event, cell))}
        class="cell"
        data-origin={origin}
        style="--origin: {originStyle(origin).color}"
        class:current={isCurrent(cell)}
        class:selected={element !== null && cellSelection.has(element.target, cell)}
        aria-label={tip}
        aria-describedby={undefined}
      >
        {originStyle(origin).symbol}
      </button>
    {/snippet}
  </Tooltip>
{/snippet}

{#if element}
  <section>
    <h3>{t("inspector.keyforms.title")}</h3>
    {#if current}
      <p class="status">
        {t("inspector.keyforms.onCell", {
          cell:
            cellText(
              current,
              bindings.map((b) => b.parameter),
            ) || t("inspector.keyforms.noAxes"),
        })}
      </p>
    {:else}
      <p class="status muted">
        {t("inspector.keyforms.betweenKeys")}
        <Button small onclick={() => goTo(snapToKeys(bindings, values, params))}
          >{t("inspector.keyforms.moveToNearest.label")}</Button
        >
      </p>
    {/if}

    <ul class="axes">
      {#each bindings as b (b.parameter)}
        {@const on = onKey(b.parameter)}
        <li>
          <span class="axis">{nameOf(b.parameter)}</span>
          <span class="keys">
            {#each b.keys as key (key)}
              {@const removeTip =
                b.keys.length === 1
                  ? t("inspector.keyforms.removeKey.tooltipLast", { key })
                  : t("inspector.keyforms.removeKey.tooltip", { key })}
              <span class="chip" class:current={key === on}>
                <button
                  class="go"
                  aria-label={t("inspector.keyforms.key.aria", { key })}
                  onclick={() => viewport.setParameter(b.parameter, key)}>{key}</button
                >
                <Tooltip text={removeTip}>
                  {#snippet children(attrs)}
                    <button
                      {...withClick(attrs, () => removeKey(b.parameter, key))}
                      class="remove"
                      aria-label={removeTip}
                      aria-describedby={undefined}>×</button
                    >
                  {/snippet}
                </Tooltip>
              </span>
            {/each}
            {#if on === undefined}
              <TipButton
                small
                tooltip={t("inspector.keyforms.addKeyHere.tooltip")}
                onclick={() => addKey(b.parameter)}
                >+ {parameterValue(b.parameter, values, params).toFixed(2)}</TipButton
              >
            {/if}
          </span>
        </li>
      {/each}
    </ul>

    {#if supportsSmooth(element.kind)}
      <div class="interpolation">
        <Checkbox
          label={t("inspector.keyforms.smooth.label")}
          checked={interpolation === "smooth"}
          describedby="{ids}-smooth"
          onchange={(v) => setSmooth(v)}
        />
        {#if interpolation === "unknown"}<span class="muted">{t("inspector.keyforms.smooth.unknown")}</span>{/if}
        <p class="ed-hint" id="{ids}-smooth">{t("inspector.keyforms.smooth.description")}</p>
      </div>
    {/if}

    {#if unbound.length > 0}
      <span class="add-axis">
        <Select
          ariaLabel={t("inspector.keyforms.newAxis.aria")}
          placeholder={t("inspector.keyforms.newAxis.placeholder")}
          bind:value={newAxis}
          options={axisOptions}
        />
        <Button
          small
          disabled={!newAxis}
          onclick={async () => {
            await addKey(newAxis);
            newAxis = "";
          }}>{t("inspector.keyforms.addKey.label")}</Button
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
        {t("inspector.keyforms.slice", {
          cell: cellText(
            slice,
            bindings.slice(2).map((b) => b.parameter),
          ),
        })}
      </p>
    {/if}
    {#if drawOrder !== null}
      <span class="draw-order">
        <NumberField
          label={t("inspector.keyforms.drawOrder.label")}
          step={1}
          decimals={4}
          value={drawOrderInput ?? 0}
          onchange={(v) => (drawOrderInput = v)}
        />
        <TipButton
          small
          disabled={drawOrderInput === drawOrder}
          onclick={setDrawOrder}
          tooltip={t("inspector.keyforms.drawOrder.setAll.tooltip")}
          >{t("inspector.keyforms.drawOrder.setAll.label")}</TipButton
        >
      </span>
    {/if}
    <OriginLegend />
    <p class="hint muted">{t("inspector.keyforms.hint")}</p>
  </section>
{/if}

<style>
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
  border: 1px solid var(--ugoe-color-border);
  border-radius: 4px;
}

.chip.current {
  border-color: var(--ugoe-color-accent);
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
  border: 1px solid var(--ugoe-color-border);
  border-radius: 4px;
  background: var(--ugoe-color-surface);
  color: inherit;
  cursor: pointer;
}

/* 推定で作ったセル: 方法の色 (--origin) の点線と薄い地。 */
.cell:not([data-origin="authored"]) {
  border: 1px dashed var(--origin);
  background: color-mix(in srgb, var(--origin) 14%, var(--ugoe-color-surface));
  color: var(--origin);
}

.cell.current {
  border: 2px solid var(--ugoe-color-accent);
}

.cell.selected {
  outline: 2px solid var(--ugoe-color-accent);
  outline-offset: 1px;
}

.interpolation {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2);
  align-items: center;
  margin-bottom: var(--ugoe-space-3);
}

.interpolation .ed-hint {
  flex-basis: 100%;
}

.draw-order {
  display: flex;
  gap: 4px;
  align-items: center;
  margin-top: 8px;
}

.hint {
  margin: 4px 0 0;
  font-size: 11px;
}
</style>
