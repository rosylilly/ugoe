<!--
  ビューポートの上に重ねる編集ツール (選択枠・頂点・ギズモ)。ツールは lib/viewport/tools/ に置き、ここで切り替える。
  座標は CSS ピクセル (ビューポート要素の左上原点)。評価後の頂点位置は ViewportScene が wasm から読む。

  - 選択モード: クリックで手前のメッシュを選ぶ (Shift / Ctrl で追加・解除)。Alt + クリックは重なった
    メッシュを手前から順に選ぶ。選択中のメッシュには輪郭を描く。ダブルクリックでメッシュ編集モードに入る。
  - メッシュ編集モード (T0606): tools/MeshEditTool.svelte。Esc で抜ける。
  - フォーム編集 (T0608): tools/form/FormTool.svelte。今のセルのキーフォームを編集する (メッシュ編集中は出さない)。
  - デフォーマのギズモ (T0607): tools/deformer/DeformerGizmo.svelte。主な選択がデフォーマのとき、
    フォーム編集の代わりに出す。
  - 深度の表示 (T0617): tools/depth/DepthOverlay.svelte。インスペクタの深度の節で切り替える (操作は受けない)。
  - 物理の表示 (T0629): tools/physics/PhysicsOverlay.svelte。物理のパネルを開いている間、粒子と衝突体を重ね、
    衝突体のドラッグを受ける (ほかのツールより上)。
  - はみ出しの表示 (T0632): tools/overflow/OverflowOverlay.svelte。「Verify」のパネルで切り替える (操作は受けない)。
  - グルーの頂点の組 (T0661): tools/glue/GlueOverlay.svelte。「Glue」タブで選んだグルーの組を重ね、組を足す・外す。
  - モジュールのプロジェクト (T0650): 本体の要素の選択枠は灰色の破線にし、編集のツール (フォーム・ギズモ・メッシュ編集) を
    出さずに「本体のプロジェクトで直す」と示す。「Module outline」でモジュールのメッシュすべての輪郭を重ね、本体と見分ける。
-->
<script lang="ts">
import { Button } from "@ugoe/ui";
import type { EditorDocument } from "../document.svelte";
import { t } from "../i18n";
import { baseElements, isBaseElement } from "../moduleProject";
import type { GlueView } from "../panels/glues/glueView.svelte";
import type { PhysicsView } from "../panels/physics/physicsView.svelte";
import type { VerifyView } from "../panels/verify/verifyView.svelte";
import type { Selection } from "../selection.svelte";
import TipButton from "../ui/TipButton.svelte";
import type { ViewportController } from "./controller.svelte";
import DeformerGizmo from "./tools/deformer/DeformerGizmo.svelte";
import DepthOverlay from "./tools/depth/DepthOverlay.svelte";
import FormTool from "./tools/form/FormTool.svelte";
import { boundaryEdges, cycleHit } from "./tools/geometry";
import GlueOverlay from "./tools/glue/GlueOverlay.svelte";
import MeshEditTool from "./tools/MeshEditTool.svelte";
import OverflowOverlay from "./tools/overflow/OverflowOverlay.svelte";
import PhysicsOverlay from "./tools/physics/PhysicsOverlay.svelte";
import { ViewportScene } from "./tools/scene.svelte";

interface Props {
  controller: ViewportController;
  doc: EditorDocument;
  selection: Selection;
  physics: PhysicsView;
  verify: VerifyView;
  glue: GlueView;
}

let { controller, doc, selection, physics, verify, glue }: Props = $props();

/** これより動かなければクリックとみなす (CSS ピクセル)。 */
const CLICK_SLOP = 3;

// svelte-ignore state_referenced_locally
const scene = new ViewportScene(controller, doc);
let svg = $state<SVGSVGElement>();
let meshTool = $state<MeshEditTool>();
let press: { id: number; x: number; y: number; alt: boolean; toggle: boolean } | null = null;

// モデル・パラメータ・ビューが変わったら評価結果を読み直す。WebPuppet への読み込みは
// Viewport.svelte の $effect が行うので、その後 (マイクロタスク) に読む。
$effect(() => {
  void doc.model;
  void controller.transform;
  void controller.status;
  void controller.animationFrame;
  for (const value of Object.values(controller.parameterValues)) void value;
  queueMicrotask(() => scene.invalidate());
});

