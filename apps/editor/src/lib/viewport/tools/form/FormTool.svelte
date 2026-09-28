<!--
  フォーム編集ツール (docs/editor.md#キーフォームの編集)。主な選択の要素について、今のパラメータ値の
  セルのキーフォームをビューポートで編集する: メッシュの頂点・ワープの制御点・回転デフォーマの原点をドラッグし、
  離したときに set_keyform_cell を 1 つ送る (ドラッグ中はオーバーレイだけでプレビューする)。
  Shift を押しながらドラッグすると、すべての点を同じだけ動かす。

  パラメータ値がどれかの軸でキーの上にないときは編集しない (どのセルに書くか決まらないため)。
  代わりに「最も近いキーへ移動」を出す。

  点は親の座標系で持つので、表示と書き込みの前にバックエンドで変換する
  (ipc.localToCanvas / ipc.canvasToLocal、ugoe_editor_core::space)。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import { ipc, type Keyform } from "../../../ipc";
import {
  cellIndex,
  currentCell,
  editablePoints,
  elementGrid,
  type Point,
  parseModel,
  snapToKeys,
  withPoint,
} from "../../../panels/inspector/keyforms/keyforms";
import type { Selection } from "../../../selection.svelte";
import type { ViewportController } from "../../controller.svelte";

interface Props {
  controller: ViewportController;
  doc: EditorDocument;
  selection: Selection;
}

let { controller, doc, selection }: Props = $props();

let enabled = $state(false);
/** 今のセルの点のキャンバス座標 (バックエンドで変換した結果)。 */
let canvasPoints = $state<Point[] | null>(null);
let error = $state<string | null>(null);
let drag = $state<{
  pointerId: number;
  index: number;
  all: boolean;
  start: Point;
  delta: Point;
} | null>(null);
let busy = $state(false);

const model = $derived(doc.model ? parseModel(doc.model.json) : null);
const element = $derived.by(() => {
  const e = model ? elementGrid(model, selection.primary) : null;
  return e && e.kind !== "part" ? e : null;
});
const params = $derived(model?.parameters ?? []);
const bindings = $derived(element?.grid.bindings ?? []);
const cell = $derived(element ? currentCell(bindings, controller.parameterValues, params) : null);
const keyform = $derived.by(() => {
  if (!element || !cell) return null;
  const index = cellIndex(bindings, cell);
  return (element.grid.keyforms[index] as Record<string, unknown> | undefined) ?? null;
});
const localPoints = $derived(element && keyform ? editablePoints(element.kind, keyform) : []);

// 表示する点を変換し直す (モデル・選択・パラメータ値が変わるたび)。古い要求の結果は捨てる。
let request = 0;
$effect(() => {
  const target = element?.target;
  const points = localPoints;
  const values = $state.snapshot(controller.parameterValues);
  const id = ++request;
  if (!enabled || !target || points.length === 0) {
    canvasPoints = null;
    return;
  }
  ipc
    .localToCanvas(target, values, points)
    .then((result) => {
      if (id !== request) return;
      canvasPoints = result;
      error = null;
    })
    .catch((e) => {
      if (id !== request) return;
      canvasPoints = null;
      error = String(e);
    });
});

/** ドラッグ中のプレビューを反映したキャンバス座標。 */
const shown = $derived.by(() => {
  const points = canvasPoints;
  if (!points || !drag) return points;
  const [dx, dy] = drag.delta;
  return points.map((p, i): Point => (drag?.all || i === drag?.index ? [p[0] + dx, p[1] + dy] : p));
});

const screen = $derived.by(() => {
  // transform が変わったとき (ズーム・パン) にも描き直す。
  void controller.transform;
  return (shown ?? []).map((p) => controller.canvasToScreen(p[0], p[1]) ?? [0, 0]);
});

/** 点どうしを結ぶ線 (メッシュの三角形の辺・ワープの格子)。 */
const edges = $derived.by((): [number, number][] => {
  if (!element) return [];
  if (element.kind === "mesh") {
    const out: [number, number][] = [];
    const tri = element.indices ?? [];
    for (let i = 0; i + 2 < tri.length; i += 3) {
      const [a, b, c] = [tri[i] ?? 0, tri[i + 1] ?? 0, tri[i + 2] ?? 0];
      out.push([a, b], [b, c], [c, a]);
    }
    return out;
  }
  if (element.kind === "warp") {
    const cols = element.cols ?? 0;
    const rows = element.rows ?? 0;
    const at = (x: number, y: number) => y * (cols + 1) + x;
    const out: [number, number][] = [];
    for (let y = 0; y <= rows; y++) {
      for (let x = 0; x <= cols; x++) {
        if (x < cols) out.push([at(x, y), at(x + 1, y)]);
        if (y < rows) out.push([at(x, y), at(x, y + 1)]);
      }
    }
    return out;
  }
  return [];
});

function canvasAt(e: PointerEvent): Point | null {
  const svg = (e.currentTarget as Element).closest("svg");
  if (!svg) return null;
  const rect = svg.getBoundingClientRect();
  return controller.screenToCanvas(e.clientX - rect.left, e.clientY - rect.top);
}

function onPointerDown(index: number, e: PointerEvent) {
  if (e.button !== 0 || e.altKey || busy) return;
  const start = canvasAt(e);
  if (!start) return;
  e.preventDefault();
  e.stopPropagation();
  (e.currentTarget as Element).setPointerCapture(e.pointerId);
  drag = { pointerId: e.pointerId, index, all: e.shiftKey, start, delta: [0, 0] };
}

function onPointerMove(e: PointerEvent) {
  if (drag?.pointerId !== e.pointerId) return;
  const p = canvasAt(e);
  if (p) drag.delta = [p[0] - drag.start[0], p[1] - drag.start[1]];
}

async function onPointerUp(e: PointerEvent) {
  const d = drag;
  if (d?.pointerId !== e.pointerId) return;
  if ((d.delta[0] === 0 && d.delta[1] === 0) || !element || !cell || !keyform || !canvasPoints) {
    drag = null;
    return;
  }
  busy = true;
  try {
    const indices = d.all ? localPoints.map((_, i) => i) : [d.index];
    const targets = indices.map((i): Point => {
      const p = canvasPoints?.[i] ?? [0, 0];
      return [p[0] + d.delta[0], p[1] + d.delta[1]];
    });
    const guesses = indices.map((i): Point => localPoints[i] ?? [0, 0]);
    const local = await ipc.canvasToLocal(
      element.target,
      $state.snapshot(controller.parameterValues),
      targets,
      guesses,
    );
    let next: Record<string, unknown> = keyform;
    indices.forEach((index, k) => {
      const point: Point = local[k] ?? guesses[k] ?? [0, 0];
      next = withPoint(element.kind, next, index, point);
    });
    await doc.apply({
      op: "set_keyform_cell",
      target: element.target,
      cell,
      // 形はモデルの JSON のセルをそのまま写したもの (点だけを置き換えた)。
      keyform: { ...next, kind: element.kind } as unknown as Keyform,
    });
    error = null;
  } catch (err) {
    error = String(err);
  } finally {
    busy = false;
    drag = null;
  }
}

function moveToNearestKey() {
  for (const [id, value] of Object.entries(
    snapToKeys(bindings, controller.parameterValues, params),
  )) {
    controller.setParameter(id, value);
  }
}
</script>

<div class="bar">
  <button class:active={enabled} onclick={() => (enabled = !enabled)} title="Edit the keyform of the current cell">
    Form edit
  </button>
  {#if enabled && element}
    {#if cell}
      <span class="muted">
        {element.target.id}
        {bindings.map((b) => `${b.parameter}=${cell[b.parameter]}`).join(" ") || "(no axes)"}
      </span>
    {:else}
      <span class="warn">Not on a key: editing would not know which cell to write.</span>
      <button onclick={moveToNearestKey}>Move to nearest key</button>
    {/if}
  {:else if enabled}
    <span class="muted">Select a mesh or deformer</span>
  {/if}
  {#if error}<span class="error">{error}</span>{/if}
</div>

{#if enabled && element && cell && shown}
  <svg class="form" role="presentation">
    {#each edges as [a, b], i (i)}
      {@const p = screen[a]}
      {@const q = screen[b]}
      {#if p && q}<line x1={p[0]} y1={p[1]} x2={q[0]} y2={q[1]} />{/if}
    {/each}
    {#each screen as p, i (i)}
      <circle
        class="handle"
        class:origin={element.kind === "rotation"}
        class:active={drag !== null && (drag.all || drag.index === i)}
        cx={p[0]}
        cy={p[1]}
        r={element.kind === "rotation" ? 7 : 4}
        role="presentation"
        onpointerdown={(e) => onPointerDown(i, e)}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={() => (drag = null)}
      />
    {/each}
  </svg>
{/if}

<style>
/* ToolOverlay のツールバーの下に置く。 */
.bar {
  position: absolute;
  top: 40px;
  left: 8px;
  display: flex;
  gap: 8px;
  align-items: center;
  pointer-events: auto;
}

.bar button.active {
  outline: 2px solid var(--accent);
}

.warn {
  color: #b45309;
}

.form {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: visible;
  pointer-events: none;
}

line {
  stroke: var(--accent);
  stroke-width: 1;
  opacity: 0.6;
}

.handle {
  fill: var(--panel);
  stroke: var(--accent);
  stroke-width: 1.5;
  pointer-events: auto;
  cursor: move;
}

.handle.active {
  fill: var(--accent);
}

/* 塗りを透明にしても当たり判定が残るように、fill は none にしない。 */
.handle.origin {
  fill-opacity: 0.2;
  stroke-width: 2;
}
</style>
