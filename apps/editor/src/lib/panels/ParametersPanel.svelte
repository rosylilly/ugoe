<!--
  パラメータ一覧。スライダーの値は表示の状態なので、ViewportController (wasm) を直接操作する (IPC を通らない)。
  パラメータの定義 (作成・削除・名前・値域・既定値) とキーの追加・削除は EditorDocument.apply で Command を送る。
  選択中の要素のキーをスライダーの上に印で示す (docs/editor.md#キーフォームの編集)。
  値の欄に数を入力するとスライダーをその値にする (値域の外は拒否)。◆ はその値にキーを足す。
  中継用 (Parameter::internal) のパラメータは名前を斜体にして「relay」を付ける。編集欄で切り替える
  (set_parameter_internal。docs/spec/model.md#パラメータ)。
  編集欄の「Group」でパラメータをグループに入れ、一覧の下の「Groups」でグループを作る・名前を変える・消す・並べ替える
  (docs/editor.md#パラメータのグループ)。
-->
<script lang="ts">
import { Button, Checkbox, IconButton, NumberField, Tooltip } from "@ugoe/ui";
import BaseLock from "../BaseLock.svelte";
import type { EditorDocument, ParameterDef } from "../document.svelte";
import { t } from "../i18n";
import { baseElements, isBaseElement } from "../moduleProject";
import type { Selection } from "../selection.svelte";
import TipButton from "../ui/TipButton.svelte";
import type { ViewportController } from "../viewport/controller.svelte";
import {
  elementGrid,
  keyAt,
  keyTolerance,
  modelData,
  snapToKeys,
} from "./inspector/keyforms/keyforms";
import ParameterGroupSelect from "./parameters/ParameterGroupSelect.svelte";
import ParameterGroups from "./parameters/ParameterGroups.svelte";
import ParameterMirror from "./parameters/ParameterMirror.svelte";
import { duplicateKeyError, formatValue, parseValue } from "./parameterValue";

interface Props {
  parameters: ParameterDef[];
  viewport: ViewportController;
  doc: EditorDocument;
  selection: Selection;
}

let { parameters, viewport, doc, selection }: Props = $props();

const model = $derived(modelData(doc.parsedModel));
const element = $derived(model ? elementGrid(model, selection.primary) : null);
const bindings = $derived(element?.grid.bindings ?? []);
/** モジュールのプロジェクトでは、パラメータはすべて本体のもの (定義は変えられない。スライダーは動かせる)。 */
const baseParameters = $derived(doc.module !== null);
/** 選択中の要素が本体のもの (キーを足せない)。 */
const elementIsBase = $derived(
  selection.primary !== null && isBaseElement(baseElements(doc.module), selection.primary),
);

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
  const at = span > 0 ? (key - p.min) / span : 0.5;
  return `calc(8px + (100% - 16px) * ${at})`;
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
    const { min, default: def, max } = draft;
    commands.push({ op: "set_parameter_range", id: p.id, min, default: def, max } as const);
  }
  const label = t("parameters.history.edit", { id: p.id });
  if (commands.length === 0 || (await doc.apply({ op: "batch", label, commands })) !== null) {
    editing = null;
  }
}

async function setInternal(p: ParameterDef, internal: boolean) {
  if (internal !== (p.internal ?? false)) {
    await doc.apply({ op: "set_parameter_internal", id: p.id, internal });
  }
}

async function remove(p: ParameterDef) {
  if ((await doc.apply({ op: "remove_parameter", id: p.id })) !== null) editing = null;
}

async function create(e: SubmitEvent) {
  e.preventDefault();
  const id = fresh.id.trim();
  if (!id) return;
  const report = await doc.apply({
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
  if (report !== null) {
    creating = false;
    fresh = { id: "", name: "", min: -1, default: 0, max: 1 };
  }
}

const ids = $props.id();

/** キーの印のボタンの説明。 */
function keyLabel(value: number, onKey: number | undefined, target: string): string {
  if (elementIsBase) return t("parameters.key.base");
  return onKey === undefined
    ? t("parameters.key.add", { value: formatValue(value), target })
    : t("parameters.key.remove", { key: onKey, target });
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
  <h2>{t("parameters.title")}</h2>
  <span class="actions">
    <TipButton
      small
      onclick={snap}
      disabled={bindings.length === 0}
      tooltip={t("parameters.snap.tooltip")}>{t("parameters.snap.label")}</TipButton
    >
    <Button small onclick={() => viewport.resetParameters()}>{t("parameters.reset.label")}</Button>
    <IconButton
      small
      variant="secondary"
      onclick={() => (creating = !creating)}
      disabled={baseParameters}
      label={t("parameters.add.aria")}
      aria-describedby={baseParameters ? `${ids}-add-note` : undefined}>+</IconButton
    >
  </span>
</div>
{#if baseParameters}<p class="ed-hint" id="{ids}-add-note">{t("parameters.add.moduleNote")}</p>{/if}

{#if creating}
  <form class="editor" onsubmit={create}>
    <label
      >{t("parameters.field.id")}
      <!-- i18n-ignore: パラメータの ID の例 (訳さない) -->
      <input bind:value={fresh.id} required placeholder="ParamAngleX" /></label
    >
    <label
      >{t("parameters.field.name")}
      <input bind:value={fresh.name} placeholder={t("parameters.field.namePlaceholder")} /></label
    >
    <span class="range">
      <NumberField label={t("parameters.field.min")} bind:value={fresh.min} decimals={3} size={5} />
      <NumberField
        label={t("parameters.field.default")}
        bind:value={fresh.default}
        decimals={3}
        size={5}
      />
      <NumberField label={t("parameters.field.max")} bind:value={fresh.max} decimals={3} size={5} />
    </span>
    <span class="buttons">
      <Button small type="submit">{t("parameters.create.label")}</Button>
      <Button small onclick={() => (creating = false)}>{t("parameters.cancel.label")}</Button>
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
        <Tooltip
          text={p.internal
            ? t("parameters.edit.tooltipRelay", { id: p.id })
            : t("parameters.edit.tooltip", { id: p.id })}
        >
          {#snippet trigger(attrs)}
            <button
              {...attrs}
              class="name"
              class:internal={p.internal}
              onclick={(e) => {
                (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
                openEditor(p);
              }}
              >{p.name}{#if p.internal}<span class="tag">{t("parameters.relay.tag")}</span>{/if}</button
            >
          {/snippet}
        </Tooltip>
        <span class="row">
          <Tooltip text={t("parameters.value.tooltip", { min: p.min, max: p.max })}>
            {#snippet trigger(attrs)}
              <input
                {...attrs}
                class="value"
                type="text"
                inputmode="decimal"
                value={formatValue(value)}
                aria-label={t("parameters.value.aria", { name: p.name })}
                onchange={(e) => onValueChange(p, e)}
                onkeydown={onValueKeyDown}
              />
            {/snippet}
          </Tooltip>
          {#if element}
            <IconButton
              small
              class={onKey !== undefined ? "key on" : "key"}
              disabled={elementIsBase}
              label={keyLabel(value, onKey, element.target.id)}
              onclick={() => toggleKey(p, onKey)}>◆</IconButton
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
          <Tooltip text={t("parameters.key.marker", { key })}>
            {#snippet trigger(attrs)}
              <button
                {...attrs}
                class="marker"
                class:current={key === onKey}
                style:left={position(p, key)}
                aria-label={t("parameters.key.goTo", { key })}
                onclick={(e) => {
                  (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
                  viewport.setParameter(p.id, key);
                }}
              ></button>
            {/snippet}
          </Tooltip>
        {/each}
      </span>
      {#if valueError?.id === p.id}<p class="error ugoe-selectable">{valueError.message}</p>{/if}
      {#if editing === p.id}
        <BaseLock locked={baseParameters} hint={t("parameters.baseHint")}>
          <div class="editor">
            <label>{t("parameters.field.name")} <input bind:value={draft.name} /></label>
            <span class="range">
              <NumberField
                label={t("parameters.field.min")}
                bind:value={draft.min}
                decimals={3}
                size={5}
              />
              <NumberField
                label={t("parameters.field.default")}
                bind:value={draft.default}
                decimals={3}
                size={5}
              />
              <NumberField
                label={t("parameters.field.max")}
                bind:value={draft.max}
                decimals={3}
                size={5}
              />
            </span>
            <ParameterGroupSelect {doc} parameter={p} groups={doc.parsedModel?.parameter_groups ?? []} />
            <ParameterMirror {doc} parameter={p} {parameters} />
            <Checkbox
              label={t("parameters.relay.label")}
              aria-describedby="{ids}-relay-{p.id}"
              checked={p.internal ?? false}
              onchange={(v) => setInternal(p, v)}
            />
            <p class="ed-hint" id="{ids}-relay-{p.id}">{t("parameters.relay.description")}</p>
            <span class="buttons">
              <Button small onclick={() => saveEditor(p)}>{t("parameters.apply.label")}</Button>
              <Button small variant="danger" onclick={() => remove(p)}
                >{t("parameters.delete.label")}</Button
              >
            </span>
          </div>
        </BaseLock>
      {/if}
    </li>
  {/each}
</ul>

<ParameterGroups {doc} {parameters} />

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
  min-height: var(--ugoe-target-min);
  background: none;
  border: none;
  padding: 0;
  color: inherit;
  cursor: pointer;
  text-align: left;
}

.name.internal {
  font-style: italic;
  color: var(--ugoe-color-text-muted);
}

.tag {
  margin-left: 4px;
  padding: 0 4px;
  border: 1px solid var(--ugoe-color-text-muted);
  border-radius: 3px;
  font-size: 10px;
  font-style: normal;
}

.buttons {
  display: flex;
  gap: 4px;
}

.value {
  width: 5em;
  min-height: var(--ugoe-target-min);
  text-align: right;
}

.row :global(.ugoe-icon-button.key) {
  color: var(--ugoe-color-text-muted);
}

.row :global(.ugoe-icon-button.key.on) {
  color: var(--ugoe-color-accent);
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
  background: var(--ugoe-color-text-muted);
  cursor: pointer;
}

.marker.current {
  background: var(--ugoe-color-accent);
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

.range {
  flex-wrap: wrap;
}
</style>
