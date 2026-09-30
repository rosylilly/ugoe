<!--
  ビューポートに重ねる光源の印 (T0708)。ライティングを使うモデルで、光源を当てていて、印を出す設定のときだけ出す。

  - 平行光源: 表示しているキャンバスの中央に破線の円を描き、光が来る向きに取っ手を置く (円の中心 = 正面から、縁 = 真横から)。
  - 点光源: キャンバスの上の位置に取っ手を置く。面の上で光が届く範囲を破線の円で描き、円の縁のドラッグで範囲を変える。
    取っ手の上の Shift + ホイールで手前の距離を変える (T0714)。
  - 取っ手 (と範囲の円の縁) だけがポインタを受けるので、取っ手をドラッグすると光源が動き、ほかの場所のドラッグは
    今までどおりパンになる。ホイールは Shift を押していなければ取っ手の上でもズーム。
    ドラッグの途中は描画だけを変え (`updateLighting` の `preview`)、離したときにパネルと同じ経路で覚える (`commitLighting`)。
  - 取っ手は Tab でフォーカスでき、矢印キーなどで動かせる (lighting.ts の `stepMarker`。キーボードはすぐ覚える、T0714)。
  - 大きく拡大して印が画面の外に出るときは、平行光源の円・点光源の取っ手を画面の端に寄せる (T0714)。
  印の形はエディタの光源の取っ手 (apps/editor/src/lib/viewport/tools/lighting/LightingOverlay.svelte) に合わせる。
-->
<script lang="ts">
import {
  anglesFromScreen,
  directionalHandle,
  lightDial,
  pointFromScreen,
  pointHandle,
  pointHandleOnScreen,
  pointRangeOnCanvas,
  rangeFromScreenRadius,
  stepMarker,
  wheelPointZ,
} from "./lighting";
import type { Player } from "./player.svelte";

let { player }: { player: Player } = $props();

let svg = $state<SVGSVGElement>();
type DragKind = "directional" | "point" | "range";
let drag = $state<{ pointer: number; kind: DragKind } | null>(null);
/** ホイールで変えた手前の距離を覚えるまでの待ち (回している間は覚えない)。 */
let wheelTimer: ReturnType<typeof setTimeout> | undefined;
/** キーボードのフォーカスがある取っ手 (フォーカスの輪を描く)。 */
let focused = $state<"directional" | "point" | null>(null);

let s = $derived(player.lighting);
let canvas = $derived(player.canvasSize);
let view = $derived(player.view);
let bounds = $derived(player.viewportSize);
let dial = $derived(canvas && view ? lightDial(canvas, view, bounds) : null);
let directional = $derived(
  dial && s.directional.enabled ? directionalHandle(dial, s.directional) : null,
);
// 範囲の円は光源の本当の位置、取っ手は画面の外なら端に寄せた位置に描く。
let point = $derived(canvas && view && s.point.enabled ? pointHandle(canvas, view, s.point) : null);
let pointShown = $derived(
  canvas && view && s.point.enabled ? pointHandleOnScreen(canvas, view, s.point, bounds) : null,
);
let pointRange = $derived(
  canvas && view && s.point.enabled ? pointRangeOnCanvas(canvas, s.point) * view.scale : 0,
);

function local(e: PointerEvent): [number, number] {
  const r = svg?.getBoundingClientRect();
  return r ? [e.clientX - r.left, e.clientY - r.top] : [e.clientX, e.clientY];
}

function onDown(e: PointerEvent, kind: DragKind) {
  if (e.button !== 0) return;
  // 下のビューポートのパンを始めない。
  e.preventDefault();
  e.stopPropagation();
  (e.currentTarget as Element).setPointerCapture(e.pointerId);
  drag = { pointer: e.pointerId, kind };
}

