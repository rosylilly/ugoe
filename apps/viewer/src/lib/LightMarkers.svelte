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
    寄せた平行光源の円は、線を点線にし、中心を白抜きにして、キャンバスの中央の向きに矢印を出す (中央を示していないことが分かるように。T0717)。
  - 点光源の範囲の円の縁が画面に見えない (円が画面を覆う・まるごと画面の外) ときは、画面の端に小さな範囲の取っ手を出し、
    そのドラッグで範囲を変える (lighting.ts の `rangeGrip`。T0717)。取っ手は縁がいちばん画面に近い向き (覆うときは
    光源からいちばん遠い角、外にあるときはいちばん近い点) に置く (T0718)。
  - 取っ手にフォーカスがあるときも `F` は「全体を表示」(アプリのショートカット) のまま。取っ手は文字のキーを使わず、
    フィットしても取っ手のフォーカスは残って印は新しい位置に描き直されるので、横取りしない (T0717)。
  印の形はエディタの光源の取っ手 (apps/editor/src/lib/viewport/tools/lighting/LightingOverlay.svelte) に合わせる。
-->
<script lang="ts">
import { formatNumber } from "@ugoe/i18n";
import {
  anglesFromScreen,
  directionalHandle,
  type LightDial,
  lightDial,
  type Player,
  pointFromScreen,
  pointHandle,
  pointHandleOnScreen,
  pointRangeOnCanvas,
  type RangeGrip,
  rangeFromScreenRadius,
  rangeGrip,
  rangeRadiusFromGrip,
  stepMarker,
  wheelPointZ,
} from "@ugoe/player";
import { t } from "./i18n";

let { player }: { player: Player } = $props();

