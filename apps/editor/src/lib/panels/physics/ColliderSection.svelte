<!--
  衝突体の一覧と編集 (docs/editor.md#物理の画面での操作)。円とカプセル。位置は取り付け先のデフォーマの局所座標
  (取り付けなしならキャンバス座標)、半径はキャンバスのピクセル。ビューポートでもドラッグで動かせる。
  取り付け先と形状の種類を変えるときは、今の見た目の位置を保つように写し直す (viewport/tools/physics/physicsGeometry.ts)。
-->
<script lang="ts">
import { Button, Notice, NumberField, Select } from "@ugoe/ui";
import BaseLock from "../../BaseLock.svelte";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { Command, Model, PhysicsCollider, Vec2 } from "../../ipc";
import { elementLabel } from "../../labels";
import { isBaseCollider, unavailableCollidersNotice } from "../../moduleProject";
import type { Selection } from "../../selection.svelte";
import TipButton from "../../ui/TipButton.svelte";
import type { ViewportController } from "../../viewport/controller.svelte";
import { puppetJacobian, puppetMap } from "../../viewport/tools/physics/mapper";
import {
  changeShapeKind,
  type LocalGuess,
  reattach,
} from "../../viewport/tools/physics/physicsGeometry";
import { finiteValue, newCollider, withRadius } from "./physics";
import type { PhysicsView } from "./physicsView.svelte";

interface Props {
  doc: EditorDocument;
  model: Model | null;
  selection: Selection;
  viewport: ViewportController;
  view: PhysicsView;
  send: (command: Command) => Promise<boolean>;
  busy: boolean;
}

let { doc, model, selection, viewport, view, send, busy }: Props = $props();

const settings = $derived(doc.physics);
const collider = $derived(settings?.colliders.find((c) => c.id === view.collider) ?? null);
const deformers = $derived(model?.deformers ?? []);
/** スロットが公開する本体の衝突体が本体に無い (本体に物理が無い) ときの案内 (T0660)。 */
const unavailable = $derived(unavailableCollidersNotice(doc.module));

$effect(() => {
  if (view.collider !== null && !collider) view.collider = null;
});

/** 逆写像の初期値: ワープは格子の中心 (UV)、回転デフォーマは原点。 */
const guess: LocalGuess = (deformer) =>
  deformers.find((d) => d.id === deformer)?.kind.type === "warp" ? [0.5, 0.5] : [0, 0];

async function add(kind: "circle" | "capsule"): Promise<void> {
  // 選択中のデフォーマがあれば、その原点 (ワープは中心) に取り付ける。なければキャンバスの中央。
  const primary = selection.primary?.kind === "deformer" ? selection.primary.id : null;
  const center: Vec2 = primary
    ? guess(primary)
    : [(model?.meta.canvas.width ?? 0) / 2, (model?.meta.canvas.height ?? 0) / 2];
  const created = newCollider(settings, kind, primary, center);
  if (await send({ op: "add_physics_collider", collider: created })) view.collider = created.id;
}

function update(next: PhysicsCollider | null): void {
  if (next) void send({ op: "set_physics_collider", collider: next });
}

function setAttach(attach: string): void {
  const puppet = viewport.puppet;
  if (!collider || !puppet) return;
  update(reattach(collider, attach || null, puppetMap(puppet), puppetJacobian(puppet), guess));
}

function setKind(kind: "circle" | "capsule"): void {
  const puppet = viewport.puppet;
  if (!collider || !puppet) return;
  const shape = changeShapeKind(collider, kind, puppetMap(puppet), puppetJacobian(puppet), guess);
  if (shape) update({ ...collider, shape });
}

function setPoint(which: "center" | "a" | "b", axis: 0 | 1, raw: number): void {
  const value = finiteValue(raw);
  if (!collider || value === null) return;
  const shape = collider.shape;
  const point = (p: Vec2): Vec2 => (axis === 0 ? [value, p[1]] : [p[0], value]);
  if (shape.type === "circle" && which === "center") {
    update({ ...collider, shape: { ...shape, center: point(shape.center) } });
  } else if (shape.type === "capsule" && which !== "center") {
    update({ ...collider, shape: { ...shape, [which]: point(shape[which]) } });
  }
}

function setRadius(raw: number): void {
  const value = finiteValue(raw);
  if (collider && value !== null) update({ ...collider, shape: withRadius(collider.shape, value) });
}

const points = $derived.by((): { key: "center" | "a" | "b"; label: string; p: Vec2 }[] => {
  const shape = collider?.shape;
  if (!shape) return [];
  return shape.type === "circle"
    ? [{ key: "center", label: t("physics.colliders.center"), p: shape.center }]
    : [
        { key: "a", label: t("physics.colliders.endA"), p: shape.a },
        { key: "b", label: t("physics.colliders.endB"), p: shape.b },
      ];
});

/** 取り付け先の選択肢の「キャンバス」(取り付けなし) の値。デフォーマの ID は空にならない。 */
const CANVAS = "\u0000canvas";
const attachOptions = $derived([
  { value: CANVAS, label: t("physics.colliders.attach.canvas") },
  ...deformers.map((d) => ({ value: d.id, label: elementLabel(d.name, d.id) })),
]);
const shapeOptions = $derived([
  { value: "circle" as const, label: t("physics.colliders.shapes.circle") },
  { value: "capsule" as const, label: t("physics.colliders.shapes.capsule") },
]);

function shapeName(kind: "circle" | "capsule"): string {
  return kind === "circle"
    ? t("physics.colliders.shapes.circle")
    : t("physics.colliders.shapes.capsule");
}

const ids = $props.id();
</script>

<section>
  <h3>{t("physics.colliders.title")}</h3>
  {#if unavailable}
    <Notice tone="warning">{unavailable}</Notice>
  {/if}
  {#if settings && settings.colliders.length > 0}
    <ul class="list">
      {#each settings.colliders as c (c.id)}
        <li>
          <button class="item" class:selected={c.id === view.collider}
            onclick={() => (view.collider = c.id === view.collider ? null : c.id)}>
            {elementLabel(c.name, c.id)}
            <span class="muted"
              >· {c.attach
                ? t("physics.colliders.summaryAttached", { shape: shapeName(c.shape.type), deformer: c.attach })
                : shapeName(c.shape.type)}</span
            >
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="ed-hint">{t("physics.colliders.empty")}</p>
  {/if}
  <div class="ed-buttons row">
    <TipButton disabled={busy} onclick={() => add("circle")} tooltip={t("physics.colliders.addCircle.tooltip")}
      >{t("physics.colliders.addCircle.label")}</TipButton
    >
    <TipButton disabled={busy} onclick={() => add("capsule")} tooltip={t("physics.colliders.addCapsule.tooltip")}
      >{t("physics.colliders.addCapsule.label")}</TipButton
    >
  </div>

  {#if collider}
    <BaseLock locked={isBaseCollider(doc.module, collider.id)}>
      <span hidden id="{ids}-x">{t("physics.axis.x")}</span>
      <span hidden id="{ids}-y">{t("physics.axis.y")}</span>
      <div class="ed-field">
        <label for="{ids}-name">{t("physics.colliders.name.label")}</label>
        <input id="{ids}-name" class="text" type="text" value={collider.name}
          onchange={(e) => update({ ...collider, name: e.currentTarget.value })} />
      </div>
      <div class="ed-field">
        <span id="{ids}-attach">{t("physics.colliders.attach.label")}</span>
        <Select labelledby="{ids}-attach" value={collider.attach ?? CANVAS} options={attachOptions}
          onchange={(v) => setAttach(v === CANVAS ? "" : v)} />
      </div>
      <div class="ed-field">
        <span id="{ids}-shape">{t("physics.colliders.shape")}</span>
        <Select labelledby="{ids}-shape" value={collider.shape.type} options={shapeOptions}
          onchange={(v) => setKind(v)} />
      </div>
      {#each points as pt (pt.key)}
        <div class="ed-field">
          <span id="{ids}-{pt.key}">{pt.label}</span>
          <span class="pair">
            <NumberField value={pt.p[0]} step={1} decimals={4} size={5}
              labelledby="{ids}-{pt.key} {ids}-x" onchange={(v) => setPoint(pt.key, 0, v)} />
            <NumberField value={pt.p[1]} step={1} decimals={4} size={5}
              labelledby="{ids}-{pt.key} {ids}-y" onchange={(v) => setPoint(pt.key, 1, v)} />
          </span>
        </div>
      {/each}
      <div class="ed-field">
        <span id="{ids}-radius">{t("physics.colliders.radius")}</span>
        <span class="pair">
          <NumberField value={collider.shape.radius} step={1} min={0} decimals={4} size={5}
            labelledby="{ids}-radius" onchange={(v) => setRadius(v)} />
        </span>
      </div>
      <p class="ed-hint">{t("physics.colliders.hint")}</p>
      <div class="ed-buttons row">
        <Button disabled={busy} onclick={() => send({ op: "remove_physics_collider", id: collider.id })}>
          {t("physics.colliders.delete")}
        </Button>
      </div>
    </BaseLock>
  {/if}
</section>

<style>
.row {
  margin: var(--ugoe-space-2) 0;
}

.pair {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2);
  align-items: center;
  min-width: 0;
}

.text {
  min-width: 0;
  min-height: var(--ugoe-control-height);
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-control-border);
  border-radius: var(--ugoe-radius-md);
  background: var(--ugoe-color-surface);
  color: var(--ugoe-color-text);
  font: inherit;
}

.text:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: 1px;
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
</style>
