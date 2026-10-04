<!--
  深度の表示 (docs/editor.md#深度の編集): 主な選択のメッシュの頂点・ワープの制御点に、深度の色の点を重ねる
  (手前 (正) ほど赤、奥 (負) ほど青、0 は灰色。尺度は深度の絶対値の最大)。ツールの点を隠さないように
  点の周りの輪で描き、ツールの上に重ねる。インスペクタの深度の節の
  「Show in viewport」(ViewportController.showDepth) で出す。操作は受け付けない (下のツールに通す)。
  位置は wasm で評価した今のパラメータ値での位置。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import { depthColor, depthElement, depthScale } from "../../../panels/inspector/depth/depth";
import type { Selection } from "../../../selection.svelte";
import type { ViewportController } from "../../controller.svelte";
import type { ViewportScene } from "../scene.svelte";

interface Props {
  controller: ViewportController;
  doc: EditorDocument;
  selection: Selection;
  scene: ViewportScene;
}

let { controller, doc, selection, scene }: Props = $props();

/** WebPuppet が新しいモデル・パラメータを反映した後に読み直すための印。 */
let tick = $state(0);
$effect(() => {
  void doc.model;
  void controller.transform;
  void controller.status;
  for (const value of Object.values(controller.parameterValues)) void value;
  queueMicrotask(() => (tick += 1));
});

const element = $derived.by(() => {
  if (!controller.showDepth || !doc.parsedModel) return null;
  return depthElement(doc.parsedModel, selection.primary);
});

/** 点の位置 (CSS ピクセル、[x0, y0, ...])。 */
const positions = $derived.by((): number[] | null => {
  void tick;
  if (!element) return null;
  if (element.kind === "mesh") return scene.screenPositions(element.target.id);
  const puppet = controller.puppet;
  const t = controller.transform;
  if (!puppet || !t || controller.status !== "ready") return null;
  const flat = puppet.warp_points(element.target.id);
  if (!flat || flat.length !== element.points * 2) return null;
  return Array.from(flat, (v, k) => (k % 2 === 0 ? t.offsetX : t.offsetY) + t.scale * v);
});

const dots = $derived.by(() => {
  if (!element || !positions) return [];
  const depth = element.depth ?? new Array<number>(element.points).fill(0);
  const scale = depthScale(depth);
  return depth.map((d, i) => ({
    x: positions[2 * i] ?? 0,
    y: positions[2 * i + 1] ?? 0,
    color: depthColor(d, scale),
    title: `#${i}: ${d}`,
  }));
});
</script>

{#if dots.length > 0}
  <svg class="layer" role="presentation">
    {#each dots as dot, i (i)}
      <circle cx={dot.x} cy={dot.y} r="7" stroke={dot.color}><title>{dot.title}</title></circle>
    {/each}
  </svg>
{/if}

<style>
.layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

/* ツールの点 (頂点・制御点) を隠さないように、点の周りの輪で示す。 */
circle {
  fill: none;
  stroke-width: 3;
}
</style>
