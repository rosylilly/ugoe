<!--
  ビューポートの上に重ねる編集ツール (選択枠・頂点・ギズモ)。ツールは lib/viewport/tools/ に置き、ここで切り替える。
  座標は CSS ピクセル (ビューポート要素の左上原点)。評価後の頂点位置は ViewportScene が wasm から読む。

  - 選択モード: クリックで手前のメッシュを選ぶ (Shift / Ctrl で追加・解除)。Alt + クリックは重なった
    メッシュを手前から順に選ぶ。選択中のメッシュには輪郭を描く。ダブルクリックでメッシュ編集モードに入る。
  - メッシュ編集モード (T0606): tools/MeshEditTool.svelte。Esc で抜ける。
  - フォーム編集 (T0608): tools/form/FormTool.svelte。今のセルのキーフォームを編集する (メッシュ編集中は出さない)。
  デフォーマのギズモ (T0607) もここに足す。
-->
<script lang="ts">
import type { EditorDocument } from "../document.svelte";
import type { Selection } from "../selection.svelte";
import type { ViewportController } from "./controller.svelte";
import FormTool from "./tools/form/FormTool.svelte";
import { boundaryEdges, cycleHit } from "./tools/geometry";
import MeshEditTool from "./tools/MeshEditTool.svelte";
import { ViewportScene } from "./tools/scene.svelte";

interface Props {
  controller: ViewportController;
  doc: EditorDocument;
  selection: Selection;
}

let { controller, doc, selection }: Props = $props();

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
  for (const value of Object.values(controller.parameterValues)) void value;
  queueMicrotask(() => scene.invalidate());
});

const primaryMesh = $derived(
  selection.primary?.kind === "mesh" && scene.mesh(selection.primary.id)
    ? selection.primary.id
    : null,
);
const editing = $derived(
  selection.meshEdit !== null && selection.meshEdit === primaryMesh ? selection.meshEdit : null,
);

// 選択が別の要素に移ったり、メッシュが消えたりしたらモードを抜ける。
$effect(() => {
  if (selection.meshEdit !== null && editing === null) selection.meshEdit = null;
});

/** 選択中のメッシュの輪郭 (SVG のパス)。 */
const outlines = $derived(
  selection.items
    .filter((r) => r.kind === "mesh")
    .flatMap((r) => {
      const mesh = scene.mesh(r.id);
      const p = scene.screenPositions(r.id);
      if (!mesh || !p) return [];
      const d = boundaryEdges(mesh.indices)
        .map(([a, b]) => `M${p[2 * a]},${p[2 * a + 1]}L${p[2 * b]},${p[2 * b + 1]}`)
        .join("");
      return [{ id: r.id, d, primary: r.id === primaryMesh }];
    }),
);

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
  {#each outlines as o (o.id)}
    <path class="outline" class:primary={o.primary} d={o.d} />
  {/each}
</svg>

{#if editing !== null}
  {#key editing}
    <MeshEditTool bind:this={meshTool} {scene} {doc} {selection} meshId={editing} />
  {/key}
{:else}
  <FormTool {controller} {doc} {selection} />
{/if}

<div class="toolbar">
  {#if editing === null}
    <button disabled={primaryMesh === null} onclick={enterMeshEdit} title="Double-click a mesh">
      Edit mesh
    </button>
  {:else}
    <span class="muted">Mesh: {editing}</span>
    <button onclick={() => meshTool?.retriangulate()}>Retriangulate</button>
    <button onclick={() => meshTool?.removeSelected()}>Delete vertices</button>
    <button onclick={() => (selection.meshEdit = null)}>Done</button>
    <span class="hint muted">
      Drag: move / box select · Shift: add · Ctrl+click: add vertex · Del: remove · Esc: exit
    </span>
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
</style>
