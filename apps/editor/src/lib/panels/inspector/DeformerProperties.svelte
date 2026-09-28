<!--
  インスペクタ: デフォーマの作成と、選択中のデフォーマの概要 (docs/editor.md#デフォーマの編集)。

  - 選択中のメッシュ・デフォーマ・パーツ (複数可) を包むワープ (行数・列数を指定) か回転デフォーマを作る
    (create_warp_deformer / create_rotation_deformer)。パーツは `parts` で送り、その中のいちばん外側の
    メッシュ・デフォーマを包む。名前が空なら、バックエンドが子の名前から名前と ID を作る (`顔のワープ`)。
    子は見た目を保って付け替わり、作ったデフォーマを選択する。
  - デフォーマの形の編集はビューポートのギズモ (viewport/tools/deformer/DeformerGizmo.svelte) で行う。
  - 「Unwrap」: 子を見た目を保って親へ戻し、デフォーマを削除する (unwrap_deformer)。子を選択する。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import type { ChildRef, Model } from "../../ipc";
import { labelOf } from "../../labels";
import type { Selection } from "../../selection.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
}

let { doc, selection }: Props = $props();

/** 行数・列数の上限 (ugoe_editor_core の MAX_WARP_DIVISIONS)。 */
const MAX_DIVISIONS = 64;

let rows = $state(3);
let cols = $state(3);
/** 作るデフォーマの名前 (空なら子の名前から作る)。作るたびに空に戻す。 */
let name = $state("");
let busy = $state(false);

const model = $derived.by((): Model | null => {
  try {
    return doc.model ? (JSON.parse(doc.model.json) as Model) : null;
  } catch {
    return null;
  }
});

const children = $derived(
  selection.items.flatMap((r): ChildRef[] =>
    r.kind === "mesh" || r.kind === "deformer" ? [{ kind: r.kind, id: r.id }] : [],
  ),
);

const parts = $derived(selection.items.filter((r) => r.kind === "part").map((r) => r.id));

/** 包むものの説明 (`2 meshes, 1 part`)。 */
const summary = $derived.by(() => {
  const count = (n: number, one: string, many: string) =>
    n > 0 ? [`${n} ${n === 1 ? one : many}`] : [];
  const meshes = children.filter((c) => c.kind === "mesh").length;
  return [
    ...count(meshes, "mesh", "meshes"),
    ...count(children.length - meshes, "deformer", "deformers"),
    ...count(parts.length, "part", "parts"),
  ].join(", ");
});

const deformer = $derived(
  selection.primary?.kind === "deformer"
    ? (model?.deformers.find((d) => d.id === selection.primary?.id) ?? null)
    : null,
);

const divisionsValid = $derived(
  [rows, cols].every((n) => Number.isInteger(n) && n >= 1 && n <= MAX_DIVISIONS),
);

async function create(kind: "warp" | "rotation"): Promise<void> {
  if (busy || (children.length === 0 && parts.length === 0)) return;
  busy = true;
  try {
    const before = new Set(model?.deformers.map((d) => d.id));
    const common = { name: name.trim(), children, parts };
    const report = await doc.apply(
      kind === "warp"
        ? { op: "create_warp_deformer", ...common, rows, cols }
        : { op: "create_rotation_deformer", ...common },
    );
    if (!report) return;
    name = "";
    const created = model?.deformers.find((d) => !before.has(d.id));
    if (created) selection.set([{ kind: "deformer", id: created.id }]);
  } finally {
    busy = false;
  }
}

async function unwrap(id: string): Promise<void> {
  if (busy || !model) return;
  busy = true;
  try {
    const kids: ChildRef[] = [
      ...model.deformers
        .filter((d) => d.parent === id)
        .map((d) => ({ kind: "deformer" as const, id: d.id })),
      ...model.meshes
        .filter((m) => m.parent === id)
        .map((m) => ({ kind: "mesh" as const, id: m.id })),
    ];
    await doc.apply({ op: "unwrap_deformer", id });
    if (!doc.error) selection.set(kids);
  } finally {
    busy = false;
  }
}
</script>

{#if deformer}
  <section>
    <h3>Deformer</h3>
    <p class="muted">
      {deformer.name}
      {#if deformer.kind.type === "warp"}
        · warp {deformer.kind.cols}×{deformer.kind.rows}
      {:else if deformer.kind.type === "rotation"}
        · rotation
      {/if}
      · {deformer.kind.keyforms.keyforms.length} keyforms · parent
      {deformer.parent ? labelOf(model?.deformers, deformer.parent) : "(none)"}
    </p>
    <p class="muted hint">Edit the shape with the gizmo in the viewport.</p>
    <div class="row">
      <span class="muted hint">Move the children to the parent (keeping their look) and delete this</span>
      <button disabled={busy} onclick={() => unwrap(deformer.id)}>Unwrap</button>
    </div>
  </section>
{/if}

{#if children.length > 0 || parts.length > 0}
  <section>
    <h3>Wrap in deformer</h3>
    <p class="muted hint">
      {summary} selected{parts.length > 0 ? " (a part wraps its outermost meshes and deformers)" : ""} ·
      children keep their look (at the default parameter values)
    </p>
    <label class="row name">
      <span>Name</span>
      <input bind:value={name} placeholder="from the first selected" />
    </label>
    <div class="row">
      <label>Rows <input type="number" min="1" max={MAX_DIVISIONS} bind:value={rows} /></label>
      <label>Cols <input type="number" min="1" max={MAX_DIVISIONS} bind:value={cols} /></label>
      <button disabled={busy || !divisionsValid} onclick={() => create("warp")}>Warp</button>
    </div>
    <div class="row">
      <span class="muted hint">Origin: center of the selection</span>
      <button disabled={busy} onclick={() => create("rotation")}>Rotation</button>
    </div>
  </section>
{/if}

<style>
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 4px 0;
}

input[type="number"] {
  width: 4em;
}

.name {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
}

.name input {
  min-width: 0;
}

h3 {
  margin: 8px 0 4px;
}

.hint {
  font-size: 11px;
}
</style>