/** モジュールのプロジェクトの本体の要素 (読み取り専用)。本体のプロジェクトなら空。 */
const base = $derived(baseElements(doc.module));
const primaryIsBase = $derived(
  selection.primary !== null && isBaseElement(base, selection.primary),
);
/** モジュールのメッシュの輪郭を重ねるか (モジュールのプロジェクトだけ)。 */
let moduleOutline = $state(true);

const primaryMesh = $derived(
  selection.primary?.kind === "mesh" && scene.mesh(selection.primary.id)
    ? selection.primary.id
    : null,
);
const primaryDeformer = $derived(
  selection.primary?.kind === "deformer" ? selection.primary.id : null,
);
const editing = $derived(
  selection.meshEdit !== null && selection.meshEdit === primaryMesh ? selection.meshEdit : null,
);

// 選択が別の要素に移ったり、メッシュが消えたりしたらモードを抜ける。
$effect(() => {
  if (selection.meshEdit !== null && editing === null) selection.meshEdit = null;
});

/** メッシュ `id` の今の輪郭 (SVG のパス)。 */
function outlineOf(id: string): string | null {
  const mesh = scene.mesh(id);
  const p = scene.screenPositions(id);
  if (!mesh || !p) return null;
  return boundaryEdges(mesh.indices)
    .map(([a, b]) => `M${p[2 * a]},${p[2 * a + 1]}L${p[2 * b]},${p[2 * b + 1]}`)
    .join("");
}

/** 選択中のメッシュの輪郭。本体の要素は `base` (灰色の破線)。 */
const outlines = $derived(
  selection.items
    .filter((r) => r.kind === "mesh")
    .flatMap((r) => {
      const d = outlineOf(r.id);
      return d === null
        ? []
        : [{ id: r.id, d, primary: r.id === primaryMesh, base: isBaseElement(base, r) }];
    }),
);

/** モジュールのプロジェクトで、モジュールのメッシュすべての輪郭 (本体と見分けるため)。 */
const moduleOutlines = $derived.by(() => {
  if (!doc.module || !moduleOutline) return [];
  const ids = scene.model.meshes.map((m) => m.id).filter((id) => !base.has(`mesh:${id}`));
  return ids.flatMap((id) => {
    const d = outlineOf(id);
    return d === null ? [] : [{ id, d }];
  });
});

function enterMeshEdit(): void {
  if (!primaryMesh) return;
  // コマンドは既定値の姿を基準にするので、表示もそれに合わせる (docs/editor.md#メッシュの編集)。
  controller.resetParameters();
  selection.meshEdit = primaryMesh;
}

function local(e: PointerEvent | MouseEvent): [number, number] {
  const r = svg?.getBoundingClientRect();
  return r ? [e.clientX - r.left, e.clientY - r.top] : [e.clientX, e.clientY];
}

function onPointerDown(e: PointerEvent): void {
  if (e.button !== 0) return;
  const [x, y] = local(e);
  press = { id: e.pointerId, x, y, alt: e.altKey, toggle: e.shiftKey || e.ctrlKey || e.metaKey };
}

// Alt + ドラッグはビューポートがパンのためにポインタを捕まえるので、離したときは window で受ける。
function onWindowPointerUp(e: PointerEvent): void {
  const p = press;
  if (p?.id !== e.pointerId) return;
  press = null;
  const [x, y] = local(e);
  if (Math.hypot(x - p.x, y - p.y) >= CLICK_SLOP) return;
  const hits = scene.hitAll(x, y);
  if (p.alt) {
    const current = selection.primary?.kind === "mesh" ? selection.primary.id : null;
    const next = cycleHit(hits, current);
    if (next) selection.set([{ kind: "mesh", id: next }]);
    return;
  }
  const top = hits[0];
  if (p.toggle) {
    if (top) selection.toggle({ kind: "mesh", id: top });
  } else if (top) {
    selection.set([{ kind: "mesh", id: top }]);
  } else {
    selection.clear();
  }
}

function onDoubleClick(e: MouseEvent): void {
  const [x, y] = local(e);
  if (primaryMesh && scene.hitAll(x, y).includes(primaryMesh)) enterMeshEdit();
}

