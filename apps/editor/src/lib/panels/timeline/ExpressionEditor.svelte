<!--
  表情の編集: 今のスライダーの値から表情を作り (既定値から動かしたパラメータを、選んだ op で)、
  名前・フェード・パラメータごとの op と値を編集する。「Preview」(プレビュー) で重み 1 で今の値に重ねて見せる (もう一度押すと戻す)。
  操作はすべて Command (docs/editor.md#表情)。
-->
<script lang="ts">
import { Button, IconButton, NumberField, Select } from "@ugoe/ui";
import { untrack } from "svelte";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { Command, Expression, ExpressionOp, ExpressionParameter } from "../../ipc";
import TipButton from "../../ui/TipButton.svelte";
import type { ViewportController } from "../../viewport/controller.svelte";
import { changedParameters, expressionFromValues, expressionOps, valueFor } from "./expressions";
import { isValidName, parameterLabel, uniqueName } from "./timeline";
import type { TimelineState } from "./timeline.svelte";

interface Props {
  doc: EditorDocument;
  viewport: ViewportController;
  timeline: TimelineState;
}

let { doc, viewport, timeline }: Props = $props();

let selectedName = $state<string | null>(null);
let newOp = $state<ExpressionOp>("add");
let message = $state<string | null>(null);
let renaming = $state("");
let adding = $state("");

const selected = $derived(doc.expressions.find((e) => e.name === selectedName) ?? null);
const ops = $derived(expressionOps());
const expressionOptions = $derived(
  doc.expressions.map((e) => ({ value: e.name, label: label(e) })),
);
const unused = $derived(
  doc.parameters.filter((p) => !selected?.parameters.some((q) => q.parameter === p.id)),
);
const unusedOptions = $derived(unused.map((p) => ({ value: p.id, label: paramLabel(p.id) })));
/** 適用に失敗したとき、数値の欄を今の値に戻すために作り直す (数値の欄は確定した値を出したままになるので)。 */
let fieldsKey = $state(0);
const ids = $props.id();

/** 名前を変えている途中の新しい名前 (読み直しで選択が外れないように、先に覚えておく)。 */
let renamedTo: string | null = null;

// 最初の表情を選ぶ。選んだ表情が消えたら選び直す。名前を変えたら新しい名前を選ぶ。
$effect(() => {
  const list = doc.expressions;
  untrack(() => {
    if (renamedTo !== null && list.some((e) => e.name === renamedTo)) {
      selectedName = renamedTo;
      renamedTo = null;
    } else if (!list.some((e) => e.name === selectedName)) {
      selectedName = list[0]?.name ?? null;
    }
  });
});

// 編集 (と Undo) の後は名前の欄とプレビューを作り直す。
$effect(() => {
  const e = selected;
  untrack(() => {
    renaming = e?.name ?? "";
    timeline.refreshPreview(e);
  });
});

/** 表情を作る基準の値: プレビュー中なら重ねる前の値、そうでなければ今の値。 */
function baseValues(): Record<string, number> {
  return timeline.previewBase ?? viewport.parameterValues;
}

async function apply(command: Command): Promise<boolean> {
  message = null;
  // 失敗の理由は doc.error (タイムラインの見出し) に出る。
  return (await doc.apply(command)) !== null;
}

async function createFromSliders() {
  const values = baseValues();
  const ids = changedParameters(doc.parameters, values);
  if (ids.length === 0) {
    message = t("timeline.expression.nothingMoved");
    return;
  }
  const name = uniqueName(
    "Expression",
    doc.expressions.map((e) => e.name),
  );
  const expression = expressionFromValues(name, doc.parameters, values, ids, newOp);
  timeline.endPreview();
  if (await apply({ op: "add_expression", expression })) selectedName = name;
}

async function remove() {
  if (!selected) return;
  if (timeline.previewExpression === selected.name) timeline.endPreview();
  await apply({ op: "remove_expression", name: selected.name });
}

async function rename() {
  if (!selected || renaming === selected.name) return;
  if (!isValidName(renaming)) {
    message = t("timeline.expression.nameInvalid");
    renaming = selected.name;
    return;
  }
  const old = selected.name;
  const next = renaming;
  if (timeline.previewExpression === old) timeline.previewExpression = next;
  renamedTo = next;
  if (!(await apply({ op: "rename_expression", name: old, new_name: next }))) {
    renamedTo = null;
    renaming = old;
    if (timeline.previewExpression === next) timeline.previewExpression = old;
  }
}

async function setFade(field: "fade_in" | "fade_out", value: number) {
  if (!selected) return;
  if (!(await apply({ op: "set_expression_properties", name: selected.name, [field]: value }))) {
    fieldsKey++;
  }
}

async function setParameters(parameters: ExpressionParameter[]) {
  if (!selected) return;
  if (!(await apply({ op: "set_expression_parameters", name: selected.name, parameters }))) {
    fieldsKey++;
  }
}

function editRow(i: number, patch: Partial<ExpressionParameter>) {
  if (!selected) return;
  setParameters(selected.parameters.map((p, j) => (j === i ? { ...p, ...patch } : p)));
}

function addRow() {
  const p = doc.parameters.find((q) => q.id === adding);
  if (!selected || !p) return;
  const current = baseValues()[p.id] ?? p.default;
  setParameters([
    ...selected.parameters,
    { parameter: p.id, op: "add", value: current - p.default },
  ]);
  adding = "";
}

/** 表の値を今のスライダーの値から作り直す (op は保つ。multiply で作れなければ今の値のまま)。 */
function setFromSliders() {
  if (!selected) return;
  const values = baseValues();
  setParameters(
    selected.parameters.map((row) => {
      const p = doc.parameters.find((q) => q.id === row.parameter);
      if (!p) return row;
      const value = valueFor(row.op, values[p.id] ?? p.default, p.default);
      return value === null ? row : { ...row, value };
    }),
  );
}

function paramLabel(id: string): string {
  const p = doc.parameters.find((q) => q.id === id);
  return p ? parameterLabel(p) : t("timeline.expression.missing", { id });
}

function label(e: Expression): string {
  return t("timeline.expression.option", { name: e.name, count: e.parameters.length });
}
</script>

<div class="expressions">
  <div class="bar">
    <Select
      ariaLabel={t("timeline.expression.select.aria")}
      value={selectedName ?? ""}
      options={expressionOptions}
      placeholder={t("timeline.expression.select.empty")}
      onchange={(v) => (selectedName = v || null)}
    />
    <TipButton small onclick={createFromSliders} tooltip={t("timeline.expression.new.tooltip")}
      >{t("timeline.expression.new.label")}</TipButton
    >
    <Select ariaLabel={t("timeline.expression.newOp.aria")} bind:value={newOp} options={ops} />
    {#if selected}
      <span class="sep"></span>
      <TipButton
        small
        pressed={timeline.previewExpression === selected.name}
        onclick={() => selected && timeline.togglePreview(selected)}
        tooltip={t("timeline.expression.preview.tooltip")}
        >{timeline.previewExpression === selected.name
          ? t("timeline.expression.endPreview.label")
          : t("timeline.expression.preview.label")}</TipButton
      >
      <Button small onclick={remove}>{t("timeline.delete")}</Button>
      <label class="inline" for="{ids}-name">{t("timeline.name.label")}</label>
      <input
        id="{ids}-name"
        class="name"
        bind:value={renaming}
        onchange={rename}
        onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()}
      />
      {#key fieldsKey}
        <NumberField
          label={t("timeline.fadeIn.label")}
          min={0}
          step={0.1}
          decimals={3}
          unit={t("timeline.unit.seconds")}
          value={selected.fade_in}
          onchange={(v) => setFade("fade_in", v)}
        />
        <NumberField
          label={t("timeline.fadeOut.label")}
          min={0}
          step={0.1}
          decimals={3}
          unit={t("timeline.unit.seconds")}
          value={selected.fade_out}
          onchange={(v) => setFade("fade_out", v)}
        />
      {/key}
    {/if}
    {#if message}<span class="error ugoe-selectable">{message}</span>{/if}
  </div>
  {#if selected}
    <div class="table">
      <table>
        <thead>
          <tr
            ><th>{t("timeline.expression.column.parameter")}</th><th
              >{t("timeline.expression.column.op")}</th
            ><th id="{ids}-value">{t("timeline.expression.column.value")}</th><th></th></tr
          >
        </thead>
        <tbody>
          {#key fieldsKey}
          {#each selected.parameters as row, i (row.parameter)}
            <tr>
              <td id="{ids}-row-{i}">{paramLabel(row.parameter)}</td>
              <td>
                <Select
                  ariaLabel={t("timeline.expression.row.op.aria", { parameter: row.parameter })}
                  value={row.op}
                  options={ops}
                  onchange={(op) => editRow(i, { op })}
                />
              </td>
              <td>
                <NumberField
                  step={0.01}
                  decimals={4}
                  size={8}
                  value={row.value}
                  labelledby="{ids}-row-{i} {ids}-value"
                  onchange={(value) => editRow(i, { value })}
                />
              </td>
              <td>
                <IconButton
                  small
                  label={t("timeline.expression.row.remove", { parameter: row.parameter })}
                  onclick={() =>
                    selected && setParameters(selected.parameters.filter((_, j) => j !== i))}
                  >×</IconButton
                >
              </td>
            </tr>
          {/each}
          {/key}
        </tbody>
      </table>
      <div class="bar">
        <Select
          ariaLabel={t("timeline.expression.add.aria")}
          bind:value={adding}
          options={unusedOptions}
          placeholder={t("timeline.expression.add.placeholder")}
        />
        <Button small onclick={addRow} disabled={!adding}>{t("timeline.expression.add.label")}</Button>
        <TipButton small onclick={setFromSliders} tooltip={t("timeline.expression.fromSliders.tooltip")}
          >{t("timeline.expression.fromSliders.label")}</TipButton
        >
      </div>
    </div>
  {:else}
    <p class="muted empty">{t("timeline.expression.empty")}</p>
  {/if}
</div>

<style>
  .expressions {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--ugoe-color-border);
  }
  .inline {
    display: inline;
  }
  input.name {
    width: 120px;
  }
  .sep {
    width: 1px;
    align-self: stretch;
    background: var(--ugoe-color-border);
  }
  .table {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  table {
    border-collapse: collapse;
    margin: 4px 8px;
  }
  th {
    text-align: left;
    font-weight: normal;
    color: var(--ugoe-color-text-muted);
    font-size: 11px;
    padding: 2px 8px;
  }
  td {
    padding: 1px 8px;
  }
  .empty {
    padding: 12px;
  }
</style>
