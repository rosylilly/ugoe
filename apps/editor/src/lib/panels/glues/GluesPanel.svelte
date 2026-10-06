<!--
  グルーのパネル (docs/editor.md#グルーの編集)。右の列の「Glue」タブ。組み立ては glues.ts にある。

  - 一覧: グルーごとに両端のメッシュと組の数。押すと選び、下に編集の欄を出す。「Select meshes」でツリーで両端を選ぶ。
  - 作る: 2 つのメッシュと距離を選んで「+ Glue」(create_glue。頂点の組は editor-core が基準の姿の近さで作る)。
    ツリーで 2 つのメッシュを選んでいれば、それを既定にする。
  - 編集: 重み (どちらへ寄せるか。すべての組に同じ値)・強さ (キーフォームごと)・両端の入れ替え・組の作り直し・削除。
    どれも set_glue でグルーを丸ごと置き換えるので、送る直前のグルーから組み立てる (updateGlue)。
  - 強さの軸 (T0661): パラメータとキーを選んで「+ Axis」、軸ごとにキーを足す・外す (add_keyform_key・remove_keyform_key)。
  - 頂点の組はビューポートに重ね、ビューポートの「Add pairs」「Remove pairs」で組を足す・外す (viewport/tools/glue/GlueOverlay.svelte)。
  - モジュールのプロジェクトでは、本体のグルー (どちらかの端が本体のメッシュ) は読み取り専用 (BaseLock)。つなげるのは
    モジュールのメッシュだけ。描画用の複製のグルーは、インスペクタの「Sync」が作り直すので編集しない。
-->
<script lang="ts">
import { Button, Icon, IconButton, NumberField, Select, Slider, Tooltip } from "@ugoe/ui";
import BaseLock from "../../BaseLock.svelte";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { Command, Glue } from "../../ipc";
import { elementLabel, labelOf } from "../../labels";
import type { Selection } from "../../selection.svelte";
import TipButton from "../../ui/TipButton.svelte";
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

const model = $derived(doc.parsedModel);
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
let distance = $state(DEFAULT_GLUE_DISTANCE);
let repairDistance = $state(DEFAULT_GLUE_DISTANCE);
/** 編集を送っている間は、ボタンを無効にする (欄の編集は列に並べて送る。docs/editor.md#パネルからの編集)。 */
const busy = $derived(doc.pending > 0);
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

/** 前の編集を読み直した後に `build` でコマンドを組み立てて送る。失敗の理由は doc.error (パネルの上) に出る。 */
async function send(build: () => Command | null): Promise<boolean> {
  return (await doc.send(build)) !== null;
}

async function create(): Promise<void> {
  if (!canCreate || createDistance === null) return;
  const [a, b, d] = [meshA, meshB, createDistance];
  let before = new Set<string>();
  const ok = await send(() => {
    before = new Set((model?.glues ?? []).map((g) => g.id));
    return createGlueCommand(a, b, d);
  });
  if (ok) {
    // 新しいグルーを選ぶ (ID は editor-core が決める)。
    const next = (model?.glues ?? []).find((g) => !before.has(g.id));
    if (next) view.glue = next.id;
  }
}

/** 送る直前のグルー `id` を `change` で作り直して set_glue を送る (消えていれば送らない)。 */
function updateGlue(id: string, change: (current: Glue) => Glue, pairWithin?: number): void {
  void send(() => {
    const current = model?.glues.find((g) => g.id === id);
    return current ? setGlueCommand(change(current), pairWithin) : null;
  });
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
const meshOptions = $derived(
  glueable.map((m) => ({ value: m.id, label: elementLabel(m.name, m.id) })),
);
const axisOptions = $derived(unbound.map((p) => ({ value: p.id, label: p.name || p.id })));
const axisKeyValue = $derived(
  axisChoice ? keyValue(axisKey === "" ? String(axisChoice.default) : axisKey, axisChoice) : null,
);
</script>

<!-- 右の列のタブと同じ名前なので見た目では隠す (読み上げと見出しの移動に残す。App.svelte) -->
<h2 class="ugoe-visually-hidden">{t("glues.title")}</h2>
<p class="ed-hint intro">{t("glues.intro")}</p>
{#if module}
  <p class="base-note" role="note">{t("glues.moduleNote")}</p>
{/if}

{#if model}
  {#if glues.length === 0}
    <p class="ed-hint">{t("glues.empty")}</p>
  {:else}
    <ul class="list" aria-label={t("glues.list.aria")}>
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
            <span class="muted"
              >{t("glues.list.summary", {
                count: g.pairs.length,
                a: meshLabel(g.mesh_a),
                b: meshLabel(g.mesh_b),
              })}</span
            >
          </button>
          {#if base}{@render tag(t("glues.tag.base.label"), t("glues.tag.base.tooltip"))}{/if}
          {#if isDrawCloneGlue(g)}{@render tag(t("glues.tag.clone.label"), t("glues.tag.clone.tooltip"))}{/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if glue}
    {@const base = isBaseGlue(glue, module)}
    {@const clone = isDrawCloneGlue(glue)}
    {@const locked = base || clone}
    <section class="editor" aria-label={t("glues.editor.aria", { id: glue.id })}>
      <header>
        <h3>{glue.id}</h3>
        <TipButton
          small
          tooltip={t("glues.selectMeshes.tooltip")}
          onclick={() =>
            selection.set([
              { kind: "mesh", id: glue.mesh_a },
              { kind: "mesh", id: glue.mesh_b },
            ])}>{t("glues.selectMeshes.label")}</TipButton
        >
      </header>
      <BaseLock locked={locked} hint={clone ? t("glues.cloneHint") : undefined}>
        <div class="ed-field">
          <span>{t("glues.meshes.label")}</span>
          <span class="meshes">
            <span class="value">{meshLabel(glue.mesh_a)} + {meshLabel(glue.mesh_b)}</span>
            <TipButton small disabled={busy} tooltip={t("glues.swap.tooltip")} onclick={() => updateGlue(glue.id, swapped)}
              >{t("glues.swap.label")}</TipButton
            >
          </span>
        </div>
        <!-- つまみを離したとき・キーで動かしたとき・欄で確定したときに送る (oncommit)。 -->
        <Slider
          label={t("glues.weight.label")}
          min={0}
          max={1}
          step={0.05}
          decimals={2}
          value={glueWeight(glue)}
          disabled={locked || glue.pairs.length === 0}
          oncommit={(w) => updateGlue(glue.id, (g) => withWeight(g, w))}
        />
        <p class="ed-hint help">
          {t("glues.weight.description", { a: meshLabel(glue.mesh_a), b: meshLabel(glue.mesh_b) })}
          {#if !uniformWeight(glue)}{t("glues.weight.mixed")}{/if}
        </p>
        {#each keyformLabels(glue) as label, i (i)}
          <NumberField
            {label}
            step={0.1}
            decimals={4}
            aria-describedby="glue-intensity-hint"
            value={glue.keyforms.keyforms[i]?.intensity ?? 0}
            disabled={locked}
            onchange={(v) => updateGlue(glue.id, (g) => withIntensity(g, i, v))}
          />
        {/each}
        <p class="ed-hint help" id="glue-intensity-hint">{t("glues.intensity.description")}</p>
        {#if glue.keyforms.bindings.length > 0}
          <p class="ed-hint help">{t("glues.intensity.follows")}</p>
        {/if}
        <div class="axes" role="group" aria-label={t("glues.axes.aria")}>
          {#each glue.keyforms.bindings as b (b.parameter)}
            {@const param = doc.parameters.find((p) => p.id === b.parameter)}
            {@const text = newKeys[b.parameter] ?? ""}
            {@const key = param ? keyValue(text, param, b.keys) : null}
            <div class="axis">
              <span class="axis-name">{parameterName(b.parameter)}</span>
              <span class="keys">
                {#each b.keys as k (k)}
                  <span class="chip"
                    >{k}<IconButton
                      small
                      variant="ghost"
                      disabled={busy}
                      label={b.keys.length === 1
                        ? t("glues.removeKey.lastAria", { key: k })
                        : t("glues.removeKey.aria", { key: k })}
                      onclick={() => send(() => removeGlueKeyCommand(glue.id, b.parameter, k))}><Icon name="close" /></IconButton
                    ></span
                  >
                {/each}
                <!-- 空の欄 (キーを入れていない) を持てるよう、数値の欄は標準の input のまま。 -->
                <input
                  type="number"
                  step="any"
                  placeholder={t("glues.newKey.placeholder")}
                  aria-label={t("glues.newKey.aria", { parameter: parameterName(b.parameter) })}
                  value={text}
                  oninput={(e) => (newKeys = { ...newKeys, [b.parameter]: e.currentTarget.value })}
                />
                <TipButton
                  small
                  disabled={busy || key === null}
                  tooltip={param ? t("glues.addKey.tooltip", { min: param.min, max: param.max }) : undefined}
                  onclick={async () => {
                    if (key !== null && (await send(() => addGlueKeyCommand(glue.id, b.parameter, key)))) {
                      newKeys = { ...newKeys, [b.parameter]: "" };
                    }
                  }}>{t("glues.addKey.label")}</TipButton
                >
              </span>
            </div>
          {/each}
          {#if unbound.length > 0}
            <div class="row">
              <span class="grow">
                <Select
                  aria-label={t("glues.axisParameter.aria")}
                  value={axisChoice?.id ?? ""}
                  options={axisOptions}
                  disabled={locked}
                  onchange={(v) => {
                    axisParameter = v;
                    axisKey = "";
                  }}
                />
              </span>
              <input
                type="number"
                step="any"
                aria-label={t("glues.axisKey.aria")}
                value={axisKey === "" ? String(axisChoice?.default ?? 0) : axisKey}
                oninput={(e) => (axisKey = e.currentTarget.value)}
              />
              <TipButton
                small
                disabled={busy || axisChoice === null || axisKeyValue === null}
                tooltip={t("glues.addAxis.tooltip")}
                onclick={async () => {
                  const [parameter, key] = [axisChoice?.id, axisKeyValue];
                  if (parameter !== undefined && key !== null && (await send(() => addGlueKeyCommand(glue.id, parameter, key)))) {
                    axisKey = "";
                  }
                }}>{t("glues.addAxis.label")}</TipButton
              >
            </div>
          {/if}
          <p class="ed-hint help">{t("glues.axes.description")}</p>
        </div>
        <p class="ed-hint help">{t("glues.pairs.description")}</p>
        <div class="row">
          <NumberField
            label={t("glues.repair.label")}
            unit={t("glues.px")}
            min={0}
            step={1}
            size={4}
            bind:value={repairDistance}
            disabled={locked}
          />
          <TipButton
            small
            disabled={busy || distanceValue(String(repairDistance)) === null}
            tooltip={t("glues.repair.tooltip")}
            onclick={() => {
              const d = distanceValue(String(repairDistance));
              if (d !== null) updateGlue(glue.id, (g) => g, d);
            }}>{t("glues.repair.button")}</TipButton
          >
        </div>
        <div class="ed-buttons">
          <Button
            small
            disabled={busy}
            onclick={async () => {
              const id = glue.id;
              if (await send(() => ({ op: "remove_glue", id }))) view.glue = null;
            }}>{t("glues.remove.label")}</Button
          >
        </div>
      </BaseLock>
    </section>
  {/if}

  <section class="add">
    <h3>{t("glues.new.title")}</h3>
    {#if glueable.length < 2}
      <p class="ed-hint">{module ? t("glues.new.emptyModule") : t("glues.new.empty")}</p>
    {:else}
      <Select
        label={t("glues.new.meshA")}
        value={meshA}
        options={meshOptions}
        onchange={(v) => (pickA = v)}
      />
      <Select
        label={t("glues.new.meshB")}
        value={meshB}
        options={meshOptions}
        onchange={(v) => (pickB = v)}
      />
      <NumberField
        label={t("glues.new.distance.label")}
        unit={t("glues.px")}
        min={0}
        step={1}
        size={4}
        aria-describedby="glue-distance-hint"
        bind:value={distance}
      />
      <p class="ed-hint help" id="glue-distance-hint">{t("glues.new.distance.description")}</p>
      <div class="ed-buttons">
        <TipButton
          small
          disabled={busy || !canCreate}
          tooltip={t("glues.new.create.tooltip")}
          onclick={create}>{t("glues.new.create.label")}</TipButton
        >
        {#if fromSelection}<span class="ed-hint">{t("glues.new.fromSelection")}</span>{/if}
        {#if meshA === meshB}<span class="ed-hint">{t("glues.new.sameMesh")}</span>{/if}
      </div>
    {/if}
  </section>
{/if}

{#snippet tag(text: string, tooltip: string)}
  <Tooltip text={tooltip}>
    {#snippet trigger(attrs)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <span {...attrs} class="tag" tabindex="0">{text}</span>
    {/snippet}
  </Tooltip>
{/snippet}

<style>
.editor header h3 {
  margin: 0;
  overflow-wrap: anywhere;
}

.help {
  margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
  overflow-wrap: anywhere;
}

.intro {
  margin-bottom: var(--ugoe-space-3);
}

.base-note {
  margin: var(--ugoe-space-2) 0 var(--ugoe-space-3);
  padding: var(--ugoe-space-2) var(--ugoe-space-3);
  border-left: 3px solid var(--ugoe-color-border);
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-sm);
}

.list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.list li {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
  margin: var(--ugoe-space-1) 0;
}

.row-button {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: flex-start;
  min-width: 0;
  border: 1px solid transparent;
  border-radius: var(--ugoe-radius-md);
  background: none;
  color: inherit;
  font: inherit;
  padding: var(--ugoe-space-1) var(--ugoe-space-2);
  text-align: left;
  overflow-wrap: anywhere;
}

.row-button:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: 1px;
}

.row-button.selected {
  border-color: var(--ugoe-color-accent);
}

.muted {
  color: var(--ugoe-color-text-muted);
}

.editor,
.add {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-2);
  margin: var(--ugoe-space-3) 0;
  padding: var(--ugoe-space-2) var(--ugoe-space-3) var(--ugoe-space-3);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
}

.add h3 {
  margin: 0;
}

.editor header {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
}

.editor header :global(.ugoe-button) {
  margin-left: auto;
}

.meshes {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ugoe-space-2);
}

.value {
  min-width: 0;
  overflow-wrap: anywhere;
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2);
  align-items: center;
}

.grow {
  flex: 1;
  min-width: 0;
}

.row input,
.keys input {
  width: 5em;
  min-width: 0;
}

.axis {
  display: grid;
  gap: var(--ugoe-space-1);
  margin-bottom: var(--ugoe-space-2);
}

.axis-name {
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-xs);
  overflow-wrap: anywhere;
}

.keys {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2);
  align-items: center;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: var(--ugoe-space-1);
  padding-left: var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
}

.tag {
  flex: none;
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-accent);
  border-radius: var(--ugoe-radius-sm);
  color: var(--ugoe-color-accent);
  font-size: var(--ugoe-font-size-xs);
}
</style>