function onKeyDown(e: KeyboardEvent): void {
  if (editing === null) return;
  const target = e.target as HTMLElement | null;
  if (target?.closest("input, textarea, select, [contenteditable]")) return;
  if (e.key === "Escape") {
    selection.meshEdit = null;
  } else if (e.key === "Delete" || e.key === "Backspace") {
    e.preventDefault();
    meshTool?.removeSelected();
  } else if (e.key === "a" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    meshTool?.selectAll();
  }
}
</script>

<svelte:window onpointerup={onWindowPointerUp} onkeydown={onKeyDown} />

<svg
  bind:this={svg}
  class="layer"
  class:interactive={editing === null}
  role="presentation"
  onpointerdown={onPointerDown}
  ondblclick={onDoubleClick}
>
  {#each moduleOutlines as o (o.id)}
    <path class="module-outline" d={o.d} />
  {/each}
  {#each outlines as o (o.id)}
    <path class="outline" class:primary={o.primary} class:base={o.base} d={o.d} />
  {/each}
</svg>

{#if editing !== null}
  {#key editing}
    <MeshEditTool bind:this={meshTool} {scene} {doc} {selection} meshId={editing} />
  {/key}
{:else if primaryIsBase}
  <!-- 本体の要素は編集できないので、編集のツールを出さない -->
{:else if primaryDeformer !== null}
  {#key primaryDeformer}
    <DeformerGizmo {controller} {doc} {selection} deformerId={primaryDeformer} />
  {/key}
{:else}
  <FormTool {controller} {doc} {selection} />
{/if}

<DepthOverlay {controller} {doc} {selection} {scene} />
<OverflowOverlay {controller} {doc} {scene} view={verify} />
<PhysicsOverlay {controller} {doc} view={physics} />
<GlueOverlay {doc} {scene} view={glue} />

<div class="toolbar">
  {#if controller.previewingPhysics}
    <TipButton
      small
      class="previewing"
      onclick={() => controller.stopPhysicsPreview()}
      tooltip={t("viewport.stopPhysics.tooltip")}>{t("viewport.stopPhysics.label")}</TipButton
    >
  {/if}
  {#if editing === null}
    <TipButton
      small
      disabled={primaryMesh === null || primaryIsBase}
      onclick={enterMeshEdit}
      tooltip={t("viewport.editMesh.tooltip")}>{t("viewport.editMesh.label")}</TipButton
    >
    {#if doc.module}
      <TipButton
        small
        pressed={moduleOutline}
        onclick={() => (moduleOutline = !moduleOutline)}
        tooltip={t("viewport.moduleOutline.tooltip")}>{t("viewport.moduleOutline.label")}</TipButton
      >
    {/if}
    {#if primaryIsBase}
      <span class="base-tag" role="note">{t("viewport.baseNote")}</span>
    {/if}
  {:else}
    <span class="muted">{t("viewport.meshEdit.title", { mesh: editing })}</span>
    <Button small onclick={() => meshTool?.retriangulate()}>{t("viewport.meshEdit.retriangulate")}</Button>
    <Button small onclick={() => meshTool?.removeSelected()}>{t("viewport.meshEdit.deleteVertices")}</Button>
    <Button small onclick={() => (selection.meshEdit = null)}>{t("viewport.meshEdit.done")}</Button>
    <span class="hint muted">{t("viewport.meshEdit.hint")}</span>
  {/if}
</div>

<style>
.layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.layer.interactive {
  pointer-events: auto;
}

.outline {
  fill: none;
  stroke: var(--accent);
  stroke-width: 1;
  opacity: 0.7;
}

.outline.primary {
  stroke-width: 2;
  opacity: 1;
}

.outline.base {
  stroke: var(--muted, #888);
  stroke-dasharray: 4 3;
}

.module-outline {
  fill: none;
  stroke: var(--accent);
  stroke-width: 1;
  stroke-dasharray: 2 3;
  opacity: 0.55;
}

.base-tag {
  padding: 2px 6px;
  background: var(--panel);
  border: 1px dashed var(--border);
  border-radius: 4px;
}

.toolbar {
  position: absolute;
  top: 8px;
  left: 8px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  pointer-events: auto;
}

.hint {
  font-size: 11px;
}

.toolbar :global(.previewing) {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