function onMove(e: PointerEvent) {
  if (drag?.pointer !== e.pointerId || canvas === null || view === null) return;
  const [x, y] = local(e);
  if (drag.kind === "directional") {
    if (dial === null) return;
    const [azimuth, elevation] = anglesFromScreen(dial, x, y);
    player.updateLighting(
      (next) => {
        next.directional.azimuth = azimuth;
        next.directional.elevation = elevation;
      },
      { preview: true },
    );
  } else if (drag.kind === "range") {
    if (point === null) return;
    const range = rangeFromScreenRadius(
      canvas,
      view,
      s.point,
      Math.hypot(x - point[0], y - point[1]),
    );
    player.updateLighting(
      (next) => {
        next.point.range = range;
      },
      { preview: true },
    );
  } else {
    const [px, py] = pointFromScreen(canvas, view, x, y);
    player.updateLighting(
      (next) => {
        next.point.x = px;
        next.point.y = py;
      },
      { preview: true },
    );
  }
}

function onUp(e: PointerEvent) {
  if (drag?.pointer !== e.pointerId) return;
  drag = null;
  player.commitLighting();
}

function onKeyDown(e: KeyboardEvent, kind: "directional" | "point") {
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  // 動かすキーのときだけ覚える (キーボードはパネルと同じくすぐ覚える)。
  const next = $state.snapshot(s);
  if (!stepMarker(next, kind, e.key, e.shiftKey)) return;
  e.preventDefault();
  e.stopPropagation();
  player.setLighting(next);
}

/** 点光源の取っ手の上の Shift + ホイールで手前の距離を変える。Shift が無ければ下のビューポートのズームに任せる。 */
function onWheel(e: WheelEvent) {
  if (!e.shiftKey) return;
  e.preventDefault();
  e.stopPropagation();
  // Shift + ホイールを横のスクロールとして送るブラウザがあるので、横の量も見る。
  const delta = e.deltaY !== 0 ? e.deltaY : e.deltaX;
  const z = wheelPointZ(s.point, delta);
  player.updateLighting(
    (next) => {
      next.point.z = z;
    },
    { preview: true },
  );
  clearTimeout(wheelTimer);
  wheelTimer = setTimeout(() => player.commitLighting(), 400);
}

$effect(() => () => {
  // 印が消えるとき (光源を切ったなど) に、回していた分を覚える。
  if (wheelTimer !== undefined) {
    clearTimeout(wheelTimer);
    player.commitLighting();
  }
});

/** キーボードで来たフォーカスのときだけ輪を描く (クリックで掴んだときは出さない)。 */
function onFocus(e: FocusEvent, kind: "directional" | "point") {
  focused = (e.currentTarget as Element).matches(":focus-visible") ? kind : null;
}

const pct = (v: number) => `${Math.round(v * 100)}%`;

/** 点光源の印 (放射する短い線)。 */
function rays(x: number, y: number): string {
  let d = "";
  for (let i = 0; i < 8; i++) {
    const a = (i * Math.PI) / 4;
    d += `M${x + 12 * Math.cos(a)},${y + 12 * Math.sin(a)}L${x + 17 * Math.cos(a)},${y + 17 * Math.sin(a)}`;
  }
  return d;
}
</script>

