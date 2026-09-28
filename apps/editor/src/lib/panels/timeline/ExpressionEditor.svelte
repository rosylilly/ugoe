<!--
  表情の編集: 今のスライダーの値から表情を作り (既定値から動かしたパラメータを、選んだ op で)、
  名前・フェード・パラメータごとの op と値を編集する。「Preview」で重み 1 で今の値に重ねて見せる (もう一度押すと戻す)。
  操作はすべて Command (docs/editor.md#表情)。
-->
<script lang="ts">
import { untrack } from "svelte";
import type { EditorDocument } from "../../document.svelte";
import type { Command, Expression, ExpressionOp, ExpressionParameter } from "../../ipc";
import type { ViewportController } from "../../viewport/controller.svelte";
import { changedParameters, EXPRESSION_OPS, expressionFromValues, valueFor } from "./expressions";
import { isValidName, uniqueName } from "./timeline";
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
const unused = $derived(
  doc.parameters.filter((p) => !selected?.parameters.some((q) => q.parameter === p.id)),
);

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
  await doc.apply(command);
  return !doc.error;
}

async function createFromSliders() {
  const values = baseValues();
  const ids = changedParameters(doc.parameters, values);
  if (ids.length === 0) {
    message = "Move some sliders away from their defaults first.";
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
    message = "Use letters, digits, _, - and . (not starting with .) in expression names.";
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

async function setFade(field: "fade_in" | "fade_out", e: Event) {
  if (!selected) return;
  const input = e.currentTarget as HTMLInputElement;
  if (!Number.isFinite(input.valueAsNumber)) return;
  if (
    !(await apply({
      op: "set_expression_properties",
      name: selected.name,
      [field]: input.valueAsNumber,
    }))
  ) {
    input.value = String(selected[field]);
  }
}

async function setParameters(parameters: ExpressionParameter[]) {
  if (!selected) return;
  await apply({ op: "set_expression_parameters", name: selected.name, parameters });
}

function editRow(i: number, patch: Partial<ExpressionParameter>) {
  if (!selected) return;
  setParameters(selected.parameters.map((p, j) => (j === i ? { ...p, ...patch } : p)));
}

function onValue(i: number, e: Event) {
  const value = (e.currentTarget as HTMLInputElement).valueAsNumber;
  if (Number.isFinite(value)) editRow(i, { value });
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
  if (!p) return `${id} (missing)`;
  return p.name && p.name !== p.id ? `${p.name} (${p.id})` : p.id;
}

function label(e: Expression): string {
  return `${e.name} (${e.parameters.length})`;
}
</script>

<div class="expressions">
  <div class="bar">
    <select
      aria-label="Expression"
      value={selectedName ?? ""}
      onchange={(e) => (selectedName = (e.currentTarget as HTMLSelectElement).value || null)}
    >
      {#if doc.expressions.length === 0}<option value="">(no expressions)</option>{/if}
      {#each doc.expressions as e (e.name)}
        <option value={e.name}>{label(e)}</option>
      {/each}
    </select>
    <button onclick={createFromSliders} title="Create an expression from the sliders moved away from their defaults"
      >New from sliders</button
    >
    <select aria-label="Operation for the new expression" bind:value={newOp}>
      {#each EXPRESSION_OPS as o (o.op)}<option value={o.op}>{o.label}</option>{/each}
    </select>
    {#if selected}
      <span class="sep"></span>
      <button
        class:on={timeline.previewExpression === selected.name}
        onclick={() => selected && timeline.togglePreview(selected)}
        title="Show the expression over the current sliders (press again to restore them)"
        >{timeline.previewExpression === selected.name ? "End preview" : "Preview"}</button
      >
      <button onclick={remove}>Delete</button>
      <label class="inline"
        >Name <input
          class="name"
          bind:value={renaming}
          onchange={rename}
          onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()}
        /></label
      >
      <label class="inline"
        >Fade in <input
          type="number"
          min="0"
          step="0.1"
          value={selected.fade_in}
          onchange={(e) => setFade("fade_in", e)}
        />s</label
      >
      <label class="inline"
        >Fade out <input
          type="number"
          min="0"
          step="0.1"
          value={selected.fade_out}
          onchange={(e) => setFade("fade_out", e)}
        />s</label
      >
    {/if}
    {#if message}<span class="error">{message}</span>{/if}
  </div>
  {#if selected}
    <div class="table">
      <table>
        <thead>
          <tr><th>Parameter</th><th>Operation</th><th>Value</th><th></th></tr>
        </thead>
        <tbody>
          {#each selected.parameters as row, i (row.parameter)}
            <tr>
              <td>{paramLabel(row.parameter)}</td>
              <td>
                <select
                  aria-label="{row.parameter} operation"
                  value={row.op}
                  onchange={(e) => editRow(i, { op: (e.currentTarget as HTMLSelectElement).value as ExpressionOp })}
                >
                  {#each EXPRESSION_OPS as o (o.op)}<option value={o.op}>{o.label}</option>{/each}
                </select>
              </td>
              <td>
                <input
                  type="number"
                  step="any"
                  value={row.value}
                  aria-label="{row.parameter} value"
                  onchange={(e) => onValue(i, e)}
                />
              </td>
              <td>
                <button
                  onclick={() => selected && setParameters(selected.parameters.filter((_, j) => j !== i))}
                  aria-label="Remove {row.parameter}">×</button
                >
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      <div class="bar">
        <select aria-label="Parameter to add" bind:value={adding}>
          <option value="">Add parameter…</option>
          {#each unused as p (p.id)}<option value={p.id}>{paramLabel(p.id)}</option>{/each}
        </select>
        <button onclick={addRow} disabled={!adding}>Add</button>
        <button onclick={setFromSliders} title="Recompute the values from the current sliders (keeping each operation)"
          >Set values from sliders</button
        >
      </div>
    </div>
  {:else}
    <p class="muted empty">
      No expressions yet. Move the sliders to make a face, then press “New from sliders”.
    </p>
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
    border-bottom: 1px solid var(--border);
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  input[type="number"] {
    width: 72px;
  }
  input.name {
    width: 120px;
  }
  .sep {
    width: 1px;
    align-self: stretch;
    background: var(--border);
  }
  button.on {
    background: var(--accent);
    color: white;
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
    color: var(--muted);
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
