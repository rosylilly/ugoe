<!--
  パラメータ一覧。スライダーの値は表示の状態なので、ViewportController (wasm) を直接操作する (IPC を通らない)。
  パラメータの定義 (作成・削除・名前・値域・既定値) とキーの追加・削除は EditorDocument.apply で Command を送る。
  選択中の要素のキーをスライダーの上に印で示す (docs/editor.md#キーフォームの編集)。
  値の欄に数を入力するとスライダーをその値にする (値域の外は拒否)。◆ はその値にキーを足す。
-->
<script lang="ts">
import type { EditorDocument, ParameterDef } from "../document.svelte";
import type { Selection } from "../selection.svelte";
import type { ViewportController } from "../viewport/controller.svelte";
import {
  elementGrid,
  keyAt,
  keyTolerance,
  parseModel,
  snapToKeys,
} from "./inspector/keyforms/keyforms";
import { duplicateKeyError, formatValue, parseValue } from "./parameterValue";

interface Props {
  parameters: ParameterDef[];
  viewport: ViewportController;
  doc: EditorDocument;
  selection: Selection;
}

let { parameters, viewport, doc, selection }: Props = $props();

const model = $derived(doc.model ? parseModel(doc.model.json) : null);
const element = $derived(model ? elementGrid(model, selection.primary) : null);
const bindings = $derived(element?.grid.bindings ?? []);

/** 定義を編集中のパラメータ (null なら閉じている)。 */
let editing = $state<string | null>(null);
let draft = $state({ name: "", min: 0, default: 0, max: 0 });
let creating = $state(false);
let fresh = $state({ id: "", name: "", min: -1, default: 0, max: 1 });
/** 値の入力やキーの追加の誤り (パラメータごとに 1 つ)。 */
let valueError = $state<{ id: string; message: string } | null>(null);

function onInput(id: string, e: Event) {
  viewport.setParameter(id, (e.currentTarget as HTMLInputElement).valueAsNumber);
  if (valueError?.id === id) valueError = null;
}

/** 値の欄の入力を確定する (Enter・フォーカスが外れたとき)。読めなければ表示を今の値に戻す。 */
function onValueChange(p: ParameterDef, e: Event) {
  const input = e.currentTarget as HTMLInputElement;
  const parsed = parseValue(input.value, p);
  if (!parsed.ok) {
    valueError = { id: p.id, message: parsed.error };
    input.value = formatValue(viewport.parameterValues[p.id] ?? p.default);
    return;
  }
  valueError = null;
  viewport.setParameter(p.id, parsed.value);
  input.value = formatValue(viewport.parameterValues[p.id] ?? parsed.value);
}

function onValueKeyDown(e: KeyboardEvent) {
  if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
}

function keysOf(id: string): number[] {
  return bindings.find((b) => b.parameter === id)?.keys ?? [];
}

/** キーの印の位置。スライダーのつまみ (幅 16px ほど) の中心が動く範囲に合わせる。 */
function position(p: ParameterDef, key: number): string {
  const span = p.max - p.min;
  const t = span > 0 ? (key - p.min) / span : 0.5;
  return `calc(8px + (100% - 16px) * ${t})`;
}

function snap() {
  const snapped = snapToKeys(bindings, viewport.parameterValues, parameters);
  for (const [id, value] of Object.entries(snapped)) viewport.setParameter(id, value);
}

function openEditor(p: ParameterDef) {
  editing = editing === p.id ? null : p.id;
  draft = { name: p.name, min: p.min, default: p.default, max: p.max };
}

async function saveEditor(p: ParameterDef) {
  const commands = [];
  if (draft.name !== p.name)
    commands.push({ op: "rename_parameter", id: p.id, name: draft.name } as const);
  if (draft.min !== p.min || draft.max !== p.max || draft.default !== p.default) {
    commands.push({ op: "set_parameter_range", id: p.id, ...draft } as const);
  }
  if (commands.length > 0) {
    await doc.apply({ op: "batch", label: `Edit parameter ${p.id}`, commands });
  }
  if (!doc.error) editing = null;
}

async function remove(p: ParameterDef) {
  await doc.apply({ op: "remove_parameter", id: p.id });
  if (!doc.error) editing = null;
}

async function create(e: SubmitEvent) {
  e.preventDefault();
  const id = fresh.id.trim();
  if (!id) return;
  await doc.apply({
    op: "add_parameter",
    parameter: {
      ...fresh,
      id,
      name: fresh.name.trim() || id,
      kind: "normal",
      mirror: null,
      group: null,
    },
  });
  if (!doc.error) {
    creating = false;
    fresh = { id: "", name: "", min: -1, default: 0, max: 1 };
  }
}

/** 選択中の要素の、このパラメータの軸のキーを今の値に足す (または今の値のキーを消す)。 */
async function toggleKey(p: ParameterDef, onKey: number | undefined) {
  if (!element) return;
  const value = viewport.parameterValues[p.id] ?? p.default;
  if (onKey === undefined) {
    const error = duplicateKeyError(value, keysOf(p.id), p);
    if (error) {
      valueError = { id: p.id, message: error };
      return;
    }
  }
  valueError = null;
  await doc.apply(
    onKey === undefined
      ? { op: "add_keyform_key", target: element.target, parameter: p.id, key: value }
      : { op: "remove_keyform_key", target: element.target, parameter: p.id, key: onKey },
  );
}
</script>

<div class="head">
  <h2>Parameters</h2>
  <span class="actions">
    <button
      onclick={snap}
      disabled={bindings.length === 0}
      title="Move the selected element's parameters to their nearest keys">Snap to keys</button
    >
    <button onclick={() => viewport.resetParameters()}>Reset</button>
    <button onclick={() => (creating = !creating)} title="New parameter" aria-label="New parameter">+</button>
  </span>
</div>

{#if creating}
  <form class="editor" onsubmit={create}>
    <label>ID <input bind:value={fresh.id} required placeholder="ParamAngleX" /></label>
    <label>Name <input bind:value={fresh.name} placeholder="(same as ID)" /></label>
    <span class="range">
      <label>Min <input type="number" step="any" bind:value={fresh.min} /></label>
      <label>Default <input type="number" step="any" bind:value={fresh.default} /></label>
      <label>Max <input type="number" step="any" bind:value={fresh.max} /></label>
    </span>
    <span class="row">
      <button type="submit">Create</button>
      <button type="button" onclick={() => (creating = false)}>Cancel</button>
    </span>
  </form>
{/if}

<ul>
  {#each parameters as p (p.id)}
    {@const value = viewport.parameterValues[p.id] ?? p.default}
    {@const keys = keysOf(p.id)}
    {@const onKey = keyAt(keys, value, keyTolerance(p))}
    <li>
      <span class="row">
        <button class="name" onclick={() => openEditor(p)} title="Edit {p.id}">{p.name}</button>
        <span class="row">
          <input
            class="value"
            type="text"
            inputmode="decimal"
            value={formatValue(value)}
            aria-label="{p.name} value"
            title="Type a value between {p.min} and {p.max}, then press Enter"
            onchange={(e) => onValueChange(p, e)}
            onkeydown={onValueKeyDown}
          />
          {#if element}
            <button
              class="key"
              class:on={onKey !== undefined}
              onclick={() => toggleKey(p, onKey)}
              title={onKey === undefined
                ? `Add a key at ${formatValue(value)} to ${element.target.id}`
                : `Remove the key ${onKey} from ${element.target.id}`}>◆</button
            >
          {/if}
        </span>
      </span>
      <span class="slider">
        <input
          type="range"
          min={p.min}
          max={p.max}
          step={(p.max - p.min) / 1000 || 0.001}
          {value}
          aria-label={p.name}
          oninput={(e) => onInput(p.id, e)}
        />
        {#each keys as key (key)}
          <button
            class="marker"
            class:current={key === onKey}
            style:left={position(p, key)}
            title="Key {key}"
            aria-label="Go to key {key}"
            onclick={() => viewport.setParameter(p.id, key)}
          ></button>
        {/each}
      </span>
      {#if valueError?.id === p.id}<p class="error">{valueError.message}</p>{/if}
      {#if editing === p.id}
        <div class="editor">
          <label>Name <input bind:value={draft.name} /></label>
          <span class="range">
            <label>Min <input type="number" step="any" bind:value={draft.min} /></label>
            <label>Default <input type="number" step="any" bind:value={draft.default} /></label>
            <label>Max <input type="number" step="any" bind:value={draft.max} /></label>
          </span>
          <span class="row">
            <button onclick={() => saveEditor(p)}>Apply</button>
            <button class="danger" onclick={() => remove(p)} title="Delete {p.id}">Delete</button>
          </span>
        </div>
      {/if}
    </li>
  {/each}
</ul>

<style>
.head,
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
}

.actions {
  display: flex;
  gap: 4px;
}

ul {
  list-style: none;
  padding: 0;
}

li {
  margin-bottom: 8px;
}

.name {
  background: none;
  border: none;
  padding: 0;
  color: inherit;
  cursor: pointer;
  text-align: left;
}

.value {
  width: 5em;
  text-align: right;
}

.key {
  padding: 0 4px;
  opacity: 0.5;
}

.key.on {
  opacity: 1;
  color: var(--accent);
}

.slider {
  position: relative;
  display: block;
}

.slider input {
  width: 100%;
}

.marker {
  position: absolute;
  bottom: -2px;
  width: 6px;
  height: 6px;
  margin-left: -3px;
  padding: 0;
  border: none;
  transform: rotate(45deg);
  background: var(--muted);
  cursor: pointer;
}

.marker.current {
  background: var(--accent);
}

.editor {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 4px 0;
}

.range {
  display: flex;
  gap: 4px;
}

.range input {
  width: 5em;
}

.danger {
  color: #dc2626;
}
</style>
