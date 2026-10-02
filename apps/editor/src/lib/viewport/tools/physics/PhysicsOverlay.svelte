<!--
  物理の表示 (docs/editor.md#物理の画面での操作): 物理のパネルを開いている間、粒子の鎖と衝突体を重ねる。

  - 衝突体は粒子の鎖の上に描く (重なっていても衝突体をつかめるように)。
  - 粒子: 動的物理の設定のときだけ (物理空間がキャンバス座標なので)。プレビュー中は wasm の今の位置
    (WebPuppet.physics_particles)、止めているときは初期位置 (根元は取り付け先のデフォーマで写す)。
    根元は四角で描く。鎖を押すとそのリグを選ぶ。
  - 衝突体: 取り付け先のデフォーマの今の変換で置いた円・カプセル。押すと選び、ドラッグで動かす
    (カプセルは端の丸で端だけを動かせる)。離したときに set_physics_collider を 1 つ送る (Undo の 1 単位)。
    ドラッグ中はオーバーレイだけでプレビューする。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import { t } from "../../../i18n";
import type { PhysicsCollider } from "../../../ipc";
import { isDynamicRig, isDynamicSettings } from "../../../panels/physics/physics";
import type { PhysicsView } from "../../../panels/physics/physicsView.svelte";
import type { ViewportController } from "../../controller.svelte";
import type { Point } from "../geometry";
import { puppetJacobian, puppetMap } from "./mapper";
import {
  type ColliderHandle,
  capsulePath,
  dragShape,
  restParticles,
  toPoints,
  type WorldShape,
  worldShape,
} from "./physicsGeometry";

interface Props {
  controller: ViewportController;
  doc: EditorDocument;
  view: PhysicsView;
}

let { controller, doc, view }: Props = $props();

/** WebPuppet が新しいモデル・パラメータ・フレームを反映した後に読み直すための印。 */
let tick = $state(0);
$effect(() => {
  void doc.model;
  void doc.physics;
  void controller.transform;
  void controller.status;
  void controller.animationFrame;
  for (const value of Object.values(controller.parameterValues)) void value;
  queueMicrotask(() => (tick += 1));
});

const settings = $derived(doc.physics);

function screen(p: Point): Point {
  const t = controller.transform;
  return t ? [t.offsetX + t.scale * p[0], t.offsetY + t.scale * p[1]] : p;
}

/** リグごとの粒子の位置 (CSS ピクセル)。 */
const chains = $derived.by(() => {
  void tick;
  const puppet = controller.puppet;
  if (!view.active || !settings || !puppet || controller.status !== "ready") return [];
  if (!isDynamicSettings(settings)) return [];
  const map = puppetMap(puppet);
  return settings.rigs.flatMap((rig, i) => {
    // パラメータにだけ出力するリグ (体の追従など) の粒子はキャンバス座標にないので出さない。
    if (!isDynamicRig(rig)) return [];
    let points: Point[] | null = null;
    if (controller.previewingPhysics) {
      const live = toPoints(puppet.physics_particles(i));
      if (live.length === rig.particles.length) points = live;
    }
    points ??= restParticles(rig, map);
    if (!points || points.length === 0) return [];
    const s = points.map(screen);
    return [
      {
        id: rig.id,
        points: s,
        d: s.map((p, k) => `${k === 0 ? "M" : "L"}${p[0]},${p[1]}`).join(""),
        selected: rig.id === view.rig,
      },
    ];
  });
});

/** ドラッグ中の衝突体。 */
let drag = $state<{
  id: string;
  handle: ColliderHandle;
  pointer: number;
  start: Point;
  delta: Point;
} | null>(null);

/** 衝突体の形状 (CSS ピクセル。ドラッグ中はずらした形)。 */
const colliders = $derived.by(() => {
  void tick;
  const puppet = controller.puppet;
  const t = controller.transform;
  if (!view.active || !settings || !puppet || !t || controller.status !== "ready") return [];
  const map = puppetMap(puppet);
  return settings.colliders.flatMap((c) => {
    const w = worldShape(c, map);
    if (!w) return [];
    const moved = drag?.id === c.id ? offset(w, drag.handle, drag.delta) : w;
    return [{ collider: c, shape: toScreen(moved, t.scale), selected: c.id === view.collider }];
  });
});

function offset(w: WorldShape, handle: ColliderHandle, d: Point): WorldShape {
  const add = (p: Point): Point => [p[0] + d[0], p[1] + d[1]];
  if (w.type === "circle") return { ...w, center: add(w.center) };
  return {
    ...w,
    a: handle === "b" ? w.a : add(w.a),
    b: handle === "a" ? w.b : add(w.b),
  };
}

function toScreen(w: WorldShape, scale: number): WorldShape {
  if (w.type === "circle") return { ...w, center: screen(w.center), radius: w.radius * scale };
  return { ...w, a: screen(w.a), b: screen(w.b), radius: w.radius * scale };
}

function canvasPoint(e: PointerEvent): Point | null {
  const svg = (e.currentTarget as Element).closest("svg");
  const r = svg?.getBoundingClientRect();
  if (!r) return null;
  return controller.screenToCanvas(e.clientX - r.left, e.clientY - r.top);
}

function onDown(e: PointerEvent, collider: PhysicsCollider, handle: ColliderHandle): void {
  if (e.button !== 0 || e.altKey) return;
  e.stopPropagation();
  const p = canvasPoint(e);
  if (!p) return;
  view.collider = collider.id;
  (e.currentTarget as Element).setPointerCapture(e.pointerId);
  drag = { id: collider.id, handle, pointer: e.pointerId, start: p, delta: [0, 0] };
}

function onMove(e: PointerEvent): void {
  if (drag?.pointer !== e.pointerId) return;
  const p = canvasPoint(e);
  if (p) drag = { ...drag, delta: [p[0] - drag.start[0], p[1] - drag.start[1]] };
}

async function onUp(e: PointerEvent): Promise<void> {
  const d = drag;
  if (d?.pointer !== e.pointerId) return;
  const collider = settings?.colliders.find((c) => c.id === d.id);
  const puppet = controller.puppet;
  if (!collider || !puppet || Math.hypot(d.delta[0], d.delta[1]) < 1e-6) {
    drag = null;
    return;
  }
  const shape = dragShape(collider, d.handle, d.delta, puppetMap(puppet), puppetJacobian(puppet));
  if (shape) await doc.apply({ op: "set_physics_collider", collider: { ...collider, shape } });
  drag = null;
}
</script>

{#if chains.length > 0 || colliders.length > 0}
  <svg class="layer" role="presentation">
    {#each chains as chain (chain.id)}
      <g class="chain" class:selected={chain.selected} role="presentation"
        onpointerdown={(e) => {
          if (e.button !== 0 || e.altKey) return;
          e.stopPropagation();
          view.rig = chain.id;
        }}>
        <path d={chain.d} />
        {#each chain.points as p, k (k)}
          {#if k === 0}
            <rect x={p[0] - 4} y={p[1] - 4} width="8" height="8" />
          {:else}
            <circle cx={p[0]} cy={p[1]} r="4" />
          {/if}
        {/each}
        <title>{chain.id}</title>
      </g>
    {/each}
    {#each colliders as c (c.collider.id)}
      {@const s = c.shape}
      <g class="collider" class:selected={c.selected}>
        {#if s.type === "circle"}
          <circle class="body" cx={s.center[0]} cy={s.center[1]} r={Math.max(s.radius, 2)}
            role="presentation"
            onpointerdown={(e) => onDown(e, c.collider, "body")}
            onpointermove={onMove} onpointerup={onUp} onpointercancel={onUp}>
            <title>{c.collider.name}</title>
          </circle>
        {:else}
          <path class="body" d={capsulePath(s.a, s.b, Math.max(s.radius, 2))} role="presentation"
            onpointerdown={(e) => onDown(e, c.collider, "body")}
            onpointermove={onMove} onpointerup={onUp} onpointercancel={onUp}>
            <title>{c.collider.name}</title>
          </path>
          <line class="axis" x1={s.a[0]} y1={s.a[1]} x2={s.b[0]} y2={s.b[1]} />
          {#each [["a", s.a], ["b", s.b]] as const as [handle, p] (handle)}
            <circle class="end" cx={p[0]} cy={p[1]} r="5" role="presentation"
              onpointerdown={(e) => onDown(e, c.collider, handle)}
              onpointermove={onMove} onpointerup={onUp} onpointercancel={onUp}>
              <title>{t("viewport.collider.end", { name: c.collider.name, end: handle.toUpperCase() })}</title>
            </circle>
          {/each}
        {/if}
      </g>
    {/each}
  </svg>
{/if}

<style>
.layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

/* 衝突体: 半透明の塗りの上を押して動かす。 */
.collider .body {
  fill: rgb(64 160 255 / 0.15);
  stroke: rgb(64 160 255 / 0.9);
  stroke-width: 1.5;
  pointer-events: visiblePainted;
  cursor: move;
}

.collider.selected .body {
  fill: rgb(64 160 255 / 0.3);
  stroke-width: 2.5;
}

.collider .axis {
  stroke: rgb(64 160 255 / 0.7);
  stroke-dasharray: 4 3;
}

.collider .end {
  fill: white;
  stroke: rgb(64 160 255);
  stroke-width: 1.5;
  pointer-events: visiblePainted;
  cursor: grab;
}

/* 粒子の鎖。 */
.chain {
  pointer-events: visiblePainted;
  cursor: pointer;
}

.chain path {
  fill: none;
  stroke: rgb(255 140 0 / 0.8);
  stroke-width: 2;
}

.chain circle,
.chain rect {
  fill: rgb(255 140 0);
  stroke: white;
  stroke-width: 1;
}

.chain.selected path {
  stroke: rgb(255 80 0);
  stroke-width: 3;
}

.chain.selected circle,
.chain.selected rect {
  fill: rgb(255 80 0);
}
</style>
