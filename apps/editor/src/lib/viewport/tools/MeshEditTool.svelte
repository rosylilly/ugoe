<!--
  メッシュ編集モード (docs/editor.md#メッシュの編集): 選択中のメッシュの頂点と辺を表示し、
  頂点の選択・移動・追加・削除と三角形の作り直しを行う。

  - クリック: 頂点を選ぶ (Shift で追加・解除)。何もない所からのドラッグは矩形選択 (Shift で追加)。
  - 頂点のドラッグ: 選択中の頂点を動かす。ドラッグ中はここでプレビューだけを描き、
    離したときに move_mesh_vertices を 1 つ送る。
  - Ctrl (Cmd) + クリック: その位置に頂点を足す。Delete / Backspace: 選択中の頂点を消す。
  - Esc: モードを抜ける (ToolOverlay が処理する)。
  - `]` `[`: 次・前の頂点を 1 つ選ぶ。Shift + `]` `[`: 1 つ目を残して 2 つ目の頂点を次・前へ。
    `I`・「Add midpoint」: 選んだ 2 つの頂点の中点に頂点を足す (辺を分ける。Ctrl + クリックの代わり)。
    矢印キー: 選択中の頂点を動かす (キーは ToolOverlay が受けて呼ぶ)。
  頂点位置は wasm で評価した今のパラメータでの位置。コマンドは既定値の姿を基準にするので、
  モードに入るときにパラメータを既定値に戻す (ToolOverlay)。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { Selection } from "../../selection.svelte";
import {
  type Box,
  meshEdges,
  nearestVertex,
  type Point,
  updateVertexSelection,
  verticesInBox,
} from "./geometry";
import { extendedPoints, steppedPoint } from "./keys";
import type { ViewportScene } from "./scene.svelte";

interface Props {
  scene: ViewportScene;
  doc: EditorDocument;
  selection: Selection;
  meshId: string;
  /** `]` `[` で選んだ頂点の表示位置 (CSS ピクセル) で呼ぶ (ビューの外なら見える所まで動かす。ToolOverlay)。 */
  reveal?: (x: number, y: number) => void;
}

let { scene, doc, selection, meshId, reveal }: Props = $props();

/** 頂点をつかめる距離 (CSS ピクセル)。 */
const PICK_RADIUS = 8;
/** これより動かなければクリックとみなす (CSS ピクセル)。 */
const CLICK_SLOP = 3;

type Drag =
  | { kind: "move"; id: number; x0: number; y0: number; dx: number; dy: number }
  | { kind: "box"; id: number; box: Box; toggle: boolean };

let svg: SVGSVGElement;
let drag = $state<Drag | null>(null);
let busy = false;

const mesh = $derived(scene.mesh(meshId));
const positions = $derived(scene.screenPositions(meshId));
const edges = $derived(mesh ? meshEdges(mesh.indices) : []);
const selected = $derived(
  (selection.vertices[meshId] ?? []).filter((i) => mesh !== null && i < mesh.vertices),
);
const selectedSet = $derived(new Set(selected));
/** 1 つだけ選んだ頂点 (`]` `[` で選ぶ点。囲みを描く)。 */
const current = $derived(selected.length === 1 ? (selected[0] ?? null) : null);

/** ドラッグ中のプレビューを含めた表示位置。 */
const shown = $derived.by(() => {
  if (!positions || drag?.kind !== "move") return positions;
  const out = positions.slice();
  for (const i of selected) {
    out[2 * i] = (out[2 * i] ?? 0) + drag.dx;
    out[2 * i + 1] = (out[2 * i + 1] ?? 0) + drag.dy;
  }
  return out;
});

function setVertices(indices: number[]): void {
  selection.vertices = { ...selection.vertices, [meshId]: indices };
}

function local(e: PointerEvent): [number, number] {
  const r = svg.getBoundingClientRect();
  return [e.clientX - r.left, e.clientY - r.top];
}

async function run(action: () => Promise<void>): Promise<void> {
  if (busy) return;
  busy = true;
  try {
    await action();
  } finally {
    busy = false;
  }
}

function onPointerDown(e: PointerEvent): void {
  // 中ボタンと Alt + ドラッグはビューポートのパン (Viewport.svelte) に任せる。
  // 前の編集を送っている間は受けない (離したときの run が捨てられ、ずらした表示が残るため)。
  if (e.button !== 0 || e.altKey || busy || !positions || !mesh) return;
  const [x, y] = local(e);
  if (e.ctrlKey || e.metaKey) {
    addVertex(x, y);
    return;
  }
  svg.setPointerCapture(e.pointerId);
  const hit = nearestVertex(positions, x, y, PICK_RADIUS);
  if (hit === null) {
    drag = {
      kind: "box",
      id: e.pointerId,
      box: { x0: x, y0: y, x1: x, y1: y },
      toggle: e.shiftKey,
    };
    return;
  }
  if (e.shiftKey) {
    setVertices(updateVertexSelection(selected, [hit], true));
    if (!selectedSet.has(hit)) return;
  } else if (!selectedSet.has(hit)) {
    setVertices([hit]);
  }
  drag = { kind: "move", id: e.pointerId, x0: x, y0: y, dx: 0, dy: 0 };
}

function onPointerMove(e: PointerEvent): void {
  // 送っている間は、送ったドラッグを動かさない・送り直さない (離した後の移動と、別の押下の離す操作が届く)。
  if (busy || drag?.id !== e.pointerId) return;
  const [x, y] = local(e);
  if (drag.kind === "move") {
    drag.dx = x - drag.x0;
    drag.dy = y - drag.y0;
  } else {
    drag.box.x1 = x;
    drag.box.y1 = y;
  }
}

function onPointerUp(e: PointerEvent): void {
  const d = drag;
  if (busy || d?.id !== e.pointerId) return;
  if (d.kind === "box") {
    drag = null;
    const { box } = d;
    const clicked = Math.hypot(box.x1 - box.x0, box.y1 - box.y0) < CLICK_SLOP;
    if (clicked) {
      if (!d.toggle) setVertices([]);
    } else if (positions) {
      setVertices(updateVertexSelection(selected, verticesInBox(positions, box), d.toggle));
    }
    return;
  }
  if (Math.hypot(d.dx, d.dy) < CLICK_SLOP || selected.length === 0) {
    drag = null;
    return;
  }
  const delta: [number, number] = [scene.canvasLength(d.dx), scene.canvasLength(d.dy)];
  const moves = selected.map((index) => ({ index, delta }));
  // 読み直すまでプレビューを残す (元の位置に一瞬戻らないように)。
  run(async () => {
    await doc.apply({ op: "move_mesh_vertices", id: meshId, moves });
    drag = null;
  });
}

function addVertex(x: number, y: number): void {
  const p = scene.toCanvas(x, y);
  addVertexAt(p);
}

function addVertexAt(p: Point | null): void {
  if (!p || !mesh) return;
  const added = mesh.vertices;
  run(async () => {
    await doc.apply({ op: "add_mesh_vertex", id: meshId, position: p });
    if (scene.mesh(meshId)?.vertices === added + 1) setVertices([added]);
  });
}

/** 選択中の頂点を消す (Delete キーとツールバーから呼ぶ)。 */
export function removeSelected(): void {
  if (selected.length === 0) return;
  const indices = selected;
  run(async () => {
    if (await doc.apply({ op: "remove_mesh_vertices", id: meshId, indices })) setVertices([]);
  });
}

/** 選択中の頂点を画面の CSS ピクセルで `(dx, dy)` だけ動かす (矢印キー。ドラッグの代わり)。1 回が Undo の 1 単位。 */
export function nudge(dx: number, dy: number): void {
  if (busy || selected.length === 0) return;
  const delta: [number, number] = [scene.canvasLength(dx), scene.canvasLength(dy)];
  const moves = selected.map((index) => ({ index, delta }));
  run(async () => {
    await doc.apply({ op: "move_mesh_vertices", id: meshId, moves });
  });
}

export function retriangulate(): void {
  run(async () => {
    await doc.apply({ op: "retriangulate_mesh", id: meshId });
  });
}

/** 次 (`dir` = 1)・前 (-1) の頂点を 1 つ選ぶ (`]` `[`。クリックの代わり)。選んだら true。 */
export function step(dir: 1 | -1): boolean {
  const next = steppedPoint(selected, mesh?.vertices ?? 0, dir);
  if (next === null) return false;
  if (drag === null) {
    setVertices([next]);
    const [x, y] = point(next);
    if (positions) reveal?.(x, y);
  }
  return true;
}

/**
 * 1 つ目を残して 2 つ目の頂点を次 (`dir` = 1)・前 (-1) へ (Shift + `]` `[`。中点に足す 2 つの頂点を選ぶ)。選んだら true。
 */
export function extend(dir: 1 | -1): boolean {
  const next = extendedPoints(selected, mesh?.vertices ?? 0, dir);
  if (next === null) return false;
  if (drag === null) {
    setVertices(next);
    const [x, y] = point(next.at(-1) ?? 0);
    if (positions) reveal?.(x, y);
  }
  return true;
}

/**
 * 選んだ 2 つの頂点の中点に頂点を 1 つ足す (`I`・「Add midpoint」。2 つが辺で結ばれていれば、その辺を分ける)。
 * Ctrl + クリックで頂点を足す操作の、マウスを使わない代わり。add_mesh_vertex を 1 つ送り、足した頂点を選ぶ。
 * 2 つを選んでいなければ何もしない (false)。
 */
export function addMidpoint(): boolean {
  const [a, b] = selected;
  if (selected.length !== 2 || a === undefined || b === undefined) return false;
  if (busy || drag !== null || !positions) return true;
  const [ax, ay] = point(a);
  const [bx, by] = point(b);
  addVertexAt(scene.toCanvas((ax + bx) / 2, (ay + by) / 2));
  return true;
}

export function selectAll(): void {
  if (mesh) setVertices(Array.from({ length: mesh.vertices }, (_, i) => i));
}

function point(i: number): [number, number] {
  return [shown?.[2 * i] ?? 0, shown?.[2 * i + 1] ?? 0];
}
</script>

<svg
  bind:this={svg}
  class="tool"
  role="application"
  aria-label={t("viewport.meshEdit.aria")}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={() => (drag = null)}
>
  {#if shown}
    <g class="edges">
      {#each edges as [a, b] (`${a},${b}`)}
        {@const [ax, ay] = point(a)}
        {@const [bx, by] = point(b)}
        <line x1={ax} y1={ay} x2={bx} y2={by} />
      {/each}
    </g>
    <g class="vertices">
      {#each { length: shown.length / 2 } as _, i (i)}
        {@const [x, y] = point(i)}
        <rect
          class:selected={selectedSet.has(i)}
          x={x - 3.5}
          y={y - 3.5}
          width="7"
          height="7"
        />
      {/each}
    </g>
    {#if current !== null}
      {@const [x, y] = point(current)}
      <rect class="ring" x={x - 7} y={y - 7} width="14" height="14" />
    {/if}
  {/if}
  {#if drag?.kind === "box"}
    {@const b = drag.box}
    <rect
      class="box"
      x={Math.min(b.x0, b.x1)}
      y={Math.min(b.y0, b.y1)}
      width={Math.abs(b.x1 - b.x0)}
      height={Math.abs(b.y1 - b.y0)}
    />
  {/if}
</svg>

<style>
.tool {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: auto;
  user-select: none;
  cursor: crosshair;
}

.edges line {
  stroke: var(--viewport-edit);
  stroke-width: 1;
}

.vertices rect {
  fill: var(--viewport-point);
  stroke: var(--viewport-edit);
  stroke-width: 1.5;
}

.vertices rect.selected {
  fill: var(--viewport-picked);
  stroke: var(--viewport-picked-edge);
}

/* 1 つだけ選んだ頂点 (`]` `[` で選ぶ点) の囲み。密なメッシュの中でも見つけやすくする */
.ring {
  fill: none;
  stroke: var(--viewport-picked-edge);
  stroke-width: 1.5;
}

.box {
  fill: color-mix(in srgb, var(--viewport-edit) 12%, transparent);
  stroke: var(--viewport-edit);
  stroke-dasharray: 4 3;
}
</style>
