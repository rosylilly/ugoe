<!--
  衝突体の一覧と編集 (docs/editor.md#物理の画面での操作)。円とカプセル。位置は取り付け先のデフォーマの局所座標
  (取り付けなしならキャンバス座標)、半径はキャンバスのピクセル。ビューポートでもドラッグで動かせる。
  取り付け先と形状の種類を変えるときは、今の見た目の位置を保つように写し直す (viewport/tools/physics/physicsGeometry.ts)。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import type { Command, Model, PhysicsCollider, Vec2 } from "../../ipc";
import { elementLabel } from "../../labels";
import type { Selection } from "../../selection.svelte";
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

function number(e: Event): number {
  return (e.currentTarget as HTMLInputElement).valueAsNumber;
}

const points = $derived.by((): { key: "center" | "a" | "b"; label: string; p: Vec2 }[] => {
  const shape = collider?.shape;
  if (!shape) return [];
  return shape.type === "circle"
    ? [{ key: "center", label: "Center", p: shape.center }]
    : [
        { key: "a", label: "End A", p: shape.a },
        { key: "b", label: "End B", p: shape.b },
      ];
});
</script>

<section>
  <h3>Colliders</h3>
  {#if settings && settings.colliders.length > 0}
    <ul class="list">
      {#each settings.colliders as c (c.id)}
        <li>
          <button class="item" class:selected={c.id === view.collider}
            onclick={() => (view.collider = c.id === view.collider ? null : c.id)}>
            {elementLabel(c.name, c.id)}
            <span class="muted">· {c.shape.type}{c.attach ? ` on ${c.attach}` : ""}</span>
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted hint">No colliders.</p>
  {/if}
  <div class="row">
    <button disabled={busy} onclick={() => add("circle")}
      title="Add a circle collider (attached to the selected deformer, if any)">+ Circle</button>
    <button disabled={busy} onclick={() => add("capsule")}
      title="Add a capsule collider (attached to the selected deformer, if any)">+ Capsule</button>
  </div>

  {#if collider}
    <div class="grid editor">
      <span>Name</span>
      <input type="text" value={collider.name} aria-label="Collider name"
        onchange={(e) => update({ ...collider, name: e.currentTarget.value })} />
      <span>Attach to</span>
      <select value={collider.attach ?? ""} aria-label="Collider attachment"
        onchange={(e) => setAttach(e.currentTarget.value)}>
        <option value="">Canvas</option>
        {#each deformers as d (d.id)}
          <option value={d.id}>{elementLabel(d.name, d.id)}</option>
        {/each}
      </select>
      <span>Shape</span>
      <select value={collider.shape.type} aria-label="Collider shape"
        onchange={(e) => setKind(e.currentTarget.value as "circle" | "capsule")}>
        <option value="circle">Circle</option>
        <option value="capsule">Capsule</option>
      </select>
      {#each points as pt (pt.key)}
        <span>{pt.label}</span>
        <span class="row">
          <input type="number" step="1" value={pt.p[0]} aria-label="{pt.label} x"
            onchange={(e) => setPoint(pt.key, 0, number(e))} />
          <input type="number" step="1" value={pt.p[1]} aria-label="{pt.label} y"
            onchange={(e) => setPoint(pt.key, 1, number(e))} />
        </span>
      {/each}
      <span>Radius</span>
      <input type="number" step="1" min="0" value={collider.shape.radius}
        aria-label="Collider radius" onchange={(e) => setRadius(number(e))} />
    </div>
    <p class="muted hint">
      Positions are in the attached deformer's local space. Drag the collider in the viewport to move it.
    </p>
    <div class="row">
      <button disabled={busy} onclick={() => send({ op: "remove_physics_collider", id: collider.id })}>
        Delete collider
      </button>
    </div>
  {/if}
</section>

<style>
h3 {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.grid {
  display: grid;
  grid-template-columns: 6em 1fr;
  gap: 4px;
  align-items: center;
}

.editor {
  margin-top: 6px;
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
  margin: 4px 0;
}

.grid .row {
  margin: 0;
}

input[type="number"] {
  width: 4.5em;
  min-width: 0;
}

input[type="text"],
select {
  min-width: 0;
  max-width: 100%;
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
  background: none;
  overflow-wrap: anywhere;
}

.item.selected {
  border-color: var(--accent);
}

.hint {
  font-size: 11px;
}
</style>
