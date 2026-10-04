<!--
  プレビューのビューポートで、着せ替えの違いの要素を強調する (T0657。docs/editor.md#プレビューの着せ替え)。
  DressUp.highlight の要素を、いまビューポートに出ている版 (今の編集かファイル。shownVersion) の WebPuppet から描く:

  - メッシュの輪郭: 要素の `meshes` (パーツ・デフォーマはその下、グルーは両端、物理のリグは出力先)。作業用のモデルに
    同じ頂点数のメッシュがあればその三角形の外周、無ければ (ファイルだけのメッシュなど) 凸包。
  - デフォーマ: ワープの制御点、回転の原点と軸。
  - 物理のリグ: 粒子の鎖 (動かしていれば今の位置、でなければ初期位置)。衝突体: 円・カプセル。

  - ポーズのグループと追従: 要素とリンクのパーツの下のメッシュの輪郭 (T0659)。
  - 描いている版に無い要素 (今の編集で除いたもの・ファイルに無い足したもの) は、残っているメッシュを破線で描き、
    知らせに「Removed」「Added」と出す (T0661)。

  操作は受けない。下に何を強調しているか (描けなければ、どちらを付ければ見えるか) を出す。知らせの「Fit」で
  ビューポートを描いた要素に寄せる (fitMove。全体に戻すのはビューポートの「Fit」。T0659)。
-->
<script lang="ts">
import type { EditorDocument } from "../document.svelte";
import { t } from "../i18n";
import type { PhysicsSettings } from "../ipc";
import { isDynamicRig } from "../panels/physics/physics";
import TipButton from "../ui/TipButton.svelte";
import type { ViewportController } from "../viewport/controller.svelte";
import { toPoints } from "../viewport/tools/deformer/gizmo";
import { boundaryEdges, type Point } from "../viewport/tools/geometry";
import { summarizeModel } from "../viewport/tools/model";
import { puppetMap } from "../viewport/tools/physics/mapper";
import {
  capsulePath,
  restParticles,
  type WorldShape,
  worldShape,
} from "../viewport/tools/physics/physicsGeometry";
import {
  convexHull,
  fitMove,
  highlightNotice,
  missingIn,
  shownVersion,
} from "./differenceHighlight";
import type { PreviewMode } from "./preview.svelte";

interface Props {
  controller: ViewportController;
  doc: EditorDocument;
  preview: PreviewMode;
}

let { controller, doc, preview }: Props = $props();

/** WebPuppet が新しいモデル・付け外し・フレームを反映した後に読み直すための印。 */
let tick = $state(0);
$effect(() => {
  void doc.model;
  void controller.transform;
  void controller.status;
  void controller.animationFrame;
  void preview.dressUp.entries.map((e) => e.attached);
  for (const value of Object.values(preview.values)) void value;
  queueMicrotask(() => (tick += 1));
});

const highlight = $derived(preview.dressUp.highlight);
/** 作業用のモデルのメッシュ (三角形の外周を取る)。 */
const workspace = $derived(new Map(summarizeModel(doc.parsedModel).meshes.map((m) => [m.id, m])));

function screen(p: Point): Point {
  const t = controller.transform;
  return t ? [t.offsetX + t.scale * p[0], t.offsetY + t.scale * p[1]] : p;
}

function polyline(points: Point[], close = false): string {
  const d = points.map((p, k) => `${k === 0 ? "M" : "L"}${p[0]},${p[1]}`).join("");
  return close ? `${d}Z` : d;
}

interface Drawn {
  outlines: { id: string; d: string }[];
  /** 描いたものの画面の点 (寄せる範囲)。 */
  bounds: Point[];
  points: Point[];
  axes: { origin: Point; x: Point; y: Point }[];
  chain: Point[];
  shape: WorldShape | null;
  shown: ReturnType<typeof shownVersion>;
}

const drawn = $derived.by((): Drawn | null => {
  void tick;
  const puppet = controller.puppet;
  const t = controller.transform;
  if (!highlight || !puppet || !t || controller.status !== "ready" || !puppet.has_model()) {
    return null;
  }
  const shown = shownVersion(highlight.module, preview.comparing, puppet.attached_modules());
  const out: Drawn = {
    outlines: [],
    bounds: [],
    points: [],
    axes: [],
    chain: [],
    shape: null,
    shown,
  };
  if (!shown) return out;
  const item = highlight.item;
  const prefix = `${shown.name}/`;
  const ids = puppet.drawable_ids();
  for (const mesh of item.meshes) {
    const index = ids.indexOf(prefix + mesh);
    const flat = index < 0 ? undefined : puppet.drawable_positions(index);
    if (!flat) continue;
    const p = toPoints(flat).map(screen);
    const info = workspace.get(`${doc.module?.name ?? highlight.module}/${mesh}`);
    const d =
      info && info.vertices === p.length
        ? boundaryEdges(info.indices)
            .map(([a, b]) => `M${p[a]?.[0]},${p[a]?.[1]}L${p[b]?.[0]},${p[b]?.[1]}`)
            .join("")
        : polyline(convexHull(p), true);
    out.outlines.push({ id: mesh, d });
    // push(...p) は引数の数の上限で止まるので、1 つずつ足す (頂点の多いメッシュ)。
    for (const q of p) out.bounds.push(q);
  }
  if (item.kind === "deformer") {
    const id = prefix + item.id;
    const warp = puppet.warp_points(id);
    if (warp) out.points = toPoints(warp).map(screen);
    const frame = puppet.rotation_frame(id);
    if (frame && frame.length === 6) {
      const [ox, oy, ax, ay, bx, by] = Array.from(frame) as [
        number,
        number,
        number,
        number,
        number,
        number,
      ];
      const origin = screen([ox, oy]);
      // 軸は画面で 24 ピクセルの長さで描く。
      out.axes.push({
        origin,
        x: [origin[0] + 24 * ax, origin[1] + 24 * ay],
        y: [origin[0] + 24 * bx, origin[1] + 24 * by],
      });
    }
  } else if (item.kind === "physics_rig" || item.kind === "collider") {
    let settings: PhysicsSettings | null = null;
    try {
      const json = puppet.physics_json();
      settings = json ? (JSON.parse(json) as PhysicsSettings) : null;
    } catch (e) {
      console.warn("ugoe:", e);
    }
    const map = puppetMap(puppet);
    if (item.kind === "physics_rig") {
      const index = settings?.rigs.findIndex((r) => r.id === prefix + item.id) ?? -1;
      const rig = settings?.rigs[index];
      if (rig && isDynamicRig(rig)) {
        const live = toPoints(puppet.physics_particles(index));
        const points = live.length === rig.particles.length ? live : restParticles(rig, map);
        out.chain = (points ?? []).map(screen);
      }
    } else {
      const collider = settings?.colliders.find((c) => c.id === prefix + item.id);
      const w = collider ? worldShape(collider, map) : null;
      if (w) {
        out.shape =
          w.type === "circle"
            ? { ...w, center: screen(w.center), radius: w.radius * t.scale }
            : { ...w, a: screen(w.a), b: screen(w.b), radius: w.radius * t.scale };
      }
    }
  }
  out.bounds.push(...out.points, ...out.axes.map((a) => a.origin), ...out.chain);
  const shape = out.shape;
  if (shape?.type === "circle") {
    const [x, y] = shape.center;
    out.bounds.push([x - shape.radius, y - shape.radius], [x + shape.radius, y + shape.radius]);
  } else if (shape?.type === "capsule") {
    for (const [x, y] of [shape.a, shape.b]) {
      out.bounds.push([x - shape.radius, y - shape.radius], [x + shape.radius, y + shape.radius]);
    }
  }
  return out;
});

let layer = $state<SVGSVGElement>();

/** ビューポートを強調している要素に寄せる。 */
function fit(): void {
  const box = layer?.getBoundingClientRect();
  const move = drawn && box ? fitMove(drawn.bounds, box.width, box.height) : null;
  if (!move) return;
  controller.zoomAt(move.factor, move.center[0], move.center[1]);
  controller.panBy(move.pan[0], move.pan[1]);
}

const found = $derived(
  drawn !== null &&
    (drawn.outlines.length > 0 ||
      drawn.points.length > 0 ||
      drawn.axes.length > 0 ||
      drawn.chain.length > 0 ||
      drawn.shape !== null),
);
</script>

{#if highlight && drawn}
  {@const missing = missingIn(highlight.item, drawn.shown)}
  <svg class="layer" role="presentation" bind:this={layer}>
    {#each drawn.outlines as o (o.id)}
      <!-- 除いた要素の、残っているメッシュは破線にする (T0661) -->
      <path class="outline" class:missing d={o.d} />
    {/each}
    {#each drawn.points as p, k (k)}
      <circle class="point" cx={p[0]} cy={p[1]} r="3" />
    {/each}
    {#each drawn.axes as a, k (k)}
      <circle class="point" cx={a.origin[0]} cy={a.origin[1]} r="4" />
      <line class="axis" x1={a.origin[0]} y1={a.origin[1]} x2={a.x[0]} y2={a.x[1]} />
      <line class="axis" x1={a.origin[0]} y1={a.origin[1]} x2={a.y[0]} y2={a.y[1]} />
    {/each}
    {#if drawn.chain.length > 0}
      <path class="chain" d={polyline(drawn.chain)} />
      {#each drawn.chain as p, k (k)}
        <circle class="point" cx={p[0]} cy={p[1]} r="4" />
      {/each}
    {/if}
    {#if drawn.shape?.type === "circle"}
      <circle class="shape" cx={drawn.shape.center[0]} cy={drawn.shape.center[1]}
        r={Math.max(drawn.shape.radius, 2)} />
    {:else if drawn.shape?.type === "capsule"}
      <path class="shape" d={capsulePath(drawn.shape.a, drawn.shape.b, Math.max(drawn.shape.radius, 2))} />
    {/if}
  </svg>
  <div class="notice" role="status">
    <span>{highlightNotice(highlight.item, drawn.shown, found)}</span>
    <TipButton
      small
      onclick={fit}
      disabled={!found}
      tooltip={t("preview.difference.fit.tooltip", { id: highlight.item.id })}>{t("preview.difference.fit.label")}</TipButton
    >
    <TipButton small onclick={() => (preview.dressUp.highlight = null)} tooltip={t("preview.difference.clear.tooltip")}
      >{t("preview.difference.clear.label")}</TipButton
    >
  </div>
{/if}

<style>
.layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.outline,
.chain,
.shape {
  fill: none;
  stroke: var(--highlight, #ff9f1a);
  stroke-width: 2;
}

.outline.missing {
  stroke-dasharray: 6 4;
}

.shape {
  fill: color-mix(in srgb, var(--highlight, #ff9f1a) 20%, transparent);
}

.axis {
  stroke: var(--highlight, #ff9f1a);
  stroke-width: 1.5;
}

.point {
  fill: var(--highlight, #ff9f1a);
}

.notice {
  position: absolute;
  bottom: 8px;
  left: 8px;
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
  max-width: calc(100% - 16px);
  padding: var(--ugoe-space-1) var(--ugoe-space-2);
  background: var(--ugoe-color-surface);
  border: 1px solid var(--highlight, #ff9f1a);
  border-radius: var(--ugoe-radius-md);
  pointer-events: auto;
}
</style>
