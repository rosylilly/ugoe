<!--
  はみ出しの表示 (docs/editor.md#はみ出しの検証): 今のパラメータ値で親のワープの格子 ([0,1]²) の外にある
  頂点・制御点・回転の原点に赤い印を重ねる。「Verify」のパネルの「Show overflows in viewport」
  (VerifyView.showOverflows) で出す。操作は受け付けない (下のツールに通す)。

  どの点がはみ出しているかはバックエンド (find_overflows) に問い合わせ、位置は wasm で評価した今の位置を読む。
  問い合わせは同時に 1 つだけにし、その間に変わった値は終わってから最新のものだけを問い合わせる。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import { errorText } from "../../../errors";
import { ipc, type KeyformTarget } from "../../../ipc";
import { overflowDots } from "../../../panels/verify/overflow";
import type { VerifyView } from "../../../panels/verify/verifyView.svelte";
import type { ViewportController } from "../../controller.svelte";
import type { ViewportScene } from "../scene.svelte";

interface Props {
  controller: ViewportController;
  doc: EditorDocument;
  scene: ViewportScene;
  view: VerifyView;
}

let { controller, doc, scene, view }: Props = $props();

/** WebPuppet が新しいモデル・パラメータを反映した後に読み直すための印。 */
let tick = $state(0);
$effect(() => {
  void doc.model;
  void controller.transform;
  void controller.status;
  void controller.animationFrame;
  for (const value of Object.values(controller.parameterValues)) void value;
  queueMicrotask(() => (tick += 1));
});

let busy = false;
let queued: Record<string, number> | null = null;

async function request(parameters: Record<string, number>): Promise<void> {
  if (busy) {
    queued = parameters;
    return;
  }
  busy = true;
  try {
    const found = await ipc.findOverflows(parameters);
    if (view.showOverflows) view.current = found;
    view.error = null;
  } catch (e) {
    view.current = [];
    view.error = errorText(e);
  } finally {
    busy = false;
    const next = queued;
    queued = null;
    if (next) void request(next);
  }
}

// モデルかパラメータの値が変わったら問い合わせ直す。
$effect(() => {
  if (!view.showOverflows) {
    view.current = [];
    return;
  }
  void doc.model;
  void request({ ...controller.parameterValues });
});

const model = $derived(doc.parsedModel);

/** 要素の点の位置 (CSS ピクセル、[x0, y0, ...])。 */
function positions(target: KeyformTarget): ArrayLike<number> | null {
  if (target.kind === "mesh") return scene.screenPositions(target.id);
  const puppet = controller.puppet;
  const t = controller.transform;
  if (target.kind !== "deformer" || !puppet || !t || controller.status !== "ready") return null;
  const flat = puppet.warp_points(target.id) ?? puppet.rotation_frame(target.id)?.slice(0, 2);
  if (!flat) return null;
  return Array.from(flat, (v, k) => (k % 2 === 0 ? t.offsetX : t.offsetY) + t.scale * v);
}

const dots = $derived.by(() => {
  void tick;
  return view.showOverflows ? overflowDots(view.current, positions, model) : [];
});
</script>

{#if dots.length > 0}
  <svg class="layer" role="presentation">
    {#each dots as dot (dot.key)}
      <circle cx={dot.x} cy={dot.y} r="6"><title>{dot.title}</title></circle>
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

/* ツールの点を隠さないように、塗りは薄くし、輪を太くする。 */
circle {
  fill: rgb(239 68 68 / 0.25);
  stroke: #ef4444;
  stroke-width: 2.5;
}
</style>