<svg bind:this={svg} class="light-markers" role="presentation">
  {#if dial && directional}
    <circle class="dial" cx={dial.cx} cy={dial.cy} r={dial.r} />
    <circle class="center" cx={dial.cx} cy={dial.cy} r="3" />
    <line class="beam" x1={directional[0]} y1={directional[1]} x2={dial.cx} y2={dial.cy} />
    {#if focused === "directional"}
      <circle class="focus-ring" cx={directional[0]} cy={directional[1]} r="14" />
    {/if}
    <circle
      class="handle"
      class:dragging={drag?.kind === "directional"}
      cx={directional[0]}
      cy={directional[1]}
      r="9"
      fill={s.directional.color}
      role="slider"
      tabindex="0"
      aria-label="平行光源の来る向き (矢印キーで動かす、Shift で大きく)"
      aria-valuenow={s.directional.azimuth}
      aria-valuemin={-180}
      aria-valuemax={180}
      aria-valuetext="方位 {Math.round(s.directional.azimuth)}°、高さ {Math.round(s.directional.elevation)}°"
      data-light="directional"
      onpointerdown={(e) => onDown(e, "directional")}
      onpointermove={onMove}
      onpointerup={onUp}
      onpointercancel={onUp}
      onkeydown={(e) => onKeyDown(e, "directional")}
      onfocus={(e) => onFocus(e, "directional")}
      onblur={() => (focused = null)}
      ondblclick={(e) => e.stopPropagation()}
    >
      <title>平行光源: 取っ手のある向きから光が来る (中心 = 正面、縁 = 真横)。ドラッグか矢印キーで動かす</title>
    </circle>
  {/if}
  {#if point && pointShown}
    {#if pointRange > 0}
      <circle class="range" cx={point[0]} cy={point[1]} r={pointRange} />
      <!--
        範囲の円の縁 (見えない太い線) のドラッグで範囲を変える。内側と外側のドラッグはパンのまま。
        キーボードでは点光源の取っ手の + / − で変えるので、ここはフォーカスを受けない。
      -->
      <circle
        class="range-edge"
        class:dragging={drag?.kind === "range"}
        cx={point[0]}
        cy={point[1]}
        r={pointRange}
        data-light="range"
        role="presentation"
        onpointerdown={(e) => onDown(e, "range")}
        onpointermove={onMove}
        onpointerup={onUp}
        onpointercancel={onUp}
        ondblclick={(e) => e.stopPropagation()}
      >
        <title>点光源の届く範囲: 縁をドラッグで変える</title>
      </circle>
    {/if}
    <path class="rays" class:offscreen={pointShown.offscreen} d={rays(pointShown.x, pointShown.y)} />
    {#if focused === "point"}
      <circle class="focus-ring" cx={pointShown.x} cy={pointShown.y} r="14" />
    {/if}
    <circle
      class="handle"
      class:dragging={drag?.kind === "point"}
      class:offscreen={pointShown.offscreen}
      cx={pointShown.x}
      cy={pointShown.y}
      r="9"
      fill={s.point.color}
      role="slider"
      tabindex="0"
      aria-label="点光源 (矢印キーで動かす、Shift で大きく。PageUp・PageDown で手前、+・− で範囲)"
      aria-valuenow={s.point.x}
      aria-valuemin={-0.5}
      aria-valuemax={1.5}
      aria-valuetext="横 {pct(s.point.x)}、縦 {pct(s.point.y)}、手前 {pct(s.point.z)}、範囲 {pct(s.point.range)}"
      data-light="point"
      onpointerdown={(e) => onDown(e, "point")}
      onpointermove={onMove}
      onpointerup={onUp}
      onpointercancel={onUp}
      onkeydown={(e) => onKeyDown(e, "point")}
      onwheel={onWheel}
      onfocus={(e) => onFocus(e, "point")}
      onblur={() => (focused = null)}
      ondblclick={(e) => e.stopPropagation()}
    >
      <title
        >点光源: ドラッグか矢印キーで動かす。Shift + ホイール・PageUp・PageDown で手前、範囲の円の縁のドラッグ・+・− で範囲{pointShown.offscreen
          ? " (画面の外にあるので端に寄せて出している)"
          : ""}</title
      >
    </circle>
  {/if}
</svg>

<style>
  .light-markers {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
  }
  .dial {
    fill: none;
    stroke: #facc15;
    stroke-width: 1;
    stroke-dasharray: 4 4;
    opacity: 0.8;
  }
  .center {
    fill: #facc15;
  }
  .beam {
    stroke: #facc15;
    stroke-width: 1.5;
  }
  .range {
    fill: none;
    stroke: #facc15;
    stroke-dasharray: 6 4;
    opacity: 0.6;
  }
  .rays {
    stroke: #facc15;
    stroke-width: 2;
  }
  .handle {
    stroke: #1f2937;
    stroke-width: 2;
    pointer-events: auto;
    cursor: grab;
  }
  .handle:focus {
    outline: none;
  }
  .handle.offscreen {
    stroke-dasharray: 3 2;
  }
  .rays.offscreen {
    opacity: 0.5;
  }
  .focus-ring {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
    pointer-events: none;
  }
  .range-edge {
    fill: none;
    stroke: transparent;
    stroke-width: 10;
    pointer-events: stroke;
    cursor: ew-resize;
  }
  .range-edge.dragging {
    stroke: var(--accent);
    stroke-width: 2;
  }
  .handle.dragging {
    stroke: var(--accent);
    stroke-width: 3;
    cursor: grabbing;
  }
</style>
