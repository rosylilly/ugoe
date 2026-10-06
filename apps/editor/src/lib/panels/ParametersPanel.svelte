<!--
  パラメータ一覧。スライダーの値は表示の状態なので、ViewportController (wasm) を直接操作する (IPC を通らない)。
  パラメータの定義 (作成・削除・名前・値域・既定値) とキーの追加・削除は EditorDocument.apply で Command を送る。
  選択中の要素のキーをスライダーの上に印で示す (docs/editor.md#キーフォームの編集)。
  値の欄に数を入力するとスライダーをその値にする (値域の外は拒否)。◆ はその値にキーを足す。
  中継用 (Parameter::internal) のパラメータは名前を斜体にして「relay」を付ける。編集欄で切り替える
  (set_parameter_internal。docs/spec/model.md#パラメータ)。
  編集欄の「Group」でパラメータをグループに入れ、一覧の下の「Groups」でグループを作る・名前を変える・消す・並べ替える
  (docs/editor.md#パラメータのグループ)。グループのあるモデルでは、一覧をプレビュー・ビューアと同じ規則でグループごとに
  区切る (折りたためる見出しと、グループの無いパラメータの「Other」)。区切りの開閉は localStorage に覚える
  (parameters/sections.ts)。一覧の上の絞り込みの欄で名前と ID から絞る (絞り込み中は開閉によらず一致した行を出す)。
-->
<script lang="ts">
import {
  Button,
  Checkbox,
  Icon,
  IconButton,
  NumberField,
  Section,
  TextField,
  Tooltip,
} from "@ugoe/ui";
import BaseLock from "../BaseLock.svelte";
import type { EditorDocument, ParameterDef } from "../document.svelte";
import { t } from "../i18n";
import { baseElements, isBaseElement } from "../moduleProject";
import { filterSections, groupParameters } from "../preview/preview";
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
import {
  loadClosedSections,
  saveClosedSections,
  sectionKey,
  withSectionOpen,
} from "./parameters/sections";
import {
  duplicateKeyError,
  formatValue,
  markerLeft,
  nearestMarkerKey,
  parseValue,
} from "./parameterValue";

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

/** 一覧の区切り (プレビュー・ビューアと同じ規則。docs/editor.md#パラメータのグループ)。 */
const sections = $derived(
  groupParameters(parameters, doc.parsedModel?.parameter_groups ?? [], t("parameters.ungrouped")),
);
/** グループに入ったパラメータが無ければ (グループの無いモデル)、見出しを出さずに並べる。 */
const grouped = $derived(sections.some((s) => s.id !== null));

/** 絞り込みの語 (名前か ID に含むもの。preview.ts の filterSections)。 */
let filter = $state("");
const filtering = $derived(filter.trim() !== "");
const shown = $derived(filterSections(sections, filter));
/** 閉じた区切り (覚えておく)。 */
let closed = $state(loadClosedSections());
/** 絞り込み中に閉じた区切り (覚えない。語を変えると開き直す)。 */
let closedWhileFiltering = $state(new Set<string>());

function isOpen(id: string | null): boolean {
  const key = sectionKey(id);
  return filtering ? !closedWhileFiltering.has(key) : !closed.has(key);
}

function setOpen(id: string | null, open: boolean) {
  const key = sectionKey(id);
  if (filtering) {
    closedWhileFiltering = withSectionOpen(closedWhileFiltering, key, open);
    return;
  }
  closed = withSectionOpen(closed, key, open);
  saveClosedSections(closed);
}

function onFilterInput(value: string) {
  filter = value;
  closedWhileFiltering = new Set();
}

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

/**
 * キーの印を押したとき: 押した所に中心が最も近い印のキーへ移る (押せる範囲が隣の印と重なる所では近い方。
 * docs/ui.md#そのほか)。キーボードで押した (`detail` が 0) ときは、その印のキー。
 */
function goToMarker(p: ParameterDef, keys: number[], key: number, event: MouseEvent) {
  const strip = (event.currentTarget as HTMLElement).parentElement;
  if (event.detail > 0 && strip) {
    const rect = strip.getBoundingClientRect();
    key = nearestMarkerKey(p, keys, event.clientX - rect.left, rect.width) ?? key;
  }
  viewport.setParameter(p.id, key);
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
    <!-- i18n-ignore: パラメータの ID の例 (訳さない) -->
    <TextField label={t("parameters.field.id")} bind:value={fresh.id} required mono placeholder="ParamAngleX" />
    <TextField
      label={t("parameters.field.name")}
      bind:value={fresh.name}
      placeholder={t("parameters.field.namePlaceholder")}
    />
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

<!-- 空の状態: 何が無いかと次の操作 (docs/ui.md#デザイン の「空の状態」) -->
{#if parameters.length === 0 && !creating}
  <div class="ed-buttons empty">
    <p class="ed-hint">{t("parameters.empty.description")}</p>
    {#if !baseParameters}
      <Button small onclick={() => (creating = true)}>{t("parameters.empty.add")}</Button>
    {/if}
  </div>
{/if}

{#if parameters.length > 0}
  <div class="filter">
    <TextField
      type="search"
      placeholder={t("parameters.filter.placeholder")}
      aria-label={t("parameters.filter.aria")}
      value={filter}
      oninput={onFilterInput}
    />
  </div>
  {#if filtering && shown.length === 0}
    <p class="ed-hint" role="status">{t("parameters.filter.none", { query: filter.trim() })}</p>
  {/if}
{/if}

{#if grouped}
  {#each shown as section (section.id ?? "")}
    <Section title={section.name} open={isOpen(section.id)} onchange={(open) => setOpen(section.id, open)}>
      {#snippet actions()}<span class="count">{section.parameters.length}</span>{/snippet}
      {@render rows(section.parameters)}
    </Section>
  {/each}
{:else}
  {@render rows(shown.flatMap((s) => s.parameters))}
{/if}

<ParameterGroups {doc} {parameters} />

{#snippet rows(list: ParameterDef[])}
  <ul>
    {#each list as p (p.id)}
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
                  aria-invalid={valueError?.id === p.id || undefined}
                  aria-describedby={[attrs["aria-describedby"], valueError?.id === p.id ? `${ids}-value-error` : undefined]
                    .filter(Boolean)
                    .join(" ") || undefined}
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
                onclick={() => toggleKey(p, onKey)}><Icon name="key" /></IconButton
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
          {#if keys.length > 0}
            <span class="markers">
              {#each keys as key (key)}
                <Tooltip text={t("parameters.key.marker", { key })}>
                  {#snippet trigger(attrs)}
                    <button
                      {...attrs}
                      class="marker"
                      class:current={key === onKey}
                      style:left={markerLeft(p, key)}
                      aria-label={t("parameters.key.goTo", { key })}
                      onclick={(e) => {
                        (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
                        goToMarker(p, keys, key, e);
                      }}
                    ></button>
                  {/snippet}
                </Tooltip>
              {/each}
            </span>
          {/if}
        </span>
        {#if valueError?.id === p.id}<p class="error ugoe-selectable" id="{ids}-value-error">{valueError.message}</p>{/if}
        {#if editing === p.id}
          <BaseLock locked={baseParameters} hint={t("parameters.baseHint")}>
            <div class="editor">
              <TextField label={t("parameters.field.name")} bind:value={draft.name} />
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
{/snippet}

<style>
.head,
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ugoe-space-2);
}

.actions {
  display: flex;
  gap: var(--ugoe-space-2);
}

ul {
  list-style: none;
  padding: 0;
}

.filter {
  margin-bottom: var(--ugoe-space-2);
}

.count {
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-sm);
  font-variant-numeric: tabular-nums;
}

li {
  margin-bottom: var(--ugoe-space-3);
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
  margin-left: var(--ugoe-space-2);
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-text-muted);
  border-radius: var(--ugoe-radius-sm);
  font-size: var(--ugoe-font-size-xs);
  font-style: normal;
}

.buttons {
  display: flex;
  gap: var(--ugoe-space-2);
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

/*
 * キーの印の帯: スライダーの下に重ねる。印の押せる範囲は 24×24 で、見た目の◆は上端の 6px (::before)。
 * 帯の上の 4px はスライダーの下の余白に、下の 8px は行の間の余白 (li の margin) に重ねる。
 * 帯そのものは押せない (重なったスライダーを押せるように)。
 */
.markers {
  position: relative;
  display: block;
  height: var(--ugoe-target-min);
  margin: -4px 0 -8px;
  pointer-events: none;
}

.marker {
  position: absolute;
  top: 0;
  width: var(--ugoe-target-min);
  height: var(--ugoe-target-min);
  margin-left: calc(var(--ugoe-target-min) / -2);
  padding: 0;
  border: none;
  background: none;
  cursor: pointer;
  pointer-events: auto;
}

.marker::before {
  content: "";
  position: absolute;
  top: 1px;
  left: calc(50% - 3px);
  width: 6px;
  height: 6px;
  transform: rotate(45deg);
  background: var(--ugoe-color-text-muted);
}

.marker.current::before {
  background: var(--ugoe-color-accent);
}

.marker:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: -2px;
}

.editor {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-2);
  padding: var(--ugoe-space-2) 0;
}

.range {
  display: flex;
  gap: var(--ugoe-space-2);
}

.range {
  flex-wrap: wrap;
}
</style>
