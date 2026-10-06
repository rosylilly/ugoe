<!--
  グルーのパネル (docs/editor.md#グルーの編集)。右の列の「Glue」タブ。組み立ては glues.ts にある。

  - 一覧: グルーごとに両端のメッシュと組の数。押すと選び、下に編集の欄を出す。「Select meshes」でツリーで両端を選ぶ。
  - 作る: 2 つのメッシュと距離を選んで「+ Glue」(create_glue。頂点の組は editor-core が基準の姿の近さで作る)。
    ツリーで 2 つのメッシュを選んでいれば、それを既定にする。
  - 編集: 重み (どちらへ寄せるか。すべての組に同じ値)・強さ (キーフォームごと)・両端の入れ替え・組の作り直し・削除。
    どれも set_glue でグルーを丸ごと置き換えるので、送る直前のグルーから組み立てる (updateGlue)。
  - 強さの軸 (T0661): パラメータとキーを選んで「+ Axis」、軸ごとにキーを足す・外す (add_keyform_key・remove_keyform_key)。
  - 頂点の組はビューポートに重ね、ビューポートの「Add pairs」「Remove pairs」で組を足す・外す (viewport/tools/glue/GlueOverlay.svelte)。
  - 組の一覧 (T1648): 組ごとに「外す」、2 つのメッシュの頂点の番号 (1 から) を入れて「Pair」で 1 組足す。ビューポートで
    頂点を押す操作の、マウスを使わない代わり。どちらも set_glue を 1 つ送る (withPair・withoutPair)。
    行 (と「×」) にポインタかフォーカスがある間は、ビューポートでその組を強調する (T1651。GlueView.highlightedPair。表示だけ)。
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
  hasPair,
  isBaseGlue,
  isDrawCloneGlue,
  keyformLabels,
  keyStep,
  keyValue,
  pairVertex,
  removeGlueKeyCommand,
  selectedPair,
  setGlueCommand,
  swapped,
  unboundParameters,
  uniformWeight,
  withIntensity,
  withoutPair,
  withPair,
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

// 強調する組 (T1651) は、グルーを替えたら・組が無くなったら (「×」で外した行はフォーカスを外す事象を出さずに消える) 消す。
// パネルを閉じたときも消す。
$effect(() => {
  void view.glue;
  return () => view.clearHighlight();
});
$effect(() => {
  const pair = view.highlightedPair;
  if (pair !== null && !(glue && hasPair(glue, pair.a, pair.b))) view.clearHighlight();
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
/** 新しい強さの軸の最初のキー (null は空。パラメータの既定値を使う)。 */
let axisKey = $state<number | null>(null);
/** 軸ごとの、足すキーの入力 (打っている途中の読めた値)。 */
let newKeys = $state<Record<string, number | null>>({});

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

/** 組を足す欄の頂点の番号 (1 から。空は null)。 */
let pairA = $state<number | null>(null);
let pairB = $state<number | null>(null);

function vertexCount(id: string): number {
  return meshes.find((m) => m.id === id)?.uvs.length ?? 0;
}

/** 欄の番号から足す組 (メッシュの頂点の番号)。どちらかが空・範囲の外なら null。 */
const newPair = $derived.by(() => {
  if (!glue) return null;
  const a = pairVertex(pairA, vertexCount(glue.mesh_a));
  const b = pairVertex(pairB, vertexCount(glue.mesh_b));
  return a === null || b === null ? null : { a, b, exists: hasPair(glue, a, b) };
});

/** 送る直前のグルー `id` に `change` を当てて set_glue を送る (null なら送らない)。 */
function updatePairs(id: string, change: (current: Glue) => Glue | null): Promise<boolean> {
  return send(() => {
    const current = model?.glues.find((g) => g.id === id);
    const next = current ? change(current) : null;
    return next ? setGlueCommand(next) : null;
  });
}

async function addPair(id: string): Promise<void> {
  const pair = newPair;
  if (pair === null || pair.exists) return;
  if (await updatePairs(id, (g) => withPair(g, pair.a, pair.b))) {
    pairA = null;
    pairB = null;
  }
}

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
const ids = $props.id();
const axisOptions = $derived(unbound.map((p) => ({ value: p.id, label: p.name || p.id })));
const axisKeyValue = $derived(
  axisChoice ? keyValue(axisKey ?? axisChoice.default, axisChoice) : null,
);
</script>

<!-- 右の列のタブと同じ名前なので見た目では隠す (読み上げと見出しの移動に残す。App.svelte) -->
<h2 class="ugoe-visually-hidden">{t("glues.title")}</h2>
<p class="ed-hint intro">{t("glues.intro")}</p>
{#if module}
  <p class="ed-note" role="note">{t("glues.moduleNote")}</p>
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
            class="ed-item"
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
    <section class="ed-card editor" aria-label={t("glues.editor.aria", { id: glue.id })}>
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
        <p class="ed-help">
          {t("glues.weight.description", { a: meshLabel(glue.mesh_a), b: meshLabel(glue.mesh_b) })}
          {#if !uniformWeight(glue)}{t("glues.weight.mixed")}{/if}
        </p>
        {#each keyformLabels(glue) as label, i (i)}
          <NumberField
            {label}
            step={0.1}
            decimals={4}
            aria-describedby="{ids}-intensity-hint"
            value={glue.keyforms.keyforms[i]?.intensity ?? 0}
            disabled={locked}
            onchange={(v) => updateGlue(glue.id, (g) => withIntensity(g, i, v))}
          />
        {/each}
        <p class="ed-help" id="{ids}-intensity-hint">{t("glues.intensity.description")}</p>
        {#if glue.keyforms.bindings.length > 0}
          <p class="ed-help">{t("glues.intensity.follows")}</p>
        {/if}
        <div class="axes" role="group" aria-label={t("glues.axes.aria")}>
          {#each glue.keyforms.bindings as b (b.parameter)}
            {@const param = doc.parameters.find((p) => p.id === b.parameter)}
            {@const typed = newKeys[b.parameter] ?? null}
            {@const key = param ? keyValue(typed, param, b.keys) : null}
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
                <!-- 空 (キーを入れていない) を持てる欄。打つたびに読めた値で「Key」を押せるかを決める -->
                <NumberField
                  optional
                  min={param?.min}
                  max={param?.max}
                  step={param ? keyStep(param) : 0.1}
                  decimals={4}
                  value={typed}
                  placeholder={t("glues.newKey.placeholder")}
                  aria-label={t("glues.newKey.aria", { parameter: parameterName(b.parameter) })}
                  oninput={(v) => (newKeys = { ...newKeys, [b.parameter]: v })}
                  onchange={(v) => (newKeys = { ...newKeys, [b.parameter]: v })}
                />
                <TipButton
                  small
                  disabled={busy || key === null}
                  tooltip={param ? t("glues.addKey.tooltip", { min: param.min, max: param.max }) : undefined}
                  onclick={async () => {
                    if (key !== null && (await send(() => addGlueKeyCommand(glue.id, b.parameter, key)))) {
                      newKeys = { ...newKeys, [b.parameter]: null };
                    }
                  }}><Icon name="plus" />{t("glues.addKey.label")}</TipButton
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
                    axisKey = null;
                  }}
                />
              </span>
              <!-- 空はパラメータの既定値 (空の欄に薄く出す) -->
              <NumberField
                optional
                min={axisChoice?.min}
                max={axisChoice?.max}
                step={axisChoice ? keyStep(axisChoice) : 0.1}
                decimals={4}
                value={axisKey}
                emptyValue={axisChoice?.default}
                aria-label={t("glues.axisKey.aria")}
                oninput={(v) => (axisKey = v)}
                onchange={(v) => (axisKey = v)}
              />
              <TipButton
                small
                disabled={busy || axisChoice === null || axisKeyValue === null}
                tooltip={t("glues.addAxis.tooltip")}
                onclick={async () => {
                  const [parameter, key] = [axisChoice?.id, axisKeyValue];
                  if (parameter !== undefined && key !== null && (await send(() => addGlueKeyCommand(glue.id, parameter, key)))) {
                    axisKey = null;
                  }
                }}><Icon name="plus" />{t("glues.addAxis.label")}</TipButton
              >
            </div>
          {/if}
          <p class="ed-help">{t("glues.axes.description")}</p>
        </div>
        <p class="ed-help">{t("glues.pairs.description")}</p>
        {@render pairList(glue, locked)}
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

  <section class="ed-card add">
    <h3>{t("glues.new.title")}</h3>
    {#if glueable.length < 2}
      <p class="ed-hint">{module ? t("glues.new.emptyModule") : t("glues.new.empty")}</p>
    {:else}
      <div class="ed-field">
        <span id="{ids}-mesh-a">{t("glues.new.meshA")}</span>
        <Select
          aria-labelledby="{ids}-mesh-a"
          value={meshA}
          options={meshOptions}
          onchange={(v) => (pickA = v)}
        />
      </div>
      <div class="ed-field">
        <span id="{ids}-mesh-b">{t("glues.new.meshB")}</span>
        <Select
          aria-labelledby="{ids}-mesh-b"
          value={meshB}
          options={meshOptions}
          onchange={(v) => (pickB = v)}
        />
      </div>
      <div class="ed-field">
        <span id="{ids}-distance">{t("glues.new.distance.label")}</span>
        <NumberField
          aria-labelledby="{ids}-distance"
          unit={t("glues.px")}
          min={0}
          step={1}
          size={4}
          aria-describedby="{ids}-distance-hint"
          bind:value={distance}
        />
      </div>
      <p class="ed-help" id="{ids}-distance-hint">{t("glues.new.distance.description")}</p>
      <div class="ed-buttons">
        <TipButton
          small
          disabled={busy || !canCreate}
          tooltip={t("glues.new.create.tooltip")}
          onclick={create}><Icon name="plus" />{t("glues.new.create.label")}</TipButton
        >
        {#if fromSelection}<span class="ed-hint">{t("glues.new.fromSelection")}</span>{/if}
        {#if meshA === meshB}<span class="ed-hint">{t("glues.new.sameMesh")}</span>{/if}
      </div>
    {/if}
  </section>
{/if}

{#snippet pairList(g: Glue, locked: boolean)}
  <div class="pairs" role="group" aria-labelledby="{ids}-pairs">
    <h4 id="{ids}-pairs">{t("glues.pairs.title")}</h4>
    <p class="ed-help">
      {t("glues.pairs.legend", { a: meshLabel(g.mesh_a), b: meshLabel(g.mesh_b) })}
    </p>
    {#if g.pairs.length === 0}
      <p class="ed-hint">{t("glues.pairs.empty")}</p>
    {:else}
      <ul class="pair-list" aria-label={t("glues.pairs.listAria", { id: g.id })}>
        {#each g.pairs as pair, i (i)}
          <li
            class:highlighted={view.highlightedPair?.a === pair.a &&
              view.highlightedPair.b === pair.b}
            onpointerenter={() => (view.hoveredPair = { a: pair.a, b: pair.b })}
            onpointerleave={() => (view.hoveredPair = null)}
            onfocusin={() => (view.focusedPair = { a: pair.a, b: pair.b })}
            onfocusout={() => (view.focusedPair = null)}
          >
            <span>{t("glues.pairs.row", { a: pair.a + 1, b: pair.b + 1 })}</span>
            <IconButton
              small
              variant="ghost"
              disabled={busy || locked}
              label={t("glues.pairs.removeAria", {
                a: pair.a + 1,
                b: pair.b + 1,
                meshA: meshLabel(g.mesh_a),
                meshB: meshLabel(g.mesh_b),
              })}
              onclick={() => updatePairs(g.id, (current) => withoutPair(current, pair.a, pair.b))}
              ><Icon name="close" /></IconButton
            >
          </li>
        {/each}
      </ul>
    {/if}
    <div class="row">
      <NumberField
        optional
        label={t("glues.pairs.vertexOf", { mesh: "A" })}
        min={1}
        max={Math.max(1, vertexCount(g.mesh_a))}
        step={1}
        size={4}
        value={pairA}
        disabled={locked}
        oninput={(v) => (pairA = v)}
        onchange={(v) => (pairA = v)}
      />
      <NumberField
        optional
        label={t("glues.pairs.vertexOf", { mesh: "B" })}
        min={1}
        max={Math.max(1, vertexCount(g.mesh_b))}
        step={1}
        size={4}
        value={pairB}
        disabled={locked}
        oninput={(v) => (pairB = v)}
        onchange={(v) => (pairB = v)}
      />
      <TipButton
        small
        disabled={busy || locked || newPair === null || newPair.exists}
        tooltip={t("glues.pairs.add.tooltip")}
        onclick={() => addPair(g.id)}><Icon name="plus" />{t("glues.pairs.add.label")}</TipButton
      >
    </div>
    {#if newPair?.exists}<p class="ed-hint" role="status">{t("glues.pairs.exists")}</p>{/if}
  </div>
{/snippet}

{#snippet tag(text: string, tooltip: string)}
  <Tooltip text={tooltip}>
    {#snippet trigger(attrs)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <span {...attrs} class="ed-tag accent" tabindex="0">{text}</span>
    {/snippet}
  </Tooltip>
{/snippet}

<style>
.editor header h3 {
  margin: 0;
  overflow-wrap: anywhere;
}

.intro {
  margin-bottom: var(--ugoe-space-3);
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

.editor,
.add {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-2);
}

/* 縦の並びの間は gap で取る */
.editor :global(.ed-field),
.add :global(.ed-field),
.add .ed-help {
  margin: 0;
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

/* 選択欄の幅 (中身の幅か、Select の最小の幅) より狭くは縮めず、収まらなければ数値の欄を次の行へ送る
   (min-width: 0 だと選択欄が枠の外へはみ出して数値の欄の下に隠れる。T1650)。列より長い名前は列の幅で止める */
.grow {
  flex: 1;
  max-width: 100%;
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

.pairs {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-2);
}

.pairs h4 {
  margin: 0;
  font-size: var(--ugoe-font-size-sm);
}

.pairs .ed-help,
.pairs .ed-hint {
  margin: 0;
}

/* 組の多いグルーでもパネルを押し広げないよう、一覧だけをスクロールする (中のボタンでキーボードでも動く) */
.pair-list {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-1) var(--ugoe-space-2);
  max-height: 12rem;
  margin: 0;
  padding: 0 var(--ugoe-scrollbar-gap) 0 0;
  overflow-y: auto;
  list-style: none;
}

.pair-list li {
  display: inline-flex;
  align-items: center;
  gap: var(--ugoe-space-1);
  padding-left: var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
  font-variant-numeric: tabular-nums;
}

/* ビューポートで強調している組の行 (T1651)。枠の色と太さで示す (outline は強制色でも残る) */
.pair-list li.highlighted {
  border-color: var(--ugoe-color-accent);
  outline: 1px solid var(--ugoe-color-accent);
  outline-offset: -2px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: var(--ugoe-space-1);
  padding-left: var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
}
</style>
