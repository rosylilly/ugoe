<!--
  物理の設定のパネル (docs/editor.md#物理の画面での操作)。右の列の「Physics」タブ。
  操作はすべて物理のコマンド (set_physics_* など) を送るだけで、組み立ては physics.ts にある。

  - 「Preview」で物理のプレビューを始める (ViewportController.startPhysicsPreview)。パラメータや頭を動かすと揺れる。
    「Stop」で止めると編集中の姿に戻る。
  - リグの一覧・テンプレート (TemplateSection)・選んだリグの編集 (RigEditor)・衝突体 (ColliderSection)。
  - 静止姿勢で衝突体に当たっている鎖があれば、上に警告を出す (RestPoseWarnings)。行を押すとそのリグと衝突体を選ぶ。
  - リグは上から順に処理される。▲▼ で並べ替える (reorder_physics_rig)。前のリグの出力を読むリグには「relay」と出す
    (docs/spec/physics.md#リグの順序と中継)。モジュールのプロジェクトでは、本体のリグは動かせず、モジュールのリグは
    モジュールのリグどうしでだけ並べ替える (canMoveRig)。
  - パネルを開いている間、ビューポートに粒子と衝突体を重ねる (viewport/tools/physics/PhysicsOverlay.svelte)。
-->
<script lang="ts">
import { Button, Icon, IconButton, NumberField } from "@ugoe/ui";
import BaseLock from "../../BaseLock.svelte";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { PhysicsRig } from "../../ipc";
import { elementLabel } from "../../labels";
import { baseElements, canMoveRig, isBaseElement, isBaseRig } from "../../moduleProject";
import type { Selection } from "../../selection.svelte";
import TipButton from "../../ui/TipButton.svelte";
import type { ViewportController } from "../../viewport/controller.svelte";
import ColliderSection from "./ColliderSection.svelte";
import {
  chainCommand,
  chainOrder,
  DEFAULT_SETTINGS,
  finiteValue,
  isDynamicSettings,
  moveRigCommand,
  newRig,
  relayInputs,
  type SendPhysics,
  type WorldField,
  worldFieldCommand,
} from "./physics";
import type { PhysicsView } from "./physicsView.svelte";
import RestPoseWarnings from "./RestPoseWarnings.svelte";
import RigEditor from "./RigEditor.svelte";
import TemplateSection from "./TemplateSection.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
  view: PhysicsView;
}

let { doc, selection, viewport, view }: Props = $props();

const model = $derived(doc.parsedModel);
const settings = $derived(doc.physics);
const world = $derived(settings ?? DEFAULT_SETTINGS);
const rig = $derived(settings?.rigs.find((r) => r.id === view.rig) ?? null);

// 選んでいたリグが消えたら (Undo など) 選択を外す。
$effect(() => {
  if (view.rig !== null && !rig) view.rig = settings?.rigs[0]?.id ?? null;
});

const base = $derived(baseElements(doc.module));
/** 選択中の回転デフォーマ (親から子の順)。「From selected deformers」で鎖にする。 */
const chain = $derived(
  model
    ? chainOrder(
        model,
        // 本体のデフォーマはモジュールのリグで動かせない。
        selection.items
          .filter((r) => r.kind === "deformer" && !isBaseElement(base, r))
          .map((r) => r.id),
      )
    : [],
);

/** 編集を送っている間は、並べ替え・削除のように今の並びに頼るボタンを無効にする (欄の編集は列に並べて送る)。 */
const busy = $derived(doc.pending > 0);

/** 前の編集を読み直した後の物理の設定からコマンドを組み立てて送る (docs/editor.md#パネルからの編集)。 */
const send: SendPhysics = async (build) => (await doc.send(() => build(doc.physics))) !== null;

async function addRig(): Promise<void> {
  let created = null as PhysicsRig | null;
  const ok = await send((current) => {
    // i18n-ignore: 既定の名前から ID を作るので、言語によらず英語の識別子にする (衝突体の Circle と同じ)
    created = newRig(current, `Rig ${(current?.rigs.length ?? 0) + 1}`);
    return { op: "add_physics_rig", rig: created };
  });
  if (ok && created) view.rig = created.id;
}

async function addChain(): Promise<void> {
  let before = new Set<string>();
  const ok = await send((current) => {
    before = new Set(current?.rigs.map((r) => r.id));
    return chain.length > 0 ? chainCommand(chain) : null;
  });
  if (ok) view.rig = doc.physics?.rigs.find((r) => !before.has(r.id))?.id ?? view.rig;
}

function setWorld(field: WorldField, raw: number): void {
  const value = finiteValue(raw);
  if (value !== null) void send((current) => worldFieldCommand(current, field, value));
}

function moveRig(id: string, delta: -1 | 1): void {
  void send((current) => moveRigCommand(current, id, delta));
}

const rigIds = $derived(settings?.rigs.map((r) => r.id) ?? []);

/** ▲▼ の説明 (モジュールのプロジェクトで本体のリグに関わるため動かせないときは、その理由)。 */
function moveTitle(index: number, delta: -1 | 1): string {
  const plain = delta < 0 ? t("physics.rigs.moveUp.tooltip") : t("physics.rigs.moveDown.tooltip");
  const id = rigIds[index];
  const other = rigIds[index + delta];
  if (other === undefined || id === undefined) return plain;
  if (isBaseRig(doc.module, id)) return t("physics.rigs.baseOrder");
  if (isBaseRig(doc.module, other)) return t("physics.rigs.moduleOrder");
  return plain;
}

const ids = $props.id();

function togglePreview(): void {
  if (viewport.previewingPhysics) viewport.stopPhysicsPreview();
  else viewport.startPhysicsPreview();
}
</script>

<h2>{t("physics.title")}</h2>

<div class="ed-buttons row">
  <TipButton
    disabled={!settings || settings.rigs.length === 0}
    onclick={togglePreview}
    tooltip={t("physics.preview.tooltip")}
  >
    {viewport.previewingPhysics ? t("physics.preview.stop") : t("physics.preview.label")}
  </TipButton>
  {#if viewport.previewingPhysics}<span class="muted" role="status">{t("physics.preview.running")}</span>{/if}
</div>
{#if settings && !isDynamicSettings(settings) && settings.rigs.length > 0}
  <p class="ed-hint">{t("physics.parameterOnly")}</p>
{/if}
<RestPoseWarnings
  {doc}
  selected={{ rig: view.rig, collider: view.collider }}
  onSelect={(rig, collider) => ((view.rig = rig), (view.collider = collider))}
/>

<section>
  <h3>{t("physics.world.title")}</h3>
  <BaseLock locked={doc.module !== null} hint={t("physics.world.baseHint")}>
    <span hidden id="{ids}-x">{t("physics.axis.x")}</span>
    <span hidden id="{ids}-y">{t("physics.axis.y")}</span>
    <div class="ed-field">
      <span id="{ids}-gravity">{t("physics.world.gravity")}</span>
      <span class="pair">
        <NumberField value={world.gravity[0]} step={0.1} decimals={4} size={5}
          aria-labelledby="{ids}-gravity {ids}-x" onchange={(v) => setWorld("gx", v)} />
        <NumberField value={world.gravity[1]} step={0.1} decimals={4} size={5}
          aria-labelledby="{ids}-gravity {ids}-y" onchange={(v) => setWorld("gy", v)} />
      </span>
    </div>
    <div class="ed-field">
      <span id="{ids}-wind">{t("physics.world.wind")}</span>
      <span class="pair">
        <NumberField value={world.wind[0]} step={0.1} decimals={4} size={5}
          aria-labelledby="{ids}-wind {ids}-x" onchange={(v) => setWorld("wx", v)} />
        <NumberField value={world.wind[1]} step={0.1} decimals={4} size={5}
          aria-labelledby="{ids}-wind {ids}-y" onchange={(v) => setWorld("wy", v)} />
      </span>
    </div>
    <div class="ed-field">
      <span aria-hidden="true">{t("physics.world.fps.label")}</span>
      <span class="pair">
        <span hidden id="{ids}-fps">{t("physics.world.fps.aria")}</span>
        <NumberField value={world.fps} step={1} min={1} decimals={4} size={5}
          aria-labelledby="{ids}-fps" onchange={(v) => setWorld("fps", v)} />
      </span>
    </div>
  </BaseLock>
</section>

<section>
  <h3>{t("physics.rigs.title")}</h3>
  {#if settings && settings.rigs.length > 0}
    <p class="ed-hint">{t("physics.rigs.order")}</p>
    <ul class="list">
      {#each settings.rigs as r, i (r.id)}
        {@const relay = relayInputs(settings, i)}
        {@const name = elementLabel(r.name, r.id)}
        <li class="rig">
          <button class="item" class:selected={r.id === view.rig} onclick={() => (view.rig = r.id)}>
            {name}
            <span class="muted"
              >· {t("physics.rigs.particles", { count: r.particles.length })} · {t("physics.rigs.outputs", {
                count: r.outputs.length,
              })}</span
            >
            {#if relay.length > 0}
              <span class="relay" title={t("physics.rigs.relay.tooltip", { parameters: relay.join(", ") })}
                >{t("physics.rigs.relay.label")}</span
              >
            {/if}
          </button>
          <span class="order">
            <IconButton
              small
              disabled={busy || !canMoveRig(doc.module, rigIds, i, -1)}
              onclick={() => moveRig(r.id, -1)}
              label={t("physics.rigs.moveUp.aria", { name })}
              tooltip={moveTitle(i, -1)}><Icon name="arrowUp" /></IconButton
            >
            <IconButton
              small
              disabled={busy || !canMoveRig(doc.module, rigIds, i, 1)}
              onclick={() => moveRig(r.id, 1)}
              label={t("physics.rigs.moveDown.aria", { name })}
              tooltip={moveTitle(i, 1)}><Icon name="arrowDown" /></IconButton
            >
          </span>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="ed-hint">{t("physics.rigs.empty")}</p>
  {/if}
  <div class="ed-buttons row">
    <Button disabled={busy} onclick={addRig}>{t("physics.rigs.add")}</Button>
    <TipButton
      disabled={busy || chain.length === 0}
      onclick={addChain}
      tooltip={t("physics.rigs.fromSelection.tooltip")}
    >
      {chain.length > 0
        ? t("physics.rigs.fromSelection.labelCount", { count: chain.length })
        : t("physics.rigs.fromSelection.label")}
    </TipButton>
  </div>
</section>

{#if model}
  <TemplateSection
    {model}
    module={doc.module}
    {send}
    {busy}
    onApplied={(rigs) => (view.rig = rigs.find((id) => doc.physics?.rigs.some((r) => r.id === id)) ?? view.rig)}
  />
{/if}

{#if rig && model}
  {#key rig.id}
    <BaseLock locked={isBaseRig(doc.module, rig.id)}>
      <RigEditor {rig} {model} {settings} {send} {busy} />
    </BaseLock>
  {/key}
{/if}

<ColliderSection {doc} {model} {selection} {viewport} {view} {send} {busy} />

<style>
.pair {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2);
  align-items: center;
  min-width: 0;
}

.row {
  margin: var(--ugoe-space-2) 0;
}

.list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.item {
  width: 100%;
  text-align: left;
  border: 1px solid transparent;
  border-radius: var(--ugoe-radius-sm);
  background: none;
  color: inherit;
  font: inherit;
  overflow-wrap: anywhere;
}

.item.selected {
  border-color: var(--ugoe-color-accent);
}

.item:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: 1px;
}

.rig {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-1);
}

.rig .item {
  flex: 1;
  min-width: 0;
}

.order {
  display: flex;
  flex: none;
}

.relay {
  margin-left: var(--ugoe-space-2);
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-accent);
  border-radius: var(--ugoe-radius-sm);
  color: var(--ugoe-color-accent);
  font-size: var(--ugoe-font-size-xs);
}
</style>
