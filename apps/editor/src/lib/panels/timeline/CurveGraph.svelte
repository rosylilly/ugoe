<!--
  選んだ行のカーブのグラフ: キー (円) とベジェの取っ手 (四角) をドラッグで編集し、離したときに Command を 1 つ送る
  (キーは move_motion_key、取っ手は set_motion_segment)。ダブルクリックでその位置にキーを打つ。
  左の欄: 選んだキーの時刻と値、そのキーに入る区間の種類 (直線・ベジェ・ステップ)、カーブ個別のフェード、カーブの削除。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import type { Command, Curve, Motion, Point, Segment } from "../../ipc";
import {
  canMoveKey,
  clampHandle,
  curveKeys,
  curvePath,
  endPoint,
  type GraphView,
  keyCommand,
  SEGMENT_KINDS,
  type SegmentKind,
  segmentEndingAt,
  segmentEnds,
  segmentShape,
  snapToFrame,
  type Track,
  targetKey,
  timeToX,
  valueRange,
  valueToY,
  xToTime,
  yToValue,
} from "./timeline";
import type { TimelineState } from "./timeline.svelte";

interface Props {
  doc: EditorDocument;
  timeline: TimelineState;
  motion: Motion;
  row: Track;
  view: { t0: number; t1: number; width: number };
  onmessage: (message: string | null) => void;
}

let { doc, timeline, motion, row, view, onmessage }: Props = $props();

const HEIGHT = 110;
const PAD = 8;

type Drag =
  | { kind: "key"; t: number; point: Point }
  | { kind: "c1" | "c2"; t: number; point: Point };
let drag = $state<Drag | null>(null);

const key = $derived(targetKey(row.target));
const selectedT = $derived(timeline.selectedKey?.track === key ? timeline.selectedKey.t : null);
const selectedPoint = $derived(
  row.curve && selectedT !== null
    ? (curveKeys(row.curve).find((k) => k.t === selectedT) ?? null)
    : null,
);
const incoming = $derived(
  row.curve && selectedT !== null ? segmentEndingAt(row.curve, selectedT) : null,
);

/** ドラッグ中はその変更を当てたカーブを描く (離すまでドキュメントは変えない)。 */
const shown = $derived.by(() => (row.curve ? preview(row.curve, drag) : null));
const range = $derived(valueRange(shown, row.parameter));
const graph = $derived<GraphView>({
  t0: view.t0,
  t1: view.t1,
  v0: range[0],
  v1: range[1],
  width: view.width,
  height: HEIGHT - 2 * PAD,
});

function preview(curve: Curve, d: Drag | null): Curve {
  if (!d) return curve;
  const segments = curve.segments.map((s): Segment => {
    if (endPoint(s).t !== d.t) return s;
    if (d.kind === "key") return s.type === "start" ? { ...s, p: d.point } : { ...s, to: d.point };
    if (s.type !== "bezier") return s;
    return d.kind === "c1" ? { ...s, c1: d.point } : { ...s, c2: d.point };
  });
  return { ...curve, segments };
}

function x(t: number): number {
  return timeToX(t, graph);
}

function y(v: number): number {
  return PAD + valueToY(v, graph);
}

function pointAt(e: PointerEvent, svg: SVGSVGElement): Point {
  const rect = svg.getBoundingClientRect();
  return {
    t: xToTime(e.clientX - rect.left, graph),
    v: yToValue(e.clientY - rect.top - PAD, graph),
  };
}

async function apply(command: Command): Promise<boolean> {
  onmessage(null);
  await doc.apply(command);
  return !doc.error;
}

function round(v: number): number {
  return Math.round(v * 1000) / 1000;
}

// --- ドラッグ ---

function onDown(e: PointerEvent, kind: Drag["kind"], t: number, point: Point) {
  e.stopPropagation();
  timeline.stop();
  timeline.selectKey(row.target, t);
  if (kind === "key") timeline.seek(t);
  (e.currentTarget as SVGElement).setPointerCapture(e.pointerId);
  drag = { kind, t, point };
}

function onMove(e: PointerEvent) {
  const d = drag;
  const curve = row.curve;
  const svg = (e.currentTarget as SVGElement).ownerSVGElement;
  if (!d || !curve || !svg) return;
  const p = pointAt(e, svg);
  if (d.kind === "key") {
    const t = e.shiftKey ? d.t : snapToFrame(p.t);
    drag = { ...d, point: { t: canMoveKey(curve, d.t, t) ? t : d.t, v: round(p.v) } };
    return;
  }
  const ends = segmentEnds(curve, d.t);
  if (!ends) return;
  drag = { ...d, point: clampHandle({ t: p.t, v: round(p.v) }, ends[0], ends[1]) };
}

