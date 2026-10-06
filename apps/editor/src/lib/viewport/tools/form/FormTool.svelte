<!--
  フォーム編集ツール (docs/editor.md#キーフォームの編集)。主な選択の要素について、今のパラメータ値の
  セルのキーフォームをビューポートで編集する: メッシュの頂点・ワープの制御点・回転デフォーマの原点を選んで
  ドラッグし、離したときに set_keyform_cell を 1 つ送る (ドラッグ中はオーバーレイだけでプレビューする)。

  - 点のクリックで選ぶ (Shift で追加・解除)。要素の内側 (メッシュの三角形・ワープの格子) の何もない所からの
    ドラッグで矩形選択 (Shift で追加)、クリックで選択を解く。外側のクリックは下の選択レイヤーに通す。
  - 選んだ点のドラッグで、選んだ点をまとめて同じだけ動かす。「All」・Ctrl (Cmd) + A ですべて選ぶ。Esc で解く。
  - キーボード: `]` `[` で次・前の点を 1 つ選び (帯の読み上げの領域に何番目かを出す)、矢印キーで選んだ点を動かす。

  パラメータ値がどれかの軸でキーの上にないときは編集しない (どのセルに書くか決まらないため)。
  代わりに「最も近いキーへ移動」を出す。

  点は親の座標系で持つので、表示と書き込みの前にバックエンドで変換する
  (ipc.localToCanvas / ipc.canvasToLocal、ugoe_editor_core::space)。
-->
<script lang="ts">
import { Button, Icon } from "@ugoe/ui";
import type { EditorDocument } from "../../../document.svelte";
import { errorText } from "../../../errors";
import { t } from "../../../i18n";
import { ipc, type Keyform } from "../../../ipc";
import {
  cellIndex,
  currentCell,
  editablePoints,
  elementGrid,
  modelData,
  type Point,
  snapToKeys,
  withPoint,
} from "../../../panels/inspector/keyforms/keyforms";
import type { Selection } from "../../../selection.svelte";
import TipButton from "../../../ui/TipButton.svelte";
import type { ViewportController } from "../../controller.svelte";
import { flatten, gridEdges, gridOutline, movePoints } from "../deformer/gizmo";
import { type Box, meshEdges, updateVertexSelection, verticesInBox } from "../geometry";
import { acceptsNudgeKey, acceptsToolKey, nudgeOf, pointStepOf, steppedPoint } from "../keys";
import { pressPoint, trianglesPath, validIndices } from "./form";

interface Props {
  controller: ViewportController;
  doc: EditorDocument;
  selection: Selection;
  /** フォーム編集を出すか (ToolOverlay の道具の並びの「Form edit」で切り替える)。 */
  enabled: boolean;
}

let { controller, doc, selection, enabled }: Props = $props();
/** 今のセルの点のキャンバス座標 (バックエンドで変換した結果)。 */
let canvasPoints = $state<Point[] | null>(null);
let error = $state<string | null>(null);
/** これより動かなければクリックとみなす (CSS ピクセル)。 */
const CLICK_SLOP = 3;

type Drag =
  | { kind: "points"; pointerId: number; start: Point; delta: Point }
  | { kind: "box"; pointerId: number; box: Box; toggle: boolean };

let drag = $state<Drag | null>(null);
let busy = $state(false);
/** 選んだ点 (要素ごと。キーは `種類:ID`)。 */
let selected = $state<{ key: string; indices: number[] } | null>(null);

const model = $derived(modelData(doc.parsedModel));
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
const selectionKey = $derived(element ? `${element.target.kind}:${element.target.id}` : "");
const picked = $derived(
  selected && selected.key === selectionKey
    ? validIndices(selected.indices, localPoints.length)
    : [],
);
const pickedSet = $derived(new Set(picked));
/** 1 つだけ選んだ点 (`]` `[` で選ぶ点。囲みを描き、帯に何番目かを出す)。 */
const current = $derived(picked.length === 1 ? (picked[0] ?? null) : null);

function setPicked(indices: number[]): void {
  selected = { key: selectionKey, indices };
}

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
      error = errorText(e);
    });
});

/** キャンバス座標の点の画面上の位置 (CSS ピクセル)。 */
const base = $derived.by(() => {
  // transform が変わったとき (ズーム・パン) にも描き直す。
  void controller.transform;
  return (canvasPoints ?? []).map((p): Point => controller.canvasToScreen(p[0], p[1]) ?? [0, 0]);
});

/** ドラッグ中のプレビューを反映した画面上の位置。 */
const screen = $derived(drag?.kind === "points" ? movePoints(base, picked, drag.delta) : base);

/** 1 つだけ選んだ点の囲みの中心。 */
const ring = $derived(current !== null ? screen[current] : undefined);

