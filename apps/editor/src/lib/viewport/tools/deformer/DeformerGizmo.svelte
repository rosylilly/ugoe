<!--
  デフォーマのギズモ (docs/editor.md#デフォーマの編集)。主な選択がデフォーマのときに ToolOverlay が出す。
  今のパラメータ値のセルのキーフォームを編集し、離したときに set_keyform_cell を 1 つ送る
  (ドラッグ中はギズモだけをプレビューする)。

  - ワープ: 制御点のクリックで選ぶ (Shift で追加・解除)、何もない所からのドラッグで矩形選択 (Shift で追加)、
    選んだ点のドラッグでまとめて動かす。「All」ですべて選ぶ。選んだ点は Selection.controlPoints に持つ
    (インスペクタの深度の節が「選んだ点の深度」に使う)。`]` `[` で次・前の点を 1 つ選び (帯の読み上げの領域に
    何番目かを出す)、矢印キーで選んだ点を動かす。
  - 回転デフォーマ: 原点 (円) のドラッグで移動、上の取っ手で回転 (Shift で 15 度刻み)、右の四角で拡大率。
    原点は今のセルだけを動かす。「All cells」を付けるか Shift を押しながらドラッグすると、すべてのセルの原点を
    同じだけ動かし、子の絵は動かさない (回転の中心だけを動かす。move_rotation_origin)。

  形は wasm (WebPuppet.warp_points / rotation_frame) で評価した今のパラメータ値でのキャンバス座標。
  書き込む点はバックエンドで親の座標系に直す (ipc.canvasToLocal)。
  パラメータ値がどれかの軸でキーの上にないときは編集しない (フォーム編集と同じ。T0608)。
-->
<script lang="ts">
import { Button, Checkbox, Icon, Tooltip } from "@ugoe/ui";
import type { EditorDocument } from "../../../document.svelte";
import { errorText } from "../../../errors";
import { t } from "../../../i18n";
import { ipc, type Keyform } from "../../../ipc";
import {
  cellIndex,
  currentCell,
  elementGrid,
  modelData,
  snapToKeys,
} from "../../../panels/inspector/keyforms/keyforms";
import type { Selection } from "../../../selection.svelte";
import type { ViewportController } from "../../controller.svelte";
import { type Box, nearestVertex, updateVertexSelection, verticesInBox } from "../geometry";
import { acceptsNudgeKey, acceptsToolKey, nudgeOf, pointStepOf, steppedPoint } from "../keys";
import {
  addTurn,
  flatten,
  gridEdges,
  gridOutline,
  MIN_SCALE,
  movePoints,
  type Point,
  rotationFrame,
  rotationHandles,
  rotationSense,
  scaleFactor,
  snapAngle,
  toPoints,
  transformFrame,
} from "./gizmo";

interface Props {
  controller: ViewportController;
  doc: EditorDocument;
  selection: Selection;
  deformerId: string;
}

let { controller, doc, selection, deformerId }: Props = $props();

/** 点をつかめる距離 (CSS ピクセル)。 */
const PICK_RADIUS = 8;
const CLICK_SLOP = 3;
/** 回転・拡大率の取っ手の、原点からの距離 (CSS ピクセル)。 */
const HANDLE_LENGTH = 60;

type Drag =
  | { kind: "points"; id: number; start: Point; delta: Point }
  | { kind: "box"; id: number; box: Box; toggle: boolean }
  | { kind: "origin"; id: number; start: Point; delta: Point; all: boolean }
  | {
      kind: "rotate";
      id: number;
      start: Point;
      now: Point;
      /** 押してから回した角度 (度。1 周を超えても足していく)。 */
      turn: number;
      /** 押した所から `CLICK_SLOP` 以上離れたことがあるか (1 周して戻ってもクリックとみなさない)。 */
      far: boolean;
      snap: boolean;
    }
  | { kind: "scale"; id: number; start: Point; now: Point };

let svg = $state<SVGSVGElement>();
let drag = $state<Drag | null>(null);
let busy = $state(false);
/** 原点のドラッグをすべてのセルに適用する (Shift を押しながらのドラッグも同じ)。 */
let originAllCells = $state(false);
let error = $state<string | null>(null);
/** WebPuppet が新しいモデル・パラメータを反映した後に読み直すための印。 */
let tick = $state(0);

$effect(() => {
  void doc.model;
  void controller.transform;
  void controller.status;
  for (const value of Object.values(controller.parameterValues)) void value;
  queueMicrotask(() => (tick += 1));
});

const model = $derived(modelData(doc.parsedModel));
const element = $derived(model ? elementGrid(model, { kind: "deformer", id: deformerId }) : null);
const params = $derived(model?.parameters ?? []);
const bindings = $derived(element?.grid.bindings ?? []);
const cell = $derived(element ? currentCell(bindings, controller.parameterValues, params) : null);
const keyform = $derived.by(() => {
  if (!element || !cell) return null;
  return (element.grid.keyforms[cellIndex(bindings, cell)] as Record<string, unknown>) ?? null;
});

function toScreen(p: Point): Point {
  return controller.canvasToScreen(p[0], p[1]) ?? [0, 0];
}

/** ワープの制御点 (キャンバス座標)。 */
const warpCanvas = $derived.by((): Point[] | null => {
  void tick;
  const puppet = controller.puppet;
  if (element?.kind !== "warp" || !puppet || controller.status !== "ready") return null;
  const flat = puppet.warp_points(deformerId);
  const n = ((element.rows ?? 0) + 1) * ((element.cols ?? 0) + 1);
  return flat && flat.length === n * 2 ? toPoints(flat) : null;
});

/** 回転デフォーマのワールド変換 (キャンバス座標)。 */
const rotationCanvas = $derived.by(() => {
  void tick;
  const puppet = controller.puppet;
  if (element?.kind !== "rotation" || !puppet || controller.status !== "ready") return null;
  return rotationFrame(puppet.rotation_frame(deformerId));
});

/** 選んだ制御点 (インスペクタの深度の節も使うので Selection に持つ)。 */
const picked = $derived(
  (selection.controlPoints[deformerId] ?? []).filter((i) => i < (warpCanvas?.length ?? 0)),
);

function setPicked(indices: number[]): void {
  selection.controlPoints = { ...selection.controlPoints, [deformerId]: indices };
}

const scale = $derived(controller.transform?.scale ?? 1);
const warpScreen = $derived.by(() => {
  if (!warpCanvas) return null;
  const points = warpCanvas.map(toScreen);
  return drag?.kind === "points" ? movePoints(points, picked, drag.delta) : points;
});
const edges = $derived(
  element?.kind === "warp" ? gridEdges(element.cols ?? 0, element.rows ?? 0) : [],
);
const pickedSet = $derived(new Set(picked));
/** 1 つだけ選んだ制御点 (`]` `[` で選ぶ点。囲みを描き、帯に何番目かを出す)。 */
const current = $derived(picked.length === 1 ? (picked[0] ?? null) : null);
/** 格子の外周 (この内側のドラッグで矩形選択。外側のクリックはメッシュの選択に通す)。 */
const outline = $derived.by(() => {
  if (!warpScreen || element?.kind !== "warp") return "";
  const ring = gridOutline(element.cols ?? 0, element.rows ?? 0).map((i) => warpScreen[i]);
  return `M${ring.map((p) => `${p?.[0] ?? 0},${p?.[1] ?? 0}`).join("L")}Z`;
});

/** 回転の取っ手 (CSS ピクセル。ドラッグ中はプレビュー)。 */
const handles = $derived.by(() => {
  if (!rotationCanvas) return null;
  const frame = transformFrame(rotationCanvas, toScreen, scale);
  const h = rotationHandles(frame, HANDLE_LENGTH);
  if (drag?.kind === "origin") {
    const [dx, dy] = drag.delta;
    for (const p of [h.origin, h.rotate, h.scale]) {
      p[0] += dx;
      p[1] += dy;
    }
  } else if (drag?.kind === "rotate") {
    const turn = previewAngle(drag);
    const rot = (p: Point): Point => {
      const [x, y] = [p[0] - h.origin[0], p[1] - h.origin[1]];
      const r = (turn * Math.PI) / 180;
      return [
        h.origin[0] + x * Math.cos(r) - y * Math.sin(r),
        h.origin[1] + x * Math.sin(r) + y * Math.cos(r),
      ];
    };
    return { ...h, rotate: rot(h.rotate), scale: rot(h.scale), frame };
  } else if (drag?.kind === "scale") {
    const f = scaleFactor(h.origin, drag.start, drag.now) ?? 1;
    h.scale = [
      h.origin[0] + (h.scale[0] - h.origin[0]) * f,
      h.origin[1] + (h.scale[1] - h.origin[1]) * f,
    ];
  }
  return { ...h, frame };
});

function previewAngle(d: { turn: number; snap: boolean }): number {
  return d.snap ? snapAngle(d.turn, 15) : d.turn;
}

function local(e: PointerEvent): Point {
  const r = svg?.getBoundingClientRect();
  return r ? [e.clientX - r.left, e.clientY - r.top] : [e.clientX, e.clientY];
}

function onPointerDown(e: PointerEvent): void {
  // 中ボタンと Alt + ドラッグはビューポートのパン (Viewport.svelte) に任せる。
  if (e.button !== 0 || e.altKey || busy || !cell) return;
  const p = local(e);
  const id = e.pointerId;
  if (handles) {
    const near = (q: Point) => Math.hypot(q[0] - p[0], q[1] - p[1]) <= PICK_RADIUS;
    const kind = near(handles.rotate)
      ? "rotate"
      : near(handles.scale)
        ? "scale"
        : near(handles.origin)
          ? "origin"
          : null;
    if (!kind) return;
    capture(e);
    drag =
      kind === "origin"
        ? { kind, id, start: p, delta: [0, 0], all: originAllCells || e.shiftKey }
        : kind === "rotate"
          ? { kind, id, start: p, now: p, turn: 0, far: false, snap: e.shiftKey }
          : { kind, id, start: p, now: p };
    return;
  }
  if (!warpScreen) return;
  capture(e);
  const hit = nearestVertex(flatten(warpScreen), p[0], p[1], PICK_RADIUS);
  if (hit === null) {
    drag = { kind: "box", id, box: { x0: p[0], y0: p[1], x1: p[0], y1: p[1] }, toggle: e.shiftKey };
    return;
  }
  if (e.shiftKey) {
    setPicked(updateVertexSelection(picked, [hit], true));
    if (!picked.includes(hit)) return;
  } else if (!pickedSet.has(hit)) {
    setPicked([hit]);
  }
  drag = { kind: "points", id, start: p, delta: [0, 0] };
}

/** 押した要素 (取っ手・格子の内側) にポインタを捕まえる。移動と離す操作は svg に浮上してくる。 */
function capture(e: PointerEvent): void {
  e.stopPropagation();
  (e.target as Element).setPointerCapture(e.pointerId);
}

function onPointerMove(e: PointerEvent): void {
  // 送っている間は、送ったドラッグを動かさない・送り直さない (離した後の移動と、別の押下の離す操作が届く)。
  if (busy || drag?.id !== e.pointerId) return;
  const p = local(e);
  switch (drag.kind) {
    case "points":
    case "origin":
      drag.delta = [p[0] - drag.start[0], p[1] - drag.start[1]];
      break;
    case "box":
      drag.box.x1 = p[0];
      drag.box.y1 = p[1];
      break;
    case "rotate":
      if (handles) drag.turn = addTurn(drag.turn, handles.origin, drag.now, p);
      drag.far ||= Math.hypot(p[0] - drag.start[0], p[1] - drag.start[1]) >= CLICK_SLOP;
      drag.now = p;
      drag.snap = e.shiftKey;
      break;
    case "scale":
      drag.now = p;
      break;
  }
}

function onPointerUp(e: PointerEvent): void {
  const d = drag;
  if (busy || d?.id !== e.pointerId) return;
  if (d.kind === "box") {
    drag = null;
    const { box } = d;
    if (Math.hypot(box.x1 - box.x0, box.y1 - box.y0) < CLICK_SLOP) {
      if (!d.toggle) setPicked([]);
    } else if (warpScreen) {
      setPicked(updateVertexSelection(picked, verticesInBox(flatten(warpScreen), box), d.toggle));
    }
    return;
  }
  const moved =
    d.kind === "points" || d.kind === "origin"
      ? Math.hypot(d.delta[0], d.delta[1]) >= CLICK_SLOP
      : d.kind === "rotate"
        ? d.far
        : Math.hypot(d.now[0] - d.start[0], d.now[1] - d.start[1]) >= CLICK_SLOP;
  if (!moved) {
    drag = null;
    return;
  }
  commit(d);
}

/** ドラッグの結果を今のセルに書き込む。 */
async function commit(d: Drag): Promise<void> {
  const target = element?.target;
  const kf = keyform;
  const c = cell;
  if (!target || !kf || !c) {
    drag = null;
    return;
  }
  busy = true;
  try {
    const values = $state.snapshot(controller.parameterValues);
    let next: Record<string, unknown> | null = null;
    /** 送ったコマンドが失敗したか (失敗の理由は doc.error)。 */
    let failed = false;
    if (d.kind === "points" && warpCanvas && picked.length > 0) {
      const delta: Point = [d.delta[0] / scale, d.delta[1] / scale];
      const current = kf.points as Point[];
      const targets = picked.map((i): Point => {
        const p = warpCanvas[i] ?? [0, 0];
        return [p[0] + delta[0], p[1] + delta[1]];
      });
      const guesses = picked.map((i): Point => current[i] ?? [0, 0]);
      const mapped = await ipc.canvasToLocal(target, values, targets, guesses);
      const points = [...current];
      picked.forEach((index, k) => {
        points[index] = mapped[k] ?? current[index] ?? [0, 0];
      });
      next = { ...kf, points };
    } else if (d.kind === "origin" && rotationCanvas) {
      const o = rotationCanvas.origin;
      const moved: Point = [o[0] + d.delta[0] / scale, o[1] + d.delta[1] / scale];
      const current = kf.origin as Point;
      const [origin] = await ipc.canvasToLocal(target, values, [moved], [current]);
      if (origin && d.all) {
        // 今のセルでの移動量 (親の座標系) を、すべてのセルに加える。
        const delta: Point = [origin[0] - current[0], origin[1] - current[1]];
        failed = !(await doc.apply({ op: "move_rotation_origin", id: deformerId, delta }));
      } else if (origin) {
        next = { ...kf, origin };
      }
    } else if (d.kind === "rotate" && handles && rotationCanvas) {
      const sense = rotationSense(rotationCanvas, !!kf.reflect_x, !!kf.reflect_y);
      next = { ...kf, angle: (kf.angle as number) + sense * previewAngle(d) };
    } else if (d.kind === "scale" && handles) {
      const f = scaleFactor(handles.origin, d.start, d.now);
      if (f !== null) next = { ...kf, scale: Math.max(MIN_SCALE, (kf.scale as number) * f) };
    }
    if (next) {
      const report = await doc.apply({
        op: "set_keyform_cell",
        target,
        cell: c,
        keyform: { ...next, kind: element?.kind } as unknown as Keyform,
      });
      failed = report === null;
    }
    error = failed ? doc.error : null;
  } catch (err) {
    error = errorText(err);
  } finally {
    busy = false;
    drag = null;
  }
}

function selectAll(): void {
  if (warpCanvas) setPicked(warpCanvas.map((_, i) => i));
}

function moveToNearestKey(): void {
  for (const [id, value] of Object.entries(
    snapToKeys(bindings, controller.parameterValues, params),
  )) {
    controller.setParameter(id, value);
  }
}

// 選択が外れたら (Esc) 点の選択を解く。
function onKeyDown(e: KeyboardEvent): void {
  if (!acceptsToolKey(e.target)) return;
  if (e.key === "Escape" && picked.length > 0) setPicked([]);
  else if (e.key === "a" && (e.ctrlKey || e.metaKey) && warpCanvas) {
    e.preventDefault();
    selectAll();
  } else if (!step(e)) {
    nudge(e);
  }
}

/** `]` `[` でワープの次・前の制御点を 1 つ選ぶ (クリックの代わり)。キーを使ったら true。 */
function step(e: KeyboardEvent): boolean {
  const dir = pointStepOf(e);
  if (!dir || !warpCanvas || !acceptsNudgeKey(e.target)) return false;
  const next = steppedPoint(picked, warpCanvas.length, dir);
  if (next === null) return false;
  e.preventDefault();
  if (!busy && drag === null) setPicked([next]);
  return true;
}

/**
 * 矢印キーで、ワープの選んだ制御点か回転デフォーマの原点を動かす (ドラッグの代わり。
 * docs/editor.md#ビューポートの操作と代わりの手段)。原点は「Origin: all cells」に従う。1 回が Undo の 1 単位。
 */
function nudge(e: KeyboardEvent): void {
  if (busy || drag !== null || !cell || !acceptsNudgeKey(e.target)) return;
  const delta = nudgeOf(e);
  if (!delta) return;
  if (warpCanvas && picked.length > 0) {
    e.preventDefault();
    void commit({ kind: "points", id: -1, start: [0, 0], delta });
  } else if (rotationCanvas) {
    e.preventDefault();
    void commit({ kind: "origin", id: -1, start: [0, 0], delta, all: originAllCells });
  }
}
</script>

<svelte:window onkeydown={onKeyDown} />

<!-- 説明の帯 (ToolOverlay の .strip の中で、道具の並びの下に積む) -->
<div class="vp-band">
  <span
    >{t("viewport.gizmo.title", {
      kind: element?.kind === "warp" ? t("viewport.gizmo.kind.warp") : t("viewport.gizmo.kind.rotation"),
      id: deformerId,
    })}</span
  >
  {#if cell}
    <span class="muted">
      {bindings.map((b) => `${b.parameter}=${cell[b.parameter]}`).join(" ") ||
        t("viewport.cell.noAxes")}
    </span>
    {#if element?.kind === "warp"}
      <Button small onclick={selectAll}>{t("viewport.cell.selectAll")}</Button>
      <span class="muted" role="status"
        >{current !== null && warpCanvas
          ? t("viewport.point.current", { index: current + 1, total: warpCanvas.length })
          : ""}</span
      >
    {:else if element?.kind === "rotation"}
      <Tooltip text={t("viewport.gizmo.originAllCells.tooltip")}>
        {#snippet trigger(attrs)}
          <span class="check" {...attrs}>
            <Checkbox label={t("viewport.gizmo.originAllCells.label")} bind:checked={originAllCells} />
          </span>
        {/snippet}
      </Tooltip>
    {/if}
  {:else if element}
    <span class="warn"><Icon name="warning" />{t("viewport.cell.notOnKey")}</span>
    <Button small onclick={moveToNearestKey}>{t("viewport.cell.moveToNearestKey")}</Button>
  {/if}
  {#if error}<span class="error ugoe-selectable">{error}</span>{/if}
</div>

<svg
  bind:this={svg}
  class="gizmo"
  class:editable={cell !== null}
  role="application"
  aria-label={t("viewport.gizmo.aria")}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={() => (drag = null)}
>
  {#if warpScreen}
    <path class="area hit" d={outline} />
    <g class="grid">
      {#each edges as [a, b] (`${a},${b}`)}
        {@const p = warpScreen[a]}
        {@const q = warpScreen[b]}
        {#if p && q}<line x1={p[0]} y1={p[1]} x2={q[0]} y2={q[1]} />{/if}
      {/each}
    </g>
    <g class="points">
      {#each warpScreen as p, i (i)}
        <rect class="hit" class:selected={pickedSet.has(i)} x={p[0] - 3.5} y={p[1] - 3.5} width="7" height="7" />
      {/each}
    </g>
    {@const c = current !== null ? warpScreen[current] : undefined}
    {#if c}<rect class="ring" x={c[0] - 7} y={c[1] - 7} width="14" height="14" />{/if}
  {/if}
  {#if handles}
    {@const h = handles}
    <g class="rotation" class:active={drag !== null}>
      <line x1={h.origin[0]} y1={h.origin[1]} x2={h.rotate[0]} y2={h.rotate[1]} />
      <line x1={h.origin[0]} y1={h.origin[1]} x2={h.scale[0]} y2={h.scale[1]} />
      <circle class="origin hit" cx={h.origin[0]} cy={h.origin[1]} r="7" />
      <circle class="rotate hit" cx={h.rotate[0]} cy={h.rotate[1]} r="5" />
      <rect class="scale hit" x={h.scale[0] - 4.5} y={h.scale[1] - 4.5} width="9" height="9" />
    </g>
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
.warn {
  display: inline-flex;
  align-items: center;
  gap: var(--ugoe-space-1);
  color: var(--ugoe-color-warning);
}

.check {
  display: inline-flex;
  white-space: nowrap;
}

.gizmo {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  user-select: none;
}

/* 取っ手と格子の内側だけがポインタを受ける (外側のクリックは下の選択レイヤーに通す)。 */
.gizmo.editable .hit {
  pointer-events: auto;
  cursor: move;
}

.area {
  fill: transparent;
}

.gizmo.editable .area {
  cursor: crosshair;
}

.grid line {
  stroke: var(--viewport-warp);
  stroke-width: 1;
}

.points rect {
  fill: var(--viewport-point);
  stroke: var(--viewport-warp);
  stroke-width: 1.5;
}

.points rect.selected {
  fill: var(--viewport-picked);
  stroke: var(--viewport-picked-edge);
}

/* 1 つだけ選んだ点 (`]` `[` で選ぶ点) の囲み。密な格子の中でも見つけやすくする */
.ring {
  fill: none;
  stroke: var(--viewport-picked-edge);
  stroke-width: 1.5;
}

.rotation line {
  stroke: var(--viewport-rotation);
  stroke-width: 1.5;
}

.rotation circle,
.rotation rect {
  fill: var(--viewport-point);
  stroke: var(--viewport-rotation);
  stroke-width: 2;
}

/* 塗りを薄くしても当たり判定は残る (判定は距離で行う)。 */
.rotation .origin {
  fill-opacity: 0.3;
}

.rotation.active circle,
.rotation.active rect {
  fill: var(--viewport-rotation);
}

.box {
  fill: color-mix(in srgb, var(--viewport-warp) 12%, transparent);
  stroke: var(--viewport-warp);
  stroke-dasharray: 4 3;
}
</style>
