<!--
  光源の取っ手 (docs/editor.md#ライティングの画面)。ライティングのパネルを開いている間とプレビュー中に重ねる。

  - 平行光源: キャンバスの中央に円を描き、光源のある向きに取っ手を置く (円の中心 = 正面から、縁 = 真横から)。
    取っ手をドラッグして向きを変える。裏から当てる光源 (「From behind the canvas」) は破線で描く。
  - 点光源: キャンバスの上の位置に取っ手を置き、ドラッグで動かす (奥行きはパネルで変える)。光が届く範囲を破線の円で描く。
  - ドラッグ中は表示 (LightingView.preview) だけを変え、離したときにプロジェクトに残す (LightingView.commit)。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import type { Model } from "../../../ipc";
import {
  directionToHandle,
  handleToDirection,
  rangeOnCanvas,
  rgbToHex,
} from "../../../panels/lighting/lighting";
import type { LightingView } from "../../../panels/lighting/lightingView.svelte";
import type { ViewportController } from "../../controller.svelte";

interface Props {
  controller: ViewportController;
  view: LightingView;
  doc: EditorDocument;
}

let { controller, view, doc }: Props = $props();

const canvas = $derived.by(() => {
  try {
    return doc.model ? (JSON.parse(doc.model.json) as Model).meta.canvas : null;
  } catch {
    return null;
  }
});

let svg = $state<SVGSVGElement>();
let drag: { pointer: number; index: number; moved: boolean } | null = null;

/** 平行光源の円の中心と半径 (CSS ピクセル)。 */
const dial = $derived.by(() => {
  const t = controller.transform;
  if (!t || !canvas) return null;
  const [cx, cy] = [
    t.offsetX + (t.scale * canvas.width) / 2,
    t.offsetY + (t.scale * canvas.height) / 2,
  ];
  const r = Math.min(220, Math.max(60, Math.min(canvas.width, canvas.height) * t.scale * 0.35));
  return { cx, cy, r };
});

const handles = $derived.by(() => {
  const t = controller.transform;
  const d = dial;
  if (!t || !d) return [];
  return view.settings.lights.map((light, index) => {
    const color = rgbToHex(light.color);
    if (light.type === "directional") {
      const h = directionToHandle(light.direction);
      return {
        index,
        kind: "directional" as const,
        x: d.cx + d.r * h.x,
        y: d.cy + d.r * h.y,
        behind: h.behind,
        color,
        range: null,
      };
    }
    const range = rangeOnCanvas(light);
    return {
      index,
      kind: "point" as const,
      x: t.offsetX + t.scale * light.position[0],
      y: t.offsetY + t.scale * light.position[1],
      behind: false,
      color,
      range: range === null ? null : range * t.scale,
    };
  });
});

const hasDirectional = $derived(view.settings.lights.some((l) => l.type === "directional"));

function local(e: PointerEvent): [number, number] {
  const r = svg?.getBoundingClientRect();
  return r ? [e.clientX - r.left, e.clientY - r.top] : [e.clientX, e.clientY];
}

function onDown(e: PointerEvent, index: number): void {
  if (e.button !== 0) return;
  e.preventDefault();
  e.stopPropagation();
  view.selected = index;
  drag = { pointer: e.pointerId, index, moved: false };
  (e.currentTarget as Element).setPointerCapture(e.pointerId);
}

function onMove(e: PointerEvent): void {
  if (drag?.pointer !== e.pointerId) return;
  const light = view.settings.lights[drag.index];
  const d = dial;
  if (!light || !d) return;
  const [x, y] = local(e);
  drag.moved = true;
  if (light.type === "directional") {
    const behind = directionToHandle(light.direction).behind;
    view.preview(
      view.withLight(drag.index, {
        ...light,
        direction: handleToDirection((x - d.cx) / d.r, (y - d.cy) / d.r, behind),
      }),
    );
  } else {
    const p = controller.screenToCanvas(x, y);
    if (!p) return;
    const position: [number, number, number] = [
      Math.round(p[0]),
      Math.round(p[1]),
      light.position[2],
    ];
    view.preview(view.withLight(drag.index, { ...light, position }));
  }
}

function onUp(e: PointerEvent): void {
  if (drag?.pointer !== e.pointerId) return;
  const moved = drag.moved;
  drag = null;
  if (moved) void view.commit();
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

<svg bind:this={svg} class="layer" role="presentation">
  {#if dial && hasDirectional}
    <circle class="dial" cx={dial.cx} cy={dial.cy} r={dial.r} />
    <circle class="center" cx={dial.cx} cy={dial.cy} r="3" />
  {/if}
  {#each handles as h (h.index)}
    {#if h.kind === "directional" && dial}
      <line class="beam" class:behind={h.behind} x1={h.x} y1={h.y} x2={dial.cx} y2={dial.cy} />
    {/if}
    {#if h.range !== null}
      <circle class="range" cx={h.x} cy={h.y} r={h.range} />
    {/if}
    {#if h.kind === "point"}<path class="rays" d={rays(h.x, h.y)} />{/if}
    <circle
      class="handle"
      class:behind={h.behind}
      class:current={view.selected === h.index}
      cx={h.x}
      cy={h.y}
      r="9"
      fill={h.behind ? "none" : h.color}
      stroke={h.behind ? h.color : undefined}
      role="button"
      tabindex="-1"
      aria-label="Light {h.index + 1}"
      onpointerdown={(e) => onDown(e, h.index)}
      onpointermove={onMove}
      onpointerup={onUp}
      onpointercancel={onUp}
    />
    <text class="label" x={h.x + 12} y={h.y - 10}>{h.index + 1}</text>
  {/each}
</svg>

<style>
.layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
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

.beam.behind {
  stroke-dasharray: 3 3;
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

.handle.behind {
  stroke-width: 3;
}

.handle.current {
  stroke: var(--accent);
  stroke-width: 3;
}

.label {
  fill: #facc15;
  font-size: 11px;
  paint-order: stroke;
  stroke: #0008;
  stroke-width: 3px;
}
</style>