/** この内側のドラッグで矩形選択する (メッシュの三角形・ワープの格子の外周)。外側は選択レイヤーに通す。 */
const area = $derived.by(() => {
  if (!element) return "";
  if (element.kind === "mesh") return trianglesPath(base, element.indices ?? []);
  if (element.kind === "warp") {
    const ring = gridOutline(element.cols ?? 0, element.rows ?? 0).map((i) => base[i]);
    if (ring.some((p) => !p)) return "";
    return `M${ring.map((p) => `${p?.[0]},${p?.[1]}`).join("L")}Z`;
  }
  return "";
});

/** 点どうしを結ぶ線 (メッシュの三角形の辺 (共有する辺は 1 本)・ワープの格子)。 */
const edges = $derived.by((): [number, number][] => {
  if (element?.kind === "mesh") return meshEdges(element.indices ?? []);
  if (element?.kind === "warp") return gridEdges(element.cols ?? 0, element.rows ?? 0);
  return [];
});

function local(e: PointerEvent): Point {
  const svg = (e.currentTarget as Element).closest("svg");
  const r = svg?.getBoundingClientRect();
  return r ? [e.clientX - r.left, e.clientY - r.top] : [e.clientX, e.clientY];
}

/** 押した要素 (点・内側) にポインタを捕まえる。移動と離す操作はその要素で受ける。 */
function capture(e: PointerEvent): void {
  e.preventDefault();
  e.stopPropagation();
  (e.currentTarget as Element).setPointerCapture(e.pointerId);
}

function onPointPointerDown(index: number, e: PointerEvent) {
  // 中ボタンと Alt + ドラッグはビューポートのパン (Viewport.svelte) に任せる。
  if (e.button !== 0 || e.altKey || busy) return;
  capture(e);
  const next = pressPoint(picked, index, e.shiftKey);
  setPicked(next.picked);
  const start = local(e);
  if (next.drag) drag = { kind: "points", pointerId: e.pointerId, start, delta: [0, 0] };
}

function onAreaPointerDown(e: PointerEvent) {
  if (e.button !== 0 || e.altKey || busy) return;
  capture(e);
  const [x, y] = local(e);
  drag = {
    kind: "box",
    pointerId: e.pointerId,
    box: { x0: x, y0: y, x1: x, y1: y },
    toggle: e.shiftKey,
  };
}

function onPointerMove(e: PointerEvent) {
  // 送っている間は、送ったドラッグを動かさない・送り直さない (離した後の移動と、別の押下の離す操作が届く)。
  if (busy || drag?.pointerId !== e.pointerId) return;
  const [x, y] = local(e);
  if (drag.kind === "points") {
    drag.delta = [x - drag.start[0], y - drag.start[1]];
  } else {
    drag.box.x1 = x;
    drag.box.y1 = y;
  }
}

async function onPointerUp(e: PointerEvent) {
  const d = drag;
  if (busy || d?.pointerId !== e.pointerId) return;
  if (d.kind === "box") {
    drag = null;
    const { box } = d;
    if (Math.hypot(box.x1 - box.x0, box.y1 - box.y0) < CLICK_SLOP) {
      if (!d.toggle) setPicked([]);
    } else {
      setPicked(updateVertexSelection(picked, verticesInBox(flatten(base), box), d.toggle));
    }
    return;
  }
  if (Math.hypot(d.delta[0], d.delta[1]) < CLICK_SLOP) {
    drag = null;
    return;
  }
  await movePicked(d.delta);
}

/**
 * 選んだ点を画面の CSS ピクセルで `delta` だけ動かし、今のセルに set_keyform_cell を 1 つ送る
 * (ドラッグを離したときと、矢印キー)。送り終えたらドラッグのプレビューを消す。
 */
async function movePicked(delta: Point): Promise<void> {
  const scale = controller.transform?.scale ?? 1;
  if (busy || picked.length === 0 || !element || !cell || !keyform || !canvasPoints) {
    drag = null;
    return;
  }
  busy = true;
  try {
    const indices = picked;
    const moved: Point = [delta[0] / scale, delta[1] / scale];
    const targets = indices.map((i): Point => {
      const p = canvasPoints?.[i] ?? [0, 0];
      return [p[0] + moved[0], p[1] + moved[1]];
    });
    const guesses = indices.map((i): Point => localPoints[i] ?? [0, 0]);
    const mapped = await ipc.canvasToLocal(
      element.target,
      $state.snapshot(controller.parameterValues),
      targets,
      guesses,
    );
    let next: Record<string, unknown> = keyform;
    indices.forEach((index, k) => {
      const point: Point = mapped[k] ?? guesses[k] ?? [0, 0];
      next = withPoint(element.kind, next, index, point);
    });
    const report = await doc.apply({
      op: "set_keyform_cell",
      target: element.target,
      cell,
      // 形はモデルの JSON のセルをそのまま写したもの (点だけを置き換えた)。
      keyform: { ...next, kind: element.kind } as unknown as Keyform,
    });
    error = report ? null : doc.error;
  } catch (err) {
    error = errorText(err);
  } finally {
    busy = false;
    drag = null;
  }
}

