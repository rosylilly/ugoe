<!--
  グルーの頂点の組の表示と編集 (docs/editor.md#グルーの画面、T0661)。「Glue」タブで選んだグルーについて、両端の
  メッシュの輪郭・組の線 (A の頂点と B の頂点を結ぶ)・組んだ頂点を重ねる。位置は評価後 (グルーを掛けた後) なので、
  強さ 1 の組の頂点は重なって見える。

  - 「Add pairs」: 両端のメッシュの頂点をすべて出す。A と B の頂点を 1 つずつ (どちらからでも) 押すと組を足す。
    B の頂点は 1 つの A の頂点とだけ組むので、同じ B の組があれば置き換える (glues.ts の withPair)。
  - 「Remove pairs」: 組の線の中点の四角を押すと、その組を外す (withoutPair)。
  - どちらも set_glue でグルーを丸ごと置き換える (Undo の 1 単位)。Esc でモードを抜ける。
  - 本体のグルー・描画用の複製のグルーは読み取り専用 (重ねて見せるだけ)。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import { t } from "../../../i18n";
import type { Glue } from "../../../ipc";
import { labelOf } from "../../../labels";
import {
  isBaseGlue,
  isDrawCloneGlue,
  setGlueCommand,
  withoutPair,
  withPair,
} from "../../../panels/glues/glues";
import type { GlueView, PairMode } from "../../../panels/glues/glueView.svelte";
import TipButton from "../../../ui/TipButton.svelte";
import { boundaryEdges, type Point } from "../geometry";
import { acceptsToolKey } from "../keys";
import type { ViewportScene } from "../scene.svelte";

interface Props {
  doc: EditorDocument;
  scene: ViewportScene;
  view: GlueView;
}

let { doc, scene, view }: Props = $props();

const model = $derived(doc.parsedModel);
const glue = $derived(
  view.active && view.glue !== null ? (model?.glues.find((g) => g.id === view.glue) ?? null) : null,
);
const readOnly = $derived(glue !== null && (isBaseGlue(glue, doc.module) || isDrawCloneGlue(glue)));

// グルーを替えたら・読み取り専用ならモードを抜ける。
$effect(() => {
  void view.glue;
  if (readOnly || glue === null) view.setPairMode("off");
});
$effect(() => {
  void view.glue;
  view.pending = null;
});

function points(id: string): Point[] | null {
  const flat = scene.screenPositions(id);
  if (!flat) return null;
  const out: Point[] = [];
  for (let k = 0; k + 1 < flat.length; k += 2) out.push([flat[k] ?? 0, flat[k + 1] ?? 0]);
  return out;
}

function outline(id: string, p: Point[]): string {
  const mesh = scene.mesh(id);
  if (!mesh) return "";
  return boundaryEdges(mesh.indices)
    .map(([a, b]) => `M${p[a]?.[0]},${p[a]?.[1]}L${p[b]?.[0]},${p[b]?.[1]}`)
    .join("");
}

const drawn = $derived.by(() => {
  if (!glue) return null;
  const a = points(glue.mesh_a);
  const b = points(glue.mesh_b);
  if (!a || !b) return null;
  const pairs = glue.pairs.flatMap((pair, index) => {
    const pa = a[pair.a];
    const pb = b[pair.b];
    if (!pa || !pb) return [];
    return [{ index, pa, pb, mid: [(pa[0] + pb[0]) / 2, (pa[1] + pb[1]) / 2] as Point }];
  });
  return {
    a,
    b,
    outlineA: outline(glue.mesh_a, a),
    outlineB: outline(glue.mesh_b, b),
    pairs,
    pairedA: new Set(glue.pairs.map((p) => p.a)),
    pairedB: new Set(glue.pairs.map((p) => p.b)),
  };
});

/**
 * 前の編集を読み直した後のグルーに `change` を当てて set_glue で送る (null なら送らない)。押した時点のグルーから
 * 組み立てると、送っている間に押した組の変更が前の変更を戻してしまう (docs/editor.md#パネルからの編集)。
 */
function update(id: string, change: (current: Glue) => Glue | null): void {
  void doc.send(() => {
    const current = doc.parsedModel?.glues.find((g) => g.id === id);
    const next = current ? change(current) : null;
    return next ? setGlueCommand(next) : null;
  });
}

function onVertex(e: PointerEvent, side: "a" | "b", index: number): void {
  if (e.button !== 0 || e.altKey || !glue || view.pairMode !== "add") return;
  e.stopPropagation();
  const pending = view.pending;
  if (pending === null || pending.side === side) {
    view.pending = { side, index };
    return;
  }
  view.pending = null;
  const [a, b] = side === "a" ? [index, pending.index] : [pending.index, index];
  update(glue.id, (g) => withPair(g, a, b));
}

function onPair(e: PointerEvent, index: number): void {
  const pair = glue?.pairs[index];
  if (e.button !== 0 || e.altKey || !glue || !pair || view.pairMode !== "remove") return;
  e.stopPropagation();
  update(glue.id, (g) => withoutPair(g, pair.a, pair.b));
}

function setMode(mode: PairMode): void {
  view.setPairMode(view.pairMode === mode ? "off" : mode);
}

function onKeyDown(e: KeyboardEvent): void {
  if (e.key !== "Escape" || view.pairMode === "off" || !acceptsToolKey(e.target)) return;
  view.setPairMode("off");
}

const hint = $derived(
  readOnly
    ? t("viewport.glue.readOnly")
    : view.pairMode === "add"
      ? view.pending === null
        ? t("viewport.glue.addHint")
        : t("viewport.glue.pendingHint", {
            mesh: labelOf(
              model?.meshes,
              view.pending.side === "a" ? (glue?.mesh_b ?? "") : (glue?.mesh_a ?? ""),
            ),
          })
      : view.pairMode === "remove"
        ? t("viewport.glue.removeHint")
        : "",
);
</script>

<svelte:window onkeydown={onKeyDown} />

{#if glue}
  {#if drawn}
    <svg class="layer" role="presentation" data-glue={glue.id}>
      <path class="outline a" d={drawn.outlineA} />
      <path class="outline b" d={drawn.outlineB} />
      {#each drawn.pairs as p (p.index)}
        <line class="pair" x1={p.pa[0]} y1={p.pa[1]} x2={p.pb[0]} y2={p.pb[1]} />
      {/each}
      {#if view.pairMode === "add"}
        {#each drawn.a as p, i (i)}
          <circle
            class="vertex a"
            class:paired={drawn.pairedA.has(i)}
            class:pending={view.pending?.side === "a" && view.pending.index === i}
            cx={p[0]}
            cy={p[1]}
            r="4"
            role="presentation"
            onpointerdown={(e) => onVertex(e, "a", i)}
          />
        {/each}
        {#each drawn.b as p, i (i)}
          <circle
            class="vertex b"
            class:paired={drawn.pairedB.has(i)}
            class:pending={view.pending?.side === "b" && view.pending.index === i}
            cx={p[0]}
            cy={p[1]}
            r="4"
            role="presentation"
            onpointerdown={(e) => onVertex(e, "b", i)}
          />
        {/each}
      {:else}
        {#each drawn.pairs as p (p.index)}
          <circle class="end a" cx={p.pa[0]} cy={p.pa[1]} r="2" />
          <circle class="end b" cx={p.pb[0]} cy={p.pb[1]} r="2" />
        {/each}
      {/if}
      {#if view.pairMode === "remove"}
        {#each drawn.pairs as p (p.index)}
          <rect
            class="marker"
            x={p.mid[0] - 4}
            y={p.mid[1] - 4}
            width="8"
            height="8"
            role="presentation"
            onpointerdown={(e) => onPair(e, p.index)}
          />
        {/each}
      {/if}
    </svg>
  {/if}
  <div class="bar" role="group" aria-label={t("viewport.glue.aria")}>
    <span class="title">
      <span class="swatch a"></span>{labelOf(model?.meshes, glue.mesh_a)}
      <span class="swatch b"></span>{labelOf(model?.meshes, glue.mesh_b)}
      · {t("viewport.glue.pairs", { count: glue.pairs.length })}
    </span>
    <TipButton
      small
      pressed={view.pairMode === "add"}
      disabled={readOnly || !drawn}
      onclick={() => setMode("add")}
      tooltip={t("viewport.glue.add.tooltip")}>{t("viewport.glue.add.label")}</TipButton
    >
    <TipButton
      small
      pressed={view.pairMode === "remove"}
      disabled={readOnly || !drawn || glue.pairs.length === 0}
      onclick={() => setMode("remove")}
      tooltip={t("viewport.glue.remove.tooltip")}>{t("viewport.glue.remove.label")}</TipButton
    >
    {#if hint}<span class="hint muted"
        >{view.pairMode !== "off" ? t("viewport.glue.withEsc", { hint }) : hint}</span
      >{/if}
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

.outline {
  fill: none;
  stroke-width: 1.5;
  stroke-dasharray: 5 3;
}

.a {
  --glue: #22c55e;
}

.b {
  --glue: #a855f7;
}

.outline.a,
.outline.b {
  stroke: var(--glue);
}

.pair {
  stroke: #f59e0b;
  stroke-width: 1.5;
}

.end {
  fill: var(--glue);
}

.vertex {
  fill: var(--ugoe-color-surface);
  stroke: var(--glue);
  stroke-width: 1.5;
  pointer-events: visiblePainted;
  cursor: pointer;
}

.vertex.paired {
  fill: var(--glue);
}

.vertex.pending {
  stroke: #f59e0b;
  stroke-width: 3;
}

.marker {
  fill: #f59e0b;
  stroke: var(--ugoe-color-surface);
  stroke-width: 1;
  pointer-events: visiblePainted;
  cursor: pointer;
}

.bar {
  position: absolute;
  bottom: 8px;
  left: 8px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ugoe-space-2);
  max-width: calc(100% - 16px);
  padding: var(--ugoe-space-1) var(--ugoe-space-2);
  background: var(--ugoe-color-surface);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
  pointer-events: auto;
}

.title {
  display: inline-flex;
  align-items: center;
  gap: var(--ugoe-space-2);
  overflow-wrap: anywhere;
}

.swatch {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: var(--ugoe-radius-sm);
  background: var(--glue);
}

.hint {
  font-size: var(--ugoe-font-size-xs);
}
</style>