async function onUp() {
  const d = drag;
  drag = null;
  if (!d) return;
  if (d.kind === "key") {
    const original = curveKeys(row.curve as Curve).find((k) => k.t === d.t);
    if (!original || (original.t === d.point.t && original.v === d.point.v)) return;
    if (
      await apply({
        op: "move_motion_key",
        motion: motion.name,
        target: row.target,
        t: d.t,
        to: d.point,
      })
    ) {
      timeline.selectKey(row.target, d.point.t);
      timeline.seek(d.point.t);
    }
    return;
  }
  await apply({
    op: "set_motion_segment",
    motion: motion.name,
    target: row.target,
    t: d.t,
    segment: { type: "bezier", [d.kind]: d.point },
  });
}

async function onDoubleClick(e: MouseEvent) {
  const svg = e.currentTarget as SVGSVGElement;
  const rect = svg.getBoundingClientRect();
  const t = Math.max(0, snapToFrame(xToTime(e.clientX - rect.left, graph)));
  const v = round(yToValue(e.clientY - rect.top - PAD, graph));
  if (await apply(keyCommand(motion.name, row.target, t, v))) {
    timeline.selectKey(row.target, t);
    timeline.seek(t);
  }
}

// --- 左の欄 ---

async function setKeyField(field: "t" | "v", e: Event) {
  const input = e.currentTarget as HTMLInputElement;
  const value = input.valueAsNumber;
  const p = selectedPoint;
  if (!p || !row.curve || !Number.isFinite(value)) return;
  const to = field === "t" ? { t: value, v: p.v } : { t: p.t, v: value };
  if (field === "t" && !canMoveKey(row.curve, p.t, value)) {
    onmessage("A key cannot be moved past its neighbor.");
    input.value = String(p.t);
    return;
  }
  if (await apply({ op: "move_motion_key", motion: motion.name, target: row.target, t: p.t, to })) {
    timeline.selectKey(row.target, to.t);
    timeline.seek(to.t);
  } else {
    input.value = String(p[field]);
  }
}

async function setKind(kind: SegmentKind) {
  if (selectedT === null) return;
  await apply({
    op: "set_motion_segment",
    motion: motion.name,
    target: row.target,
    t: selectedT,
    segment: segmentShape(kind),
  });
}

async function setFade(field: "fade_in" | "fade_out", e: Event) {
  const curve = row.curve;
  if (!curve) return;
  const input = e.currentTarget as HTMLInputElement;
  const value = input.value.trim() === "" ? null : input.valueAsNumber;
  if (value !== null && !Number.isFinite(value)) return;
  const fades = {
    fade_in: curve.fade_in ?? null,
    fade_out: curve.fade_out ?? null,
    [field]: value,
  };
  await apply({ op: "set_motion_curve_fade", motion: motion.name, target: row.target, ...fades });
}

async function removeCurve() {
  if (await apply({ op: "remove_motion_curve", motion: motion.name, target: row.target })) {
    timeline.selectedKey = null;
  }
}
</script>