function selectAll(): void {
  setPicked(localPoints.map((_, i) => i));
}

function onKeyDown(e: KeyboardEvent): void {
  if (!enabled || !element || !cell || !acceptsToolKey(e.target)) return;
  if (e.key === "Escape" && picked.length > 0) setPicked([]);
  else if (e.key === "a" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    selectAll();
  } else if (acceptsNudgeKey(e.target)) {
    // `]` `[` で点を 1 つずつ選び、矢印キーで選んだ点を動かす
    // (クリックとドラッグの代わり。docs/editor.md#ビューポートの操作と代わりの手段)
    const dir = pointStepOf(e);
    const next = dir ? steppedPoint(picked, localPoints.length, dir) : null;
    const step = picked.length > 0 ? nudgeOf(e) : null;
    if (next !== null) {
      e.preventDefault();
      if (!busy && drag === null) setPicked([next]);
    } else if (step) {
      e.preventDefault();
      void movePicked(step);
    }
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

<svelte:window onkeydown={onKeyDown} />

{#if enabled}
  <!-- 説明の帯 (ToolOverlay の .strip の中で、道具の並びの下に積む) -->
  <div class="vp-band">
    {#if element}
      {#if cell}
        <span>
          {element.target.id}
          {bindings.map((b) => `${b.parameter}=${cell[b.parameter]}`).join(" ") ||
            t("viewport.cell.noAxes")}
        </span>
        {#if localPoints.length > 1}
          <TipButton
            small
            onclick={selectAll}
            tooltip={t("viewport.form.selectAll.tooltip")}
            shortcut="Mod+A">{t("viewport.cell.selectAll")}</TipButton
          >
          <span class="muted" role="status"
            >{current !== null
              ? t("viewport.point.current", { index: current + 1, total: localPoints.length })
              : t("viewport.form.selected", { count: picked.length, total: localPoints.length })}</span
          >
        {/if}
      {:else}
        <span class="warn"><Icon name="warning" />{t("viewport.cell.notOnKey")}</span>
        <Button small onclick={moveToNearestKey}>{t("viewport.cell.moveToNearestKey")}</Button>
      {/if}
    {:else}
      <span class="muted">{t("viewport.form.empty")}</span>
    {/if}
    {#if error}<span class="error ugoe-selectable">{error}</span>{/if}
  </div>
{/if}

{#if enabled && element && cell && canvasPoints}
  <svg class="form" role="presentation" onpointermove={onPointerMove} onpointerup={onPointerUp}>
    {#if area}
      <path
        class="area"
        d={area}
        role="presentation"
        onpointerdown={onAreaPointerDown}
        onpointercancel={() => (drag = null)}
      />
    {/if}
    {#each edges as [a, b], i (i)}
      {@const p = screen[a]}
      {@const q = screen[b]}
      {#if p && q}<line x1={p[0]} y1={p[1]} x2={q[0]} y2={q[1]} />{/if}
    {/each}
    {#each screen as p, i (i)}
      <circle
        class="handle"
        class:origin={element.kind === "rotation"}
        class:selected={pickedSet.has(i)}
        cx={p[0]}
        cy={p[1]}
        r={element.kind === "rotation" ? 7 : 4}
        role="presentation"
        onpointerdown={(e) => onPointPointerDown(i, e)}
        onpointercancel={() => (drag = null)}
      />
    {/each}
    {#if ring}<circle class="ring" cx={ring[0]} cy={ring[1]} r={element.kind === "rotation" ? 11 : 8} />{/if}
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
{/if}

<style>
.warn {
  display: inline-flex;
  align-items: center;
  gap: var(--ugoe-space-1);
  color: var(--ugoe-color-warning);
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
  stroke: var(--viewport-edit);
  stroke-width: 1;
}

.handle {
  fill: var(--viewport-point);
  stroke: var(--viewport-edit);
  stroke-width: 1.5;
  pointer-events: auto;
  cursor: move;
}

.handle.selected {
  fill: var(--viewport-picked);
  stroke: var(--viewport-picked-edge);
}

/* 1 つだけ選んだ点 (`]` `[` で選ぶ点) の囲み。密な格子の中でも見つけやすくする */
.ring {
  fill: none;
  stroke: var(--viewport-picked-edge);
  stroke-width: 1.5;
  pointer-events: none;
}

.area {
  fill: transparent;
  pointer-events: auto;
  cursor: crosshair;
}

.box {
  fill: color-mix(in srgb, var(--viewport-edit) 12%, transparent);
  stroke: var(--viewport-edit);
  stroke-dasharray: 4 3;
  pointer-events: none;
}

/* 塗りを透明にしても当たり判定が残るように、fill は none にしない。 */
.handle.origin {
  fill-opacity: 0.2;
  stroke-width: 2;
}
</style>
