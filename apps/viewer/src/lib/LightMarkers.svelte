<!--
  ビューポートに重ねる光源の印 (T0708)。ライティングを使うモデルで、光源を当てていて、印を出す設定のときだけ出す。

  - 平行光源: 表示しているキャンバスの中央に破線の円を描き、光が来る向きに取っ手を置く (円の中心 = 正面から、縁 = 真横から)。
  - 点光源: キャンバスの上の位置に取っ手を置く (手前の距離はパネルで変える)。面の上で光が届く範囲を破線の円で描く。
  - 取っ手だけがポインタを受けるので、取っ手をドラッグすると光源が動き、ほかの場所のドラッグは今までどおりパンになる。
    ドラッグの途中は描画だけを変え (`updateLighting` の `preview`)、離したときにパネルと同じ経路で覚える (`commitLighting`)。
  印の形はエディタの光源の取っ手 (apps/editor/src/lib/viewport/tools/lighting/LightingOverlay.svelte) に合わせる。
-->
<script lang="ts">
import {
  anglesFromScreen,
  directionalHandle,
  lightDial,
  pointFromScreen,
  pointHandle,
  pointRangeOnCanvas,
} from "./lighting";
import type { Player } from "./player.svelte";

let { player }: { player: Player } = $props();

let svg = $state<SVGSVGElement>();
let drag = $state<{ pointer: number; kind: "directional" | "point" } | null>(null);

let s = $derived(player.lighting);
let canvas = $derived(player.canvasSize);
let view = $derived(player.view);
let dial = $derived(canvas && view ? lightDial(canvas, view) : null);
let directional = $derived(
  dial && s.directional.enabled ? directionalHandle(dial, s.directional) : null,
);
let point = $derived(canvas && view && s.point.enabled ? pointHandle(canvas, view, s.point) : null);
let pointRange = $derived(
  canvas && view && s.point.enabled ? pointRangeOnCanvas(canvas, s.point) * view.scale : 0,
);

function local(e: PointerEvent): [number, number] {
  const r = svg?.getBoundingClientRect();
  return r ? [e.clientX - r.left, e.clientY - r.top] : [e.clientX, e.clientY];
}

function onDown(e: PointerEvent, kind: "directional" | "point") {
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
    <circle
      class="handle"
      class:dragging={drag?.kind === "directional"}
      cx={directional[0]}
      cy={directional[1]}
      r="9"
      fill={s.directional.color}
      role="button"
      tabindex="-1"
      aria-label="平行光源 (ドラッグで向きを変える)"
      data-light="directional"
      onpointerdown={(e) => onDown(e, "directional")}
      onpointermove={onMove}
      onpointerup={onUp}
      onpointercancel={onUp}
      ondblclick={(e) => e.stopPropagation()}
    >
      <title>平行光源: 取っ手のある向きから光が来る (中心 = 正面、縁 = 真横)</title>
    </circle>
  {/if}
  {#if point}
    {#if pointRange > 0}
      <circle class="range" cx={point[0]} cy={point[1]} r={pointRange} />
    {/if}
    <path class="rays" d={rays(point[0], point[1])} />
    <circle
      class="handle"
      class:dragging={drag?.kind === "point"}
      cx={point[0]}
      cy={point[1]}
      r="9"
      fill={s.point.color}
      role="button"
      tabindex="-1"
      aria-label="点光源 (ドラッグで動かす)"
      data-light="point"
      onpointerdown={(e) => onDown(e, "point")}
      onpointermove={onMove}
      onpointerup={onUp}
      onpointercancel={onUp}
      ondblclick={(e) => e.stopPropagation()}
    >
      <title>点光源: ドラッグで動かす (手前の距離と範囲はパネルで変える)</title>
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
  .handle.dragging {
    stroke: var(--accent);
    stroke-width: 3;
    cursor: grabbing;
  }
</style>