<div class="graph">
  <div class="side">
    <strong class="title" title={row.label}>{row.label}</strong>
    {#if selectedPoint}
      <span class="fields">
        <label class="inline"
          >t <input
            type="number"
            step="any"
            value={selectedPoint.t}
            aria-label="Key time"
            onchange={(e) => setKeyField("t", e)}
          /></label
        >
        <label class="inline"
          >v <input
            type="number"
            step="any"
            value={selectedPoint.v}
            aria-label="Key value"
            onchange={(e) => setKeyField("v", e)}
          /></label
        >
      </span>
      <span class="kinds" role="group" aria-label="Segment into this key">
        {#each SEGMENT_KINDS as k (k.kind)}
          <button
            class:on={incoming?.type === k.kind}
            disabled={!incoming}
            onclick={() => setKind(k.kind)}
            title={incoming ? `Make the segment into this key ${k.description}` : "The first key has no segment before it"}
            >{k.label}</button
          >
        {/each}
      </span>
    {:else if row.curve}
      <span class="muted">Click a key to edit it. Double-click the graph to add one.</span>
    {:else}
      <span class="muted">No curve yet. Press ◆ or double-click the graph to key.</span>
    {/if}
    {#if row.curve}
      <span class="fields">
        <label class="inline" title="Fade of this curve (empty: the motion's fade)"
          >In <input
            type="number"
            min="0"
            step="0.1"
            placeholder="—"
            value={row.curve.fade_in ?? ""}
            onchange={(e) => setFade("fade_in", e)}
          /></label
        >
        <label class="inline" title="Fade of this curve (empty: the motion's fade)"
          >Out <input
            type="number"
            min="0"
            step="0.1"
            placeholder="—"
            value={row.curve.fade_out ?? ""}
            onchange={(e) => setFade("fade_out", e)}
          /></label
        >
        <button onclick={removeCurve} title="Delete this curve and all its keys">Delete curve</button>
      </span>
    {/if}
  </div>
  <svg
    class="plot"
    width={view.width}
    height={HEIGHT}
    role="application"
    aria-label="Curve of {row.label}"
    ondblclick={onDoubleClick}
  >
    <line class="zero" x1="0" x2={view.width} y1={y(row.parameter?.default ?? 0)} y2={y(row.parameter?.default ?? 0)} />
    <line class="bound" x1={x(0)} x2={x(0)} y1="0" y2={HEIGHT} />
    <line class="bound" x1={x(motion.duration)} x2={x(motion.duration)} y1="0" y2={HEIGHT} />
    <line class="playhead" x1={x(timeline.time)} x2={x(timeline.time)} y1="0" y2={HEIGHT} />
    {#if shown}
      <path class="curve" d={curvePath(shown, { ...graph, height: graph.height })} transform="translate(0 {PAD})" />
      {#each shown.segments as s, i (i)}
        {#if s.type === "bezier"}
          {@const p0 = endPoint(shown.segments[i - 1] as Segment)}
          <line class="arm" x1={x(p0.t)} y1={y(p0.v)} x2={x(s.c1.t)} y2={y(s.c1.v)} />
          <line class="arm" x1={x(s.to.t)} y1={y(s.to.v)} x2={x(s.c2.t)} y2={y(s.c2.v)} />
          {#each [["c1", s.c1], ["c2", s.c2]] as const as [kind, c] (kind)}
            <rect
              class="handle"
              x={x(c.t) - 4}
              y={y(c.v) - 4}
              width="8"
              height="8"
              role="button"
              tabindex="-1"
              aria-label="{kind === 'c1' ? 'Start' : 'End'} handle of the segment into {s.to.t}"
              onpointerdown={(e) => onDown(e, kind, s.to.t, c)}
              onpointermove={onMove}
              onpointerup={onUp}
              onpointercancel={() => (drag = null)}
            />
          {/each}
        {/if}
      {/each}
      {#each curveKeys(shown) as k, i (i)}
        {@const original = row.curve ? curveKeys(row.curve)[i] : k}
        <circle
          class="point"
          class:selected={original?.t === selectedT}
          cx={x(k.t)}
          cy={y(k.v)}
          r="5"
          role="button"
          tabindex="-1"
          aria-label="Key at {k.t}: {k.v}"
          onpointerdown={(e) => original && onDown(e, "key", original.t, original)}
          onpointermove={onMove}
          onpointerup={onUp}
          onpointercancel={() => (drag = null)}
        />
      {/each}
    {/if}
  </svg>
</div>

<style>
  .graph {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr);
    border-top: 1px solid var(--border);
    flex: none;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px 8px;
    font-size: 12px;
    min-width: 0;
    overflow: hidden;
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fields {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  input[type="number"] {
    width: 56px;
  }
  .kinds {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }
  .kinds button {
    font-size: 11px;
    padding: 1px 4px;
  }
  .kinds button.on {
    background: var(--accent);
    color: white;
  }
  .plot {
    display: block;
    background: var(--viewport-bg);
    touch-action: none;
  }
  .zero {
    stroke: var(--border);
    stroke-dasharray: 3 3;
  }
  .bound {
    stroke: var(--border);
  }
  .playhead {
    stroke: #dc2626;
  }
  .curve {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
  }
  .arm {
    stroke: var(--muted);
  }
  .handle {
    fill: var(--panel);
    stroke: var(--muted);
    cursor: move;
  }
  .point {
    fill: var(--panel);
    stroke: var(--text);
    stroke-width: 1.5;
    cursor: grab;
  }
  .point.selected {
    fill: var(--accent);
    stroke: var(--accent);
  }
</style>
