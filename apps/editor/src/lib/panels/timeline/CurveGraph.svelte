<!--
  選んだ行のカーブのグラフ: キー (円) とベジェの取っ手 (四角) をドラッグで編集し、離したときに Command を 1 つ送る
  (キーは move_motion_key、取っ手は set_motion_segment。送る直前のカーブから組み立てる。docs/editor.md#パネルからの編集)。ダブルクリックでその位置にキーを打つ。
  左の欄: 選んだキーの時刻と値、そのキーに入る区間の種類 (直線・ベジェ・ステップ)、カーブ個別のフェード、カーブの削除。
  欄の確定・区間の種類・フェードも、送る直前のカーブから組み立てる。
-->
<script lang="ts">
import { formatNumber } from "@ugoe/i18n";
import { NumberField, Select } from "@ugoe/ui";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { Command, Curve, Motion, Point, Segment } from "../../ipc";
import TipButton from "../../ui/TipButton.svelte";
import {
  canMoveKey,
  clampHandle,
  curveKeys,
  curvePath,
  endPoint,
  findCurve,
  type GraphView,
  keyCommand,
  type SegmentKind,
  segmentEndingAt,
  segmentEnds,
  segmentHandle,
  segmentKinds,
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

const HEIGHT = 124;
const PAD = 8;
/** 印を押せる範囲の一辺 (CSS px。docs/ui.md の 24×24)。 */
const HIT = 24;

/** ドラッグ: 押した印 (キーの時刻 `t` と押したときの位置 `from`) と、今の位置 `point`。 */
type Drag = { kind: "key" | "c1" | "c2"; t: number; from: Point; point: Point };
let drag = $state<Drag | null>(null);
/** 適用に失敗したとき、数値の欄を今の値に戻すために作り直す (数値の欄は確定した値を出したままになるので)。 */
let fieldsKey = $state(0);
const kinds = $derived(segmentKinds());
const ids = $props.id();

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

/** 値の目盛りの文字: グラフの上端と下端の値 (既定値は破線で示す)。上は上の縁から、下は下の縁に基線を置く。 */
const axisLabels = $derived.by(() => {
  const fmt = (v: number) => formatNumber(v, { maximumFractionDigits: 2 });
  return [
    { key: "max", text: fmt(range[1]), y: 2, baseline: "hanging" },
    { key: "min", text: fmt(range[0]), y: HEIGHT - 3, baseline: "auto" },
  ];
});

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
  // 失敗の理由は doc.error (タイムラインの見出し) に出る。
  return (await doc.apply(command)) !== null;
}

function round(v: number): number {
  return Math.round(v * 1000) / 1000;
}

/** 送る直前の (前の編集を読み直した後の) このカーブ。 */
function currentCurve(): Curve | null {
  const m = doc.motions.find((x) => x.name === motion.name);
  return m ? findCurve(m, row.target) : null;
}

// --- ドラッグ ---

function onDown(e: PointerEvent, kind: Drag["kind"], t: number, point: Point) {
  e.stopPropagation();
  timeline.stop();
  timeline.selectKey(row.target, t);
  if (kind === "key") timeline.seek(t);
  (e.currentTarget as SVGElement).setPointerCapture(e.pointerId);
  drag = { kind, t, from: point, point };
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

function onUp() {
  const d = drag;
  drag = null;
  // 動かさずに離したときは送らない (Undo の履歴に何も変えないコマンドを積まない)。
  if (!d || (d.point.t === d.from.t && d.point.v === d.from.v)) return;
  const name = motion.name;
  const moves = timeline.keyMoves;
  // 組み立てたキーの移動の行き先 (送らなかったら null のまま)。
  let to: Point | null = null;
  // 離したときの表示はまだ前の編集を読み直していないことがあるので、送る直前のカーブで今の時刻のキーを引き直す。
  void moves
    .track(
      doc.send(() => {
        const curve = currentCurve();
        const key = curve ? moves.follow(name, curve, d.t) : null;
        if (!curve || !key) return null;
        if (d.kind === "key") {
          // 時刻を動かさなかった (Shift で値だけ動かした) ときは、今の時刻のまま。
          const next = { t: d.point.t === d.from.t ? key.t : d.point.t, v: d.point.v };
          if (next.t === key.t && next.v === key.v) return null;
          if (next.t !== key.t && !canMoveKey(curve, key.t, next.t)) {
            onmessage(t("timeline.keyBlocked"));
            return null;
          }
          onmessage(null);
          moves.record(name, curve, key.t, next.t);
          to = next;
          // 失敗の理由は doc.error (タイムラインの見出し) に出る。
          return { op: "move_motion_key", motion: name, target: row.target, t: key.t, to: next };
        }
        const ends = segmentEnds(curve, key.t);
        const before = segmentHandle(curve, key.t, d.kind);
        if (!ends || !before) return null;
        const point = clampHandle(d.point, ends[0], ends[1]);
        if (before.t === point.t && before.v === point.v) return null;
        onmessage(null);
        return {
          op: "set_motion_segment",
          motion: name,
          target: row.target,
          t: key.t,
          segment: { type: "bezier", [d.kind]: point },
        };
      }),
    )
    .then((report) => {
      const sent: Point | null = to;
      if (sent === null || !report) return;
      timeline.selectKey(row.target, sent.t);
      timeline.seek(sent.t);
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

function setKeyField(field: "t" | "v", value: number) {
  const from = selectedPoint?.t;
  if (from === undefined) return;
  const name = motion.name;
  const moves = timeline.keyMoves;
  // 組み立てたコマンドの行き先 (組み立てを数値の欄の次の確定に置き換えられたら null のまま)。
  let to: Point | null = null;
  // 確定したときの選択はまだ前の時刻を指していることがあるので、送った時刻の変更をたどって今の時刻のキーを動かす。
  void moves
    .track(
      doc.send(() => {
        const curve = currentCurve();
        const p = curve ? moves.follow(name, curve, from) : null;
        if (!curve || !p) return null;
        const next = field === "t" ? { t: value, v: p.v } : { t: p.t, v: value };
        if (next.t === p.t && next.v === p.v) return null;
        if (field === "t" && !canMoveKey(curve, p.t, value)) {
          onmessage(t("timeline.keyBlocked"));
          fieldsKey++;
          return null;
        }
        onmessage(null);
        moves.record(name, curve, p.t, next.t);
        to = next;
        // 失敗の理由は doc.error (タイムラインの見出し) に出る。
        return { op: "move_motion_key", motion: name, target: row.target, t: p.t, to: next };
      }),
    )
    .then((report) => {
      const sent: Point | null = to;
      if (sent === null) return;
      if (report) {
        timeline.selectKey(row.target, sent.t);
        timeline.seek(sent.t);
      } else {
        fieldsKey++;
      }
    });
}

function setKind(kind: SegmentKind) {
  const from = selectedT;
  if (from === null) return;
  const name = motion.name;
  const moves = timeline.keyMoves;
  // 押したときの選択はまだ前の時刻を指していることがあるので、送った時刻の変更をたどって今の時刻のキーで終わる区間を変える。
  void moves.track(
    doc.send(() => {
      const curve = currentCurve();
      const key = curve ? moves.follow(name, curve, from) : null;
      const segment = curve && key ? segmentEndingAt(curve, key.t) : null;
      if (!key || !segment || segment.type === kind) return null;
      onmessage(null);
      // 失敗の理由は doc.error (タイムラインの見出し) に出る。
      return {
        op: "set_motion_segment",
        motion: name,
        target: row.target,
        t: key.t,
        segment: segmentShape(kind),
      };
    }),
  );
}

function setFade(field: "fade_in" | "fade_out", value: number | null) {
  const name = motion.name;
  // もう一方のフェードは、送る直前の (前に確定したフェードを読み直した後の) カーブから取る。
  void doc.send(() => {
    const curve = currentCurve();
    if (!curve) return null;
    const fades = {
      fade_in: curve.fade_in ?? null,
      fade_out: curve.fade_out ?? null,
      [field]: value,
    };
    if (fades.fade_in === (curve.fade_in ?? null) && fades.fade_out === (curve.fade_out ?? null)) {
      return null;
    }
    onmessage(null);
    // 失敗の理由は doc.error (タイムラインの見出し) に出る。
    return { op: "set_motion_curve_fade", motion: name, target: row.target, ...fades };
  });
}

async function removeCurve() {
  if (await apply({ op: "remove_motion_curve", motion: motion.name, target: row.target })) {
    timeline.selectedKey = null;
  }
}
</script>

<div class="graph">
  <!-- 左の欄は 4 行に収める (1024×720 でもグラフと表を送らずに出すため。docs/editor.md#タイムライン) -->
  <div class="side">
    <div class="head">
      <strong class="title" title={row.label}>{row.label}</strong>
      {#if row.curve}
        <TipButton small onclick={removeCurve} tooltip={t("timeline.curve.delete.tooltip")}
          >{t("timeline.curve.delete.label")}</TipButton
        >
      {/if}
    </div>
    {#if selectedPoint}
      <span class="fields">
        {#key fieldsKey}
          <span class="ugoe-visually-hidden" id="{ids}-t">{t("timeline.curve.keyTime.aria")}</span>
          <span class="ugoe-visually-hidden" id="{ids}-v">{t("timeline.curve.keyValue.aria")}</span>
          <span class="inline"
            ><span aria-hidden="true">{t("timeline.curve.keyTime.label")}</span><NumberField
              step={0.1}
              decimals={4}
              size={5}
              value={selectedPoint.t}
              aria-labelledby="{ids}-t"
              onchange={(v) => setKeyField("t", v)}
            /></span
          >
          <span class="inline"
            ><span aria-hidden="true">{t("timeline.curve.keyValue.label")}</span><NumberField
              step={0.1}
              decimals={4}
              size={5}
              value={selectedPoint.v}
              aria-labelledby="{ids}-v"
              onchange={(v) => setKeyField("v", v)}
            /></span
          >
        {/key}
      </span>
      {#if incoming && incoming.type !== "start"}
        <Select
          aria-label={t("timeline.curve.segment.aria")}
          value={incoming.type}
          options={kinds.map((k) => ({ value: k.kind, label: k.label }))}
          tooltip={kinds.find((k) => k.kind === incoming.type)?.description}
          onchange={(v) => setKind(v as SegmentKind)}
        />
      {:else}
        <span class="muted hint">{t("timeline.curve.firstKey")}</span>
      {/if}
    {:else if row.curve}
      <span class="muted hint">{t("timeline.curve.hintKey")}</span>
    {:else}
      <span class="muted hint">{t("timeline.curve.hintEmpty")}</span>
    {/if}
    {#if row.curve}
      <span class="fields">
        <span class="inline">
          <span id="{ids}-fade_in">{t("timeline.curve.fadeIn.label")}</span>
          <!-- 空はモーションのフェードを使う (空の欄にモーションのフェードを薄く出す) -->
          <NumberField
            optional
            min={0}
            step={0.1}
            size={4}
            value={row.curve?.fade_in ?? null}
            emptyValue={motion.fade_in}
            aria-labelledby="{ids}-fade_in"
            tooltip={t("timeline.curve.fade.tooltip")}
            onchange={(v) => setFade("fade_in", v)}
          />
        </span>
        <span class="inline">
          <span id="{ids}-fade_out">{t("timeline.curve.fadeOut.label")}</span>
          <!-- 空はモーションのフェードを使う (空の欄にモーションのフェードを薄く出す) -->
          <NumberField
            optional
            min={0}
            step={0.1}
            size={4}
            value={row.curve?.fade_out ?? null}
            emptyValue={motion.fade_out}
            aria-labelledby="{ids}-fade_out"
            tooltip={t("timeline.curve.fade.tooltip")}
            onchange={(v) => setFade("fade_out", v)}
          />
        </span>
      </span>
    {/if}
  </div>
  <svg
    class="plot"
    width={view.width}
    height={HEIGHT}
    role="application"
    aria-label={t("timeline.curve.plot.aria", { label: row.label })}
    ondblclick={onDoubleClick}
  >
    <line class="zero" x1="0" x2={view.width} y1={y(row.parameter?.default ?? 0)} y2={y(row.parameter?.default ?? 0)} />
    <line class="bound" x1={x(0)} x2={x(0)} y1="0" y2={HEIGHT} />
    <line class="bound" x1={x(motion.duration)} x2={x(motion.duration)} y1="0" y2={HEIGHT} />
    <line class="playhead" x1={x(timeline.time)} x2={x(timeline.time)} y1="0" y2={HEIGHT} />
    <!-- 値の目盛り: 上端と下端の値 (グラフの左端に) -->
    {#each axisLabels as label (label.key)}
      <text class="axis" x="4" y={label.y} dominant-baseline={label.baseline}>{label.text}</text>
    {/each}
    {#if shown}
      <path class="curve" d={curvePath(shown, { ...graph, height: graph.height })} transform="translate(0 {PAD})" />
      {#each shown.segments as s, i (i)}
        {#if s.type === "bezier"}
          {@const p0 = endPoint(shown.segments[i - 1] as Segment)}
          <line class="arm" x1={x(p0.t)} y1={y(p0.v)} x2={x(s.c1.t)} y2={y(s.c1.v)} />
          <line class="arm" x1={x(s.to.t)} y1={y(s.to.v)} x2={x(s.c2.t)} y2={y(s.c2.v)} />
          {#each [["c1", s.c1], ["c2", s.c2]] as const as [kind, c] (kind)}
            <rect class="handle" x={x(c.t) - 4} y={y(c.v) - 4} width="8" height="8" />
            <!-- 押せる範囲は 24×24 (見た目の印より広い。docs/ui.md#そのほか。T0907) -->
            <rect
              class="hit move"
              x={x(c.t) - HIT / 2}
              y={y(c.v) - HIT / 2}
              width={HIT}
              height={HIT}
              role="button"
              tabindex="-1"
              aria-label={kind === "c1"
                ? t("timeline.curve.handleStart.aria", { time: s.to.t })
                : t("timeline.curve.handleEnd.aria", { time: s.to.t })}
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
        <circle class="point" class:selected={original?.t === selectedT} cx={x(k.t)} cy={y(k.v)} r="5" />
        <rect
          class="hit grab"
          x={x(k.t) - HIT / 2}
          y={y(k.v) - HIT / 2}
          width={HIT}
          height={HIT}
          role="button"
          tabindex="-1"
          aria-label={t("timeline.curve.point.aria", { time: k.t, value: k.v })}
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
    grid-template-columns: var(--label-width, 200px) minmax(0, 1fr);
    border-top: 1px solid var(--ugoe-color-border);
    flex: none;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
    padding: var(--ugoe-space-2) var(--ugoe-space-3);
    font-size: var(--ugoe-font-size-sm);
    min-width: 0;
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--ugoe-space-2);
    min-width: 0;
  }
  .hint {
    font-size: var(--ugoe-font-size-xs);
  }
  /* 線や点に重なっても読めるよう、グラフの地の色で縁取る */
  .axis {
    fill: var(--ugoe-color-text-muted);
    stroke: var(--ugoe-color-sunken);
    stroke-width: 3px;
    paint-order: stroke;
    font-size: var(--ugoe-font-size-xs);
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }
  .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fields {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-2);
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: var(--ugoe-space-1);
  }
  .plot {
    display: block;
    background: var(--ugoe-color-sunken);
    touch-action: none;
  }
  .zero {
    stroke: var(--ugoe-color-border);
    stroke-dasharray: 3 3;
  }
  .bound {
    stroke: var(--ugoe-color-border);
  }
  .playhead {
    stroke: var(--ugoe-color-danger);
  }
  .curve {
    fill: none;
    stroke: var(--ugoe-color-accent);
    stroke-width: 2;
  }
  .arm {
    stroke: var(--ugoe-color-text-muted);
  }
  .handle {
    fill: var(--ugoe-color-surface);
    stroke: var(--ugoe-color-text-muted);
    pointer-events: none;
  }
  .point {
    fill: var(--ugoe-color-surface);
    stroke: var(--ugoe-color-text);
    stroke-width: 1.5;
    pointer-events: none;
  }
  .hit {
    fill: transparent;
  }
  .hit.move {
    cursor: move;
  }
  .hit.grab {
    cursor: grab;
  }
  .point.selected {
    fill: var(--ugoe-color-accent);
    stroke: var(--ugoe-color-accent);
  }
</style>
