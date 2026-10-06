<!--
  衝突体の一覧と編集 (docs/editor.md#物理の画面での操作)。円とカプセル。位置は取り付け先のデフォーマの局所座標
  (取り付けなしならキャンバス座標)、半径はキャンバスのピクセル。ビューポートでもドラッグで動かせる。
  取り付け先と形状の種類を変えるときは、今の見た目の位置を保つように写し直す (viewport/tools/physics/physicsGeometry.ts)。
  set_physics_collider は衝突体を丸ごと置き換えるので、送る直前の衝突体から組み立てる (updateCollider)。
-->
<script lang="ts">
import { Button, Icon, Notice, NumberField, Select, TextField } from "@ugoe/ui";
import BaseLock from "../../BaseLock.svelte";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { Model, PhysicsCollider, Vec2 } from "../../ipc";
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
import { colliderOf, finiteValue, newCollider, type SendPhysics, withRadius } from "./physics";
import type { PhysicsView } from "./physicsView.svelte";

interface Props {
  doc: EditorDocument;
  model: Model | null;
  selection: Selection;
  viewport: ViewportController;
  view: PhysicsView;
  send: SendPhysics;
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
  let created = null as PhysicsCollider | null;
  const ok = await send((current) => {
    created = newCollider(current, kind, primary, center);
    return { op: "add_physics_collider", collider: created };
  });
  if (ok && created) view.collider = created.id;
}

/** 選んでいる衝突体を、送る直前の値から作り直して送る (消えていたり `change` が null なら送らない)。 */
function updateCollider(change: (current: PhysicsCollider) => PhysicsCollider | null): void {
  const id = collider?.id;
  if (id === undefined) return;
  void send((settings) => {
    const current = colliderOf(settings, id);
    const next = current ? change(current) : null;
    return next ? { op: "set_physics_collider", collider: next } : null;
  });
}

function setAttach(attach: string): void {
  const puppet = viewport.puppet;
  if (!puppet) return;
  updateCollider((c) =>
    reattach(c, attach || null, puppetMap(puppet), puppetJacobian(puppet), guess),
  );
}

function setKind(kind: "circle" | "capsule"): void {
  const puppet = viewport.puppet;
  if (!puppet) return;
  updateCollider((c) => {
    const shape = changeShapeKind(c, kind, puppetMap(puppet), puppetJacobian(puppet), guess);
    return shape ? { ...c, shape } : null;
  });
}

function setPoint(which: "center" | "a" | "b", axis: 0 | 1, raw: number): void {
  const value = finiteValue(raw);
  if (value === null) return;
  const point = (p: Vec2): Vec2 => (axis === 0 ? [value, p[1]] : [p[0], value]);
  updateCollider((c) => {
    const shape = c.shape;
    if (shape.type === "circle" && which === "center") {
      return { ...c, shape: { ...shape, center: point(shape.center) } };
    }
    if (shape.type === "capsule" && which !== "center") {
      return { ...c, shape: { ...shape, [which]: point(shape[which]) } };
    }
    return null;
  });
}

function setRadius(raw: number): void {
  const value = finiteValue(raw);
  if (value !== null) updateCollider((c) => ({ ...c, shape: withRadius(c.shape, value) }));
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
          <button class="ed-item" class:selected={c.id === view.collider}
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
      ><Icon name="plus" />{t("physics.colliders.addCircle.label")}</TipButton
    >
    <TipButton disabled={busy} onclick={() => add("capsule")} tooltip={t("physics.colliders.addCapsule.tooltip")}
      ><Icon name="plus" />{t("physics.colliders.addCapsule.label")}</TipButton
    >
  </div>

  {#if collider}
    <BaseLock locked={isBaseCollider(doc.module, collider.id)}>
      <span hidden id="{ids}-x">{t("physics.axis.x")}</span>
      <span hidden id="{ids}-y">{t("physics.axis.y")}</span>
      <div class="ed-field">
        <label for="{ids}-name">{t("physics.colliders.name.label")}</label>
        <TextField id="{ids}-name" value={collider.name}
          onchange={(name) => updateCollider((c) => ({ ...c, name }))} />
      </div>
      <div class="ed-field">
        <span id="{ids}-attach">{t("physics.colliders.attach.label")}</span>
        <Select aria-labelledby="{ids}-attach" value={collider.attach ?? CANVAS} options={attachOptions}
          onchange={(v) => setAttach(v === CANVAS ? "" : v)} />
      </div>
      <div class="ed-field">
        <span id="{ids}-shape">{t("physics.colliders.shape")}</span>
        <Select aria-labelledby="{ids}-shape" value={collider.shape.type} options={shapeOptions}
          onchange={(v) => setKind(v)} />
      </div>
      {#each points as pt (pt.key)}
        <div class="ed-field">
          <span id="{ids}-{pt.key}">{pt.label}</span>
          <span class="pair">
            <NumberField value={pt.p[0]} step={1} decimals={4} size={5}
              aria-labelledby="{ids}-{pt.key} {ids}-x" onchange={(v) => setPoint(pt.key, 0, v)} />
            <NumberField value={pt.p[1]} step={1} decimals={4} size={5}
              aria-labelledby="{ids}-{pt.key} {ids}-y" onchange={(v) => setPoint(pt.key, 1, v)} />
          </span>
        </div>
      {/each}
      <div class="ed-field">
        <span id="{ids}-radius">{t("physics.colliders.radius")}</span>
        <span class="pair">
          <NumberField value={collider.shape.radius} step={1} min={0} decimals={4} size={5}
            aria-labelledby="{ids}-radius" onchange={(v) => setRadius(v)} />
        </span>
      </div>
      <p class="ed-hint">{t("physics.colliders.hint")}</p>
      <div class="ed-buttons row">
        <Button
          disabled={busy}
          onclick={() => {
            const id = collider.id;
            void send((current) => (colliderOf(current, id) ? { op: "remove_physics_collider", id } : null));
          }}
        >
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

.list {
  list-style: none;
  margin: 0;
  padding: 0;
}
</style>
