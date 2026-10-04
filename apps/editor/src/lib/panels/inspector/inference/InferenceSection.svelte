<!--
  インスペクタの推定の操作 (docs/editor.md#推定とミラー)。主な選択の要素の格子について、推定の設定
  (加算合成 / 深度からの回転) の変更、推定・ミラーの実行、選んだセルの由来の変更、ミラー元の表示、
  左右の対の設定を行う。操作はすべて Command を送るだけ (inference.ts で組み立てる)。
-->
<script lang="ts">
import { Button, Checkbox, NumberField, Select } from "@ugoe/ui";
import { untrack } from "svelte";
import type { EditorDocument } from "../../../document.svelte";
import { t } from "../../../i18n";
import type { Command, KeyformTarget } from "../../../ipc";
import type { Selection } from "../../../selection.svelte";
import TipButton from "../../../ui/TipButton.svelte";
import type { ViewportController } from "../../../viewport/controller.svelte";
import { currentCell, elementGrid, parseModel } from "../keyforms/keyforms";
import { cellSelection } from "./cellSelection.svelte";
import {
  defaultPivot,
  describeMirrorReport,
  describeMirrorSource,
  formError,
  type InferenceForm,
  inferCommand,
  inferenceForm,
  inferenceFormKey,
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
  targetKey,
  targetName,
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
const pivot = $derived<[number, number]>(
  element && grid ? defaultPivot(element.kind, grid.keyforms[0]) : [0, 0],
);

// 読み直すたびに格子は別のオブジェクトになるので、元の値が変わったとき (要素を替えた・設定を適用した・Undo) だけ
// フォームを作り直す (関係のない編集で、編集中のフォームを消さない)。
const formKey = $derived(
  element ? inferenceFormKey(element.target, inference, bindings, pivot) : "",
);
let form = $state<InferenceForm>(inferenceForm(null, []));
$effect(() => {
  void formKey;
  untrack(() => {
    form = inferenceForm(inference, bindings, pivot);
  });
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
/** 直前の「Mirror」の結果 (書いたセルの数、何も書かなければその理由)。 */
let mirrorResult = $state<{ text: string; overwrite: boolean } | null>(null);
/** 要素の鍵 (読み直しでは変わらない文字列)。 */
const elementKey = $derived(element ? targetKey(element.target) : null);
/** ミラー元の候補の鍵 (対を張り替えると候補の並びが変わる)。 */
const choicesKey = $derived(choices.map((c) => targetKey(c.source)).join("\n"));
$effect(() => {
  // 要素か候補が変わったら先頭 (対の相手) を選び直し、前の結果を消す (読み直しでは選び直さない)。
  void elementKey;
  void choicesKey;
  untrack(() => {
    mirrorFrom = 0;
    mirrorResult = null;
  });
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
const pairAxis = $derived(pair?.axis ?? null);
$effect(() => {
  // 要素か対が変わったときだけ欄を今の値に戻す (読み直しでは戻さない)。
  void elementKey;
  void currentPartner;
  void pairAxis;
  untrack(() => {
    partnerInput = currentPartner;
    pairAxisInput = pairAxis;
  });
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

function label(target: KeyformTarget): string {
  return model ? targetName(model, target) : target.id;
}

/** 選んだ元からミラーし、結果を出す。`overwrite` なら authored のセルも上書きする。 */
async function mirror(overwrite: boolean) {
  if (!chosen || !element) return;
  const report = await doc.apply(
    mirrorCommand(chosen.source, element.target, {
      axis: finiteOrNull(mirrorAxis),
      tolerance: finiteOrNull(mirrorTolerance),
      overwriteAuthored: overwrite,
    }),
  );
  const result = report?.mirrored.at(-1);
  mirrorResult = result ? describeMirrorReport(result, label) : null;
}

/** 選択欄の「なし」(Select の値は空にできないので、ID に使わない文字で表す)。 */
const NONE = ":none";
const ids = $props.id();

const methodOptions = $derived([
  { value: "additive" as const, label: t("inspector.inference.method.option.additive") },
  {
    value: "depth_rotation" as const,
    label: t("inspector.inference.method.option.depthRotation"),
    disabled: !canRotate && form.method !== "depth_rotation",
  },
]);
const axisOptions = $derived([
  { value: NONE, label: t("inspector.inference.axisNone") },
  ...bindings.map((b) => ({ value: b.parameter, label: nameOf(b.parameter) })),
]);
const choiceOptions = $derived(choices.map((c, i) => ({ value: String(i), label: c.label })));
const partnerOptions = $derived([
  { value: NONE, label: t("inspector.inference.partner.none") },
  ...partners.map((p) => ({ value: p.id, label: p.label })),
]);
const markStatus = $derived(
  selected.length > 0
    ? t("inspector.inference.selectedCells", { count: selected.length })
    : current
      ? t("inspector.inference.currentCell")
      : t("inspector.inference.noCell"),
);

function finiteOrNull(value: number | null): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}
</script>

{#if element}
  <section>
    <h3>{t("inspector.inference.title")}</h3>
    <p class="muted">{t("inspector.inference.rederivedBy", { method: methodName(inference) })}</p>

    <div class="settings">
      <div class="ed-field">
        <span id="{ids}-method">{t("inspector.inference.method.label")}</span>
        <Select
          labelledby="{ids}-method"
          value={form.method}
          options={methodOptions}
          onchange={(v) => (form.method = v)}
        />
      </div>
      {#if form.method === "depth_rotation"}
        <div class="ed-field">
          <span id="{ids}-scale">{t("inspector.inference.angleScale.label")}</span>
          <NumberField labelledby="{ids}-scale" step={0.1} decimals={4} bind:value={form.angleScale} />
        </div>
        <div class="ed-field">
          <span id="{ids}-px">{t("inspector.inference.pivotX.label")}</span>
          <NumberField labelledby="{ids}-px" step={1} decimals={4} bind:value={form.pivotX} />
        </div>
        <div class="ed-field">
          <span id="{ids}-py">{t("inspector.inference.pivotY.label")}</span>
          <NumberField labelledby="{ids}-py" step={1} decimals={4} bind:value={form.pivotY} />
        </div>
        <div class="ed-field">
          <span id="{ids}-pd">{t("inspector.inference.pivotDepth.label")}</span>
          <NumberField labelledby="{ids}-pd" step={0.1} decimals={4} bind:value={form.pivotDepth} />
        </div>
        <div class="ed-field">
          <span id="{ids}-yaw">{t("inspector.inference.yaw.label")}</span>
          <Select
            labelledby="{ids}-yaw"
            value={form.yaw || NONE}
            options={axisOptions}
            onchange={(v) => (form.yaw = v === NONE ? "" : v)}
          />
        </div>
        <div class="ed-field">
          <span id="{ids}-pitch">{t("inspector.inference.pitch.label")}</span>
          <Select
            labelledby="{ids}-pitch"
            value={form.pitch || NONE}
            options={axisOptions}
            onchange={(v) => (form.pitch = v === NONE ? "" : v)}
          />
        </div>
      {/if}
      {#if error && edited}<p class="error ugoe-selectable">{error}</p>{/if}
      <span class="row">
        <TipButton
          small
          disabled={!edited || error !== null}
          tooltip={t("inspector.inference.apply.tooltip")}
          onclick={() => send(setInferenceCommand(element.target, inferenceFromForm(form)))}
          >{t("inspector.inference.apply.label")}</TipButton
        >
        {#if edited}
          <Button small onclick={() => (form = inferenceForm(inference, bindings, pivot))}
            >{t("inspector.inference.reset.label")}</Button
          >
        {/if}
      </span>
    </div>

    <span class="row">
      <TipButton
        small
        tooltip={t("inspector.inference.infer.tooltip")}
        onclick={() => send(inferCommand(element.target, false))}
        >{t("inspector.inference.infer.label")}</TipButton
      >
      <TipButton
        small
        tooltip={t("inspector.inference.inferOverwrite.tooltip")}
        onclick={() => send(inferCommand(element.target, true))}
        >{t("inspector.inference.inferOverwrite.label")}</TipButton
      >
    </span>

    <span class="row">
      <span class="muted">{markStatus}</span>
      <TipButton
        small
        disabled={markCells.length === 0}
        tooltip={t("inspector.inference.markAuthored.tooltip")}
        onclick={() => send(markCommand(element.target, markCells, "authored"))}
        >{t("inspector.inference.markAuthored.label")}</TipButton
      >
      <TipButton
        small
        disabled={markCells.length === 0}
        tooltip={t("inspector.inference.markDerived.tooltip")}
        onclick={() => send(markCommand(element.target, markCells, "derived"))}
        >{t("inspector.inference.markDerived.label")}</TipButton
      >
    </span>

    {#if choices.length > 0}
      <h4>{t("inspector.inference.mirror.title")}</h4>
      {#if grid?.mirror_source}
        <p class="source">
          {describeMirrorSource(grid.mirror_source, element.target.id, (id) =>
            label({ ...element.target, id } as KeyformTarget),
          )}
        </p>
      {/if}
      <div class="settings">
        <div class="ed-field">
          <span id="{ids}-from">{t("inspector.inference.mirror.from.label")}</span>
          <Select
            labelledby="{ids}-from"
            value={String(mirrorFrom)}
            options={choiceOptions}
            onchange={(v) => (mirrorFrom = Number(v))}
          />
        </div>
        <!-- 空欄 (既定) に戻せるよう、空にできる数値の欄は標準の入力欄のまま -->
        <label class="ed-field">
          <span>{t("inspector.inference.mirror.axis.label")}</span>
          <input
            type="number"
            step="any"
            placeholder={t("inspector.inference.mirror.axis.placeholder")}
            bind:value={mirrorAxis}
          />
        </label>
        <label class="ed-field">
          <span>{t("inspector.inference.mirror.tolerance.label")}</span>
          <input
            type="number"
            step="any"
            min="0"
            placeholder={t("inspector.inference.mirror.tolerance.placeholder")}
            bind:value={mirrorTolerance}
          />
        </label>
        <Checkbox
          label={t("inspector.inference.mirror.overwriteAuthored.label")}
          bind:checked={overwriteAuthored}
        />
        <span class="row">
          <TipButton
            small
            disabled={!chosen}
            tooltip={t("inspector.inference.mirror.run.tooltip")}
            onclick={() => mirror(overwriteAuthored)}>{t("inspector.inference.mirror.run.label")}</TipButton
          >
        </span>
        {#if mirrorResult}
          <p class="result" class:empty={mirrorResult.overwrite} role="status">{mirrorResult.text}</p>
          {#if mirrorResult.overwrite}
            <span class="row">
              <TipButton
                small
                tooltip={t("inspector.inference.mirror.rerunOverwrite.tooltip")}
                onclick={() => mirror(true)}
                >{t("inspector.inference.mirror.rerunOverwrite.label")}</TipButton
              >
            </span>
          {/if}
        {/if}
      </div>

      <h4>{t("inspector.inference.partner.title")}</h4>
      <div class="settings">
        <div class="ed-field">
          <span id="{ids}-partner">{t("inspector.inference.partner.label")}</span>
          <Select
            labelledby="{ids}-partner"
            value={partnerInput || NONE}
            options={partnerOptions}
            onchange={(v) => (partnerInput = v === NONE ? "" : v)}
          />
        </div>
        <label class="ed-field">
          <span>{t("inspector.inference.partner.axis.label")}</span>
          <input
            type="number"
            step="any"
            placeholder={t("inspector.inference.partner.axis.placeholder")}
            bind:value={pairAxisInput}
          />
        </label>
        <span class="row">
          <TipButton
            small
            disabled={!pairEdited}
            tooltip={t("inspector.inference.partner.set.tooltip")}
            onclick={() =>
              send(setMirrorPairCommand(element.target, partnerInput || null, finiteOrNull(pairAxisInput)))}
            >{t("inspector.inference.partner.set.label")}</TipButton
          >
        </span>
      </div>
    {:else if grid?.mirror_source}
      <p class="source">
        {describeMirrorSource(grid.mirror_source, element.target.id, (id) =>
          label({ ...element.target, id } as KeyformTarget),
        )}
      </p>
    {/if}
  </section>
{/if}

<style>
h4 {
  margin: var(--ugoe-space-4) 0 var(--ugoe-space-2);
  font-size: var(--ugoe-font-size-sm);
  font-weight: var(--ugoe-font-weight-strong);
  color: var(--ugoe-color-text-muted);
}

p {
  margin: var(--ugoe-space-2) 0;
}

.settings {
  display: grid;
  gap: var(--ugoe-space-2);
  margin: var(--ugoe-space-3) 0;
}

.settings .ed-field {
  margin: 0;
}

.settings input[type="number"] {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  min-height: var(--ugoe-control-height);
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-control-border);
  border-radius: var(--ugoe-radius-md);
  background: var(--ugoe-color-surface);
  color: var(--ugoe-color-text);
  font: inherit;
}

.settings input[type="number"]:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: 1px;
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2);
  align-items: center;
  margin: var(--ugoe-space-2) 0;
}

.source {
  font-size: var(--ugoe-font-size-sm);
  color: #2563eb;
  overflow-wrap: anywhere;
}

.result {
  font-size: var(--ugoe-font-size-sm);
  overflow-wrap: anywhere;
}

.result.empty {
  color: var(--ugoe-color-warning);
}
</style>
