<!--
  インスペクタ: デフォーマの作成と、選択中のデフォーマの概要 (docs/editor.md#デフォーマの編集)。

  - 選択中のメッシュ・デフォーマ (複数可) を包むワープ (行数・列数を指定) か回転デフォーマを作る
    (create_warp_deformer / create_rotation_deformer)。子は見た目を保って付け替わり、作ったデフォーマを選択する。
  - デフォーマの形の編集はビューポートのギズモ (viewport/tools/deformer/DeformerGizmo.svelte) で行う。
  - 「Unwrap」: 子を見た目を保って親へ戻し、デフォーマを削除する (unwrap_deformer)。子を選択する。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import type { ChildRef, Model } from "../../ipc";
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

const deformer = $derived(
  selection.primary?.kind === "deformer"
    ? (model?.deformers.find((d) => d.id === selection.primary?.id) ?? null)
    : null,
);

const divisionsValid = $derived(
  [rows, cols].every((n) => Number.isInteger(n) && n >= 1 && n <= MAX_DIVISIONS),
);

async function create(kind: "warp" | "rotation"): Promise<void> {
  if (busy || children.length === 0) return;
  busy = true;
  try {
    const before = new Set(model?.deformers.map((d) => d.id));
    const name = kind === "warp" ? "Warp" : "Rotation";
    await doc.apply(
      kind === "warp"
        ? { op: "create_warp_deformer", name, children, rows, cols }
        : { op: "create_rotation_deformer", name, children },
    );
    if (doc.error) return;
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
      · {deformer.kind.keyforms.keyforms.length} keyforms · parent {deformer.parent ?? "(none)"}
    </p>
    <p class="muted hint">Edit the shape with the gizmo in the viewport.</p>
    <div class="row">
      <span class="muted hint">Move the children to the parent (keeping their look) and delete this</span>
      <button disabled={busy} onclick={() => unwrap(deformer.id)}>Unwrap</button>
    </div>
  </section>
{/if}

{#if children.length > 0}
  <section>
    <h3>Wrap in deformer</h3>
    <p class="muted hint">
      {children.length} selected · children keep their look (at the default parameter values)
    </p>
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

h3 {
  margin: 8px 0 4px;
}

.hint {
  font-size: 11px;
}
</style>
