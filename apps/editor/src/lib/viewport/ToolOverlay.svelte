<!--
  ビューポートの上に重ねる編集ツール (選択枠・頂点・ギズモ)。ツールは lib/viewport/tools/ に置き、ここで切り替える。
  座標は CSS ピクセル (ビューポート要素の左上原点)。評価後の頂点位置は ViewportScene が wasm から読む。

  - 選択モード: クリックで手前のメッシュを選ぶ (Shift / Ctrl で追加・解除)。Alt + クリックは重なった
    メッシュを手前から順に選ぶ。選択中のメッシュには輪郭を描く。ダブルクリックでメッシュ編集モードに入る。
    ポインタが乗っている要素 (`Selection.hover`。ツリーの行と共有) の中のメッシュには細い破線の輪郭を描く。
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

  左上の並び (.strip。docs/editor.md#ビューポートの並び): 1 行目が道具のボタン (メッシュ編集・フォーム編集・モジュールの輪郭・物理を止める)、
  その下に今の道具の説明の帯 (.vp-band。対象・セル・選んだ数・注意・操作の説明) を、テーマの地に載せて縦に積む。
  帯は ToolOverlay・FormTool・DeformerGizmo が描き、並びは右上の HUD の幅を空けて折り返す (重ならない)。
  ツールの svg は .strip の中にあっても `position: absolute` でオーバーレイ全体に広がる (.strip は位置の基準にしない)。
-->
<script lang="ts">
import { Button } from "@ugoe/ui";
import type { EditorDocument } from "../document.svelte";
import { t } from "../i18n";
import { baseElements, isBaseElement } from "../moduleProject";
import type { GlueView } from "../panels/glues/glueView.svelte";
import type { PhysicsView } from "../panels/physics/physicsView.svelte";
import { indexModel, meshesWithin } from "../panels/tree/build";
import { treeModel } from "../panels/tree/model";
import type { VerifyView } from "../panels/verify/verifyView.svelte";
import type { Selection } from "../selection.svelte";
import TipButton from "../ui/TipButton.svelte";
import type { ViewportController } from "./controller.svelte";
import DeformerGizmo from "./tools/deformer/DeformerGizmo.svelte";
import DepthOverlay from "./tools/depth/DepthOverlay.svelte";
import FormTool from "./tools/form/FormTool.svelte";
import { boundaryEdges, cycleHit } from "./tools/geometry";
import GlueOverlay from "./tools/glue/GlueOverlay.svelte";
import {
  acceptsNudgeKey,
  acceptsToolKey,
  isSplitKey,
  nudgeOf,
  pointExtendOf,
  pointStepOf,
} from "./tools/keys";
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
/** フォーム編集を出しているか (道具の並びのボタンで切り替え、FormTool が描く)。 */
let formEnabled = $state(false);

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

/** メッシュ編集で選んでいる頂点 (メッシュの頂点の数より小さいもの)。 */
const pickedVertices = $derived.by(() => {
  if (editing === null) return [];
  const total = scene.mesh(editing)?.vertices ?? 0;
  return (selection.vertices[editing] ?? []).filter((i) => i < total);
});

/**
 * メッシュ編集で選んだ頂点が何番目か (帯の読み上げの領域)。1 つなら `]` `[` で選ぶ点、2 つなら Shift + `]` `[` で選んで
 * 中点に頂点を足す (`I`) 2 つ。
 */
const currentVertexText = $derived.by(() => {
  if (editing === null) return "";
  const total = scene.mesh(editing)?.vertices ?? 0;
  const [a, b] = pickedVertices;
  if (pickedVertices.length === 1 && a !== undefined) {
    return t("viewport.point.current", { index: a + 1, total });
  }
  if (pickedVertices.length === 2 && a !== undefined && b !== undefined) {
    return t("viewport.point.pair", { a: a + 1, b: b + 1, total });
  }
  return "";
});

// 選択が別の要素に移ったり、メッシュが消えたりしたらモードを抜ける。
$effect(() => {
  if (selection.meshEdit !== null && editing === null) selection.meshEdit = null;
});

/** 三角形の列ごとの境界の辺 (モデルを読み直すまで同じ配列なので、ポインタの移動や再生のたびに数え直さない)。 */
const edgeCache = new WeakMap<ArrayLike<number>, [number, number][]>();

/** メッシュ `id` の今の輪郭 (SVG のパス)。 */
function outlineOf(id: string): string | null {
  const mesh = scene.mesh(id);
  const p = scene.screenPositions(id);
  if (!mesh || !p) return null;
  let edges = edgeCache.get(mesh.indices);
  if (!edges) {
    edges = boundaryEdges(mesh.indices);
    edgeCache.set(mesh.indices, edges);
  }
  return edges.map(([a, b]) => `M${p[2 * a]},${p[2 * a + 1]}L${p[2 * b]},${p[2 * b + 1]}`).join("");
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

/** ツリーと同じ索引 (パーツ・デフォーマの中のメッシュを引く)。読めないモデルなら null。 */
const treeIndex = $derived.by(() => {
  if (!doc.parsedModel) return null;
  try {
    return indexModel(treeModel(doc.parsedModel));
  } catch {
    return null;
  }
});

/**
 * ポインタが乗っている要素 (ツリーの行かビューポートのメッシュ) の中のメッシュの輪郭。パーツ・デフォーマの行なら
 * 子孫のメッシュすべてを 1 本のパスにまとめる (本体の要素は別のパス)。選んでいるメッシュには選択の輪郭があるので重ねない
 * (docs/editor.md#選択とホバーの見せ方)。
 */
const hoverOutlines = $derived.by(() => {
  const hover = selection.hover;
  if (!hover || !treeIndex) return [];
  const parts = { own: [] as string[], base: [] as string[] };
  for (const id of meshesWithin(treeIndex, hover)) {
    const ref = { kind: "mesh" as const, id };
    if (selection.has(ref)) continue;
    const d = outlineOf(id);
    if (d !== null) (isBaseElement(base, ref) ? parts.base : parts.own).push(d);
  }
  return [
    { d: parts.own.join(""), base: false },
    { d: parts.base.join(""), base: true },
  ].filter((o) => o.d !== "");
});

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

/** ポインタの下の手前のメッシュをホバーにする (ボタンを押していない間だけ。ツリーの行も薄く示す)。 */
function onPointerMove(e: PointerEvent): void {
  if (e.buttons !== 0 || e.pointerType === "touch") return;
  const [x, y] = local(e);
  const top = scene.hitAll(x, y)[0];
  const current = selection.hover?.kind === "mesh" ? selection.hover.id : null;
  if ((top ?? null) === current) return;
  selection.hover = top ? { kind: "mesh", id: top } : null;
}

function onPointerLeave(): void {
  if (selection.hover?.kind === "mesh") selection.hover = null;
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
  if (editing === null || !acceptsToolKey(e.target)) return;
  if (e.key === "Escape") {
    selection.meshEdit = null;
  } else if (e.key === "Delete" || e.key === "Backspace") {
    e.preventDefault();
    meshTool?.removeSelected();
  } else if (e.key === "a" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    meshTool?.selectAll();
  } else if (acceptsNudgeKey(e.target)) {
    // `]` `[` で頂点を 1 つずつ選び (Shift で 2 つ目)、`I` で 2 つの中点に足し、矢印キーで動かす
    // (docs/editor.md#ビューポートの操作と代わりの手段)
    const extend = pointExtendOf(e);
    const dir = pointStepOf(e);
    const step = nudgeOf(e);
    if (extend && meshTool?.extend(extend)) {
      e.preventDefault();
    } else if (isSplitKey(e) && meshTool?.addMidpoint()) {
      e.preventDefault();
    } else if (dir && meshTool?.step(dir)) {
      e.preventDefault();
    } else if (step) {
      e.preventDefault();
      meshTool?.nudge(step[0], step[1]);
    }
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
  onpointermove={onPointerMove}
  onpointerleave={onPointerLeave}
  ondblclick={onDoubleClick}
>
  {#each moduleOutlines as o (o.id)}
    <path class="module-outline" d={o.d} />
  {/each}
  {#each hoverOutlines as o (o.base)}
    <path class="outline hover" class:base={o.base} d={o.d} />
  {/each}
  {#each outlines as o (o.id)}
    <path class="outline" class:primary={o.primary} class:base={o.base} d={o.d} />
  {/each}
</svg>

<!-- ツールの svg は下の表示の重ねより下に描く (DOM の順)。ボタンと帯は z-index で上 -->
<div class="strip">
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
      {#if !primaryIsBase && primaryDeformer === null}
        <TipButton
          small
          pressed={formEnabled}
          onclick={() => (formEnabled = !formEnabled)}
          tooltip={t("viewport.form.toggle.tooltip")}>{t("viewport.form.toggle.label")}</TipButton
        >
      {/if}
      {#if doc.module}
        <TipButton
          small
          pressed={moduleOutline}
          onclick={() => (moduleOutline = !moduleOutline)}
          tooltip={t("viewport.moduleOutline.tooltip")}>{t("viewport.moduleOutline.label")}</TipButton
        >
      {/if}
    {:else}
      <TipButton
        small
        disabled={pickedVertices.length !== 2}
        shortcut="I"
        onclick={() => meshTool?.addMidpoint()}
        tooltip={t("viewport.meshEdit.addMidpoint.tooltip")}>{t("viewport.meshEdit.addMidpoint.label")}</TipButton
      >
      <Button small onclick={() => meshTool?.retriangulate()}>{t("viewport.meshEdit.retriangulate")}</Button>
      <Button small onclick={() => meshTool?.removeSelected()}>{t("viewport.meshEdit.deleteVertices")}</Button>
      <Button small onclick={() => (selection.meshEdit = null)}>{t("viewport.meshEdit.done")}</Button>
    {/if}
  </div>
  {#if editing !== null}
    <p class="vp-band">
      <span>{t("viewport.meshEdit.title", { mesh: editing })}</span>
      <span class="hint muted">{t("viewport.meshEdit.hint")}</span>
      <span class="muted" role="status">{currentVertexText}</span>
    </p>
    {#key editing}
      <MeshEditTool
        bind:this={meshTool}
        {scene}
        {doc}
        {selection}
        meshId={editing}
        reveal={(x, y) => controller.reveal(x, y)}
      />
    {/key}
  {:else if primaryIsBase}
    <!-- 本体の要素は編集できないので、編集のツールを出さない -->
    <p class="vp-band base-note" role="note">{t("viewport.baseNote")}</p>
  {:else if primaryDeformer !== null}
    {#key primaryDeformer}
      <DeformerGizmo {controller} {doc} {selection} deformerId={primaryDeformer} />
    {/key}
  {:else}
    <FormTool {controller} {doc} {selection} enabled={formEnabled} />
  {/if}
</div>

<!-- 表示の重ね (ツールの svg より上。物理の衝突体はほかのツールより上でドラッグを受ける) -->
<DepthOverlay {controller} {doc} {selection} {scene} />
<OverflowOverlay {controller} {doc} {scene} view={verify} />
<PhysicsOverlay {controller} {doc} view={physics} />
<GlueOverlay {doc} {scene} view={glue} />

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
  stroke: var(--viewport-edit);
  stroke-width: 1;
}

.outline.primary {
  stroke-width: 2;
}

/* ホバー: 選択と同じ色の破線 (ツリーの行のホバーと対応する)。薄くすると地に 3:1 を割るので、形 (破線) で分ける */
.outline.hover {
  stroke-dasharray: 3 3;
}

.outline.base {
  stroke: var(--viewport-base);
  stroke-dasharray: 4 3;
}

.module-outline {
  fill: none;
  stroke: var(--viewport-edit);
  stroke-width: 1;
  stroke-dasharray: 2 3;
}

/* 左上の並び: 道具のボタンの行と説明の帯を縦に積む。右上の HUD (Viewport.svelte) の幅を空けて折り返す (T0907)。
   位置の基準にしない (中のツールの svg がオーバーレイ全体に広がるように) ので、`position` を付けず、流れの中に置く */
.strip {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--ugoe-space-1);
  max-width: calc(100% - var(--viewport-hud-width, 0px) - 24px);
  padding: 8px;
}

/* 押せるものと帯だけがポインタを受け、ツールの svg より上に描く */
.toolbar,
.strip :global(.vp-band) {
  position: relative;
  z-index: 1;
  pointer-events: auto;
}

.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ugoe-space-2);
}

/* 説明の帯: モデルの上に重なるので、テーマの地と枠に載せて読めるようにする (FormTool・DeformerGizmo も使う) */
.strip :global(.vp-band) {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ugoe-space-1) var(--ugoe-space-3);
  max-width: 100%;
  margin: 0;
  padding: var(--ugoe-space-1) var(--ugoe-space-2);
  background: var(--ugoe-color-surface);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
  overflow-wrap: anywhere;
}

.base-note {
  border-style: dashed;
}

/* 操作の説明は小さな字で (モデルを隠す範囲を減らす) */
.strip :global(.vp-band .hint) {
  font-size: var(--ugoe-font-size-xs);
}

.toolbar :global(.previewing) {
  border-color: var(--ugoe-color-accent);
  color: var(--ugoe-color-accent);
}
</style>