let svg = $state<SVGSVGElement>();
type DragKind = "directional" | "point" | "range" | "grip";
/** 範囲の取っ手 (`grip`) を掴んだときの取っ手・半径・ポインタの位置 (動かした分だけ半径を変える)。 */
type GripStart = { grip: RangeGrip; radius: number; at: [number, number] };
let drag = $state<{ pointer: number; kind: DragKind; grip?: GripStart } | null>(null);
/** 範囲の取っ手を動かしている間のポインタの位置 (取っ手を指に付いてこさせる)。 */
let gripAt = $state<[number, number] | null>(null);
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
// 範囲の円の縁が画面に見えないときの取っ手。ドラッグの間は、縁が画面に入っても消さずに掴んだ取っ手を動かす。
let grip = $derived.by((): RangeGrip | null => {
  const start = drag?.kind === "grip" ? drag.grip : undefined;
  if (start !== undefined) {
    const [dx, dy] = start.grip.dir;
    const along = gripAt ? (gripAt[0] - start.at[0]) * dx + (gripAt[1] - start.at[1]) * dy : 0;
    return { ...start.grip, x: start.grip.x + dx * along, y: start.grip.y + dy * along };
  }
  if (point === null || pointShown === null || pointRange <= 0) return null;
  const g = rangeGrip(point, pointRange, bounds);
  // 画面の端に寄せた点光源の取っ手と重なるなら出さない (光源を画面に戻せば縁か取っ手が出る)。
  if (g === null || Math.hypot(g.x - pointShown.x, g.y - pointShown.y) < 24) return null;
  return g;
});

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
  if (kind === "grip") {
    if (grip === null) return;
    const at = local(e);
    drag = { pointer: e.pointerId, kind, grip: { grip, radius: pointRange, at } };
    gripAt = at;
  } else drag = { pointer: e.pointerId, kind };
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
  } else if (drag.kind === "range" || drag.kind === "grip") {
    if (point === null) return;
    let radius = Math.hypot(x - point[0], y - point[1]);
    if (drag.grip !== undefined) {
      gripAt = [x, y];
      radius = rangeRadiusFromGrip(drag.grip.radius, drag.grip.grip.dir, drag.grip.at, [x, y]);
    }
    const range = rangeFromScreenRadius(canvas, view, s.point, radius);
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
  gripAt = null;
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

/** 寄せた平行光源の円の中心から、キャンバスの中央の向きへの矢印 (長さ 22px)。 */
function toCenterArrow(d: LightDial): string {
  const dx = d.canvasCenter[0] - d.cx;
  const dy = d.canvasCenter[1] - d.cy;
  const length = Math.hypot(dx, dy) || 1;
  const [ux, uy] = [dx / length, dy / length];
  const [x0, y0] = [d.cx + ux * 6, d.cy + uy * 6];
  const [x1, y1] = [d.cx + ux * 28, d.cy + uy * 28];
  // 矢じり: 先端から後ろへ 7px、左右に 4px。
  const [bx, by] = [x1 - ux * 7, y1 - uy * 7];
  return `M${x0},${y0}L${x1},${y1}M${bx - uy * 4},${by + ux * 4}L${x1},${y1}L${bx + uy * 4},${by - ux * 4}`;
}

/** 範囲の取っ手の形 (取っ手の向きに沿った両向きの矢印)。 */
function gripArrows(g: RangeGrip): string {
  const [ux, uy] = g.dir;
  const tip = (sign: number) => [g.x + sign * ux * 12, g.y + sign * uy * 12] as const;
  const head = (sign: number) => {
    const [tx, ty] = tip(sign);
    const [bx, by] = [tx - sign * ux * 5, ty - sign * uy * 5];
    return `M${bx - uy * 4},${by + ux * 4}L${tx},${ty}L${bx + uy * 4},${by - ux * 4}`;
  };
  const [ax, ay] = tip(-1);
  const [cx, cy] = tip(1);
  return `M${ax},${ay}L${cx},${cy}${head(-1)}${head(1)}`;
}

const pct = (v: number) => formatNumber(v, { style: "percent", maximumFractionDigits: 0 });

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
    <circle class="dial" class:shifted={dial.shifted} cx={dial.cx} cy={dial.cy} r={dial.r} />
    <circle class="center" class:shifted={dial.shifted} cx={dial.cx} cy={dial.cy} r="3" />
    {#if dial.shifted}
      <!-- 寄せた円: 中心はキャンバスの中央ではない。中心から本当の中央の向きに矢印を出す。 -->
      <path class="to-center" d={toCenterArrow(dial)} data-light="to-center">
        <title>{t("markers.toCenter.tooltip")}</title>
      </path>
    {/if}
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
      aria-label={t("markers.directional.aria")}
      aria-valuenow={s.directional.azimuth}
      aria-valuemin={-180}
      aria-valuemax={180}
      aria-valuetext={t("markers.directional.value", {
        azimuth: Math.round(s.directional.azimuth),
        elevation: Math.round(s.directional.elevation),
      })}
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
      <title>{t("markers.directional.tooltip")}</title>
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
        <title>{t("markers.range.tooltip")}</title>
      </circle>
    {/if}
    {#if grip}
      <!-- 範囲の円の縁が画面に見えないときの、範囲の取っ手 (キーボードは点光源の取っ手の + / −)。 -->
      <g
        class="range-grip"
        class:dragging={drag?.kind === "grip"}
        data-light="range-grip"
        role="presentation"
        onpointerdown={(e) => onDown(e, "grip")}
        onpointermove={onMove}
        onpointerup={onUp}
        onpointercancel={onUp}
        ondblclick={(e) => e.stopPropagation()}
      >
        <circle class="grip-hit" cx={grip.x} cy={grip.y} r="12" />
        <path class="grip-arrows" d={gripArrows(grip)} />
        <title>{t("markers.rangeGrip.tooltip")}</title>
      </g>
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
      aria-label={t("markers.point.aria")}
      aria-valuenow={s.point.x}
      aria-valuemin={-0.5}
      aria-valuemax={1.5}
      aria-valuetext={t("markers.point.value", {
        x: pct(s.point.x),
        y: pct(s.point.y),
        z: pct(s.point.z),
        range: pct(s.point.range),
      })}
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
        >{pointShown.offscreen ? t("markers.point.tooltipOffscreen") : t("markers.point.tooltip")}</title
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
    stroke: var(--ugoe-color-accent);
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
    stroke: var(--ugoe-color-accent);
    stroke-width: 2;
  }
  .dial.shifted {
    stroke-dasharray: 1 4;
    stroke-linecap: round;
    stroke-width: 2;
    opacity: 0.6;
  }
  .center.shifted {
    fill: none;
    stroke: #facc15;
    stroke-width: 1.5;
  }
  .to-center {
    fill: none;
    stroke: #facc15;
    stroke-width: 1.5;
    stroke-dasharray: 3 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0.8;
  }
  .range-grip {
    pointer-events: auto;
    cursor: move;
  }
  .grip-hit {
    fill: rgb(31 41 55 / 0.6);
    stroke: #facc15;
    stroke-dasharray: 3 2;
  }
  .grip-arrows {
    fill: none;
    stroke: #facc15;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .range-grip.dragging .grip-hit {
    stroke: var(--ugoe-color-accent);
    stroke-dasharray: none;
    stroke-width: 2;
  }
  .handle.dragging {
    stroke: var(--ugoe-color-accent);
    stroke-width: 3;
    cursor: grabbing;
  }
</style>
