<!--
  インスペクタ: 描画用の複製 (docs/editor.md#描画用の複製)。選択中のメッシュが複製なら元へ、元なら複製へ
  たどれるようにし、複製を作る・元に合わせ直す操作と、複製の今のセルの不透明度・描画順を出す
  (描画順のキーを作るため。すべてのセルをそろえるのは KeyformInspector)。マスクは MeshProperties で編集する。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import type { Model } from "../../../generated/types";
import { elementLabel } from "../../../labels";
import type { Selection } from "../../../selection.svelte";
import type { ViewportController } from "../../../viewport/controller.svelte";
import { cellIndex, currentCell } from "../keyforms/keyforms";
import { createCommand, drawCloneInfo, newClone } from "./drawClone";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
}

let { doc, selection, viewport }: Props = $props();

function parse(json: string | undefined): Model | null {
  if (!json) return null;
  try {
    return JSON.parse(json) as Model;
  } catch {
    return null;
  }
}

const model = $derived(parse(doc.model?.json));
const mesh = $derived(
  selection.primary?.kind === "mesh" && model
    ? (model.meshes.find((m) => m.id === selection.primary?.id) ?? null)
    : null,
);
const info = $derived(model && mesh ? drawCloneInfo(model, mesh.id) : null);

/** 今のパラメータ値のセル (キーの間なら null)。 */
const cell = $derived(
  mesh
    ? currentCell(mesh.keyforms.bindings, viewport.parameterValues, model?.parameters ?? [])
    : null,
);
const keyform = $derived(
  mesh && cell ? (mesh.keyforms.keyforms[cellIndex(mesh.keyforms.bindings, cell)] ?? null) : null,
);
let opacityInput = $state<number | null>(null);
let drawOrderInput = $state<number | null>(null);
$effect(() => {
  opacityInput = keyform?.opacity ?? null;
  drawOrderInput = keyform?.draw_order ?? null;
});

function select(id: string): void {
  selection.set([{ kind: "mesh", id }]);
}

async function createClone(): Promise<void> {
  if (!mesh || !model) return;
  const source = mesh.id;
  const before = model;
  if (!(await doc.apply(createCommand(source)))) return;
  const after = parse(doc.model?.json);
  const id = after ? newClone(before, after, source) : null;
  if (id) select(id);
}

async function sync(): Promise<void> {
  if (mesh) await doc.apply({ op: "sync_draw_clone", id: mesh.id });
}

/** 今のセルの不透明度か描画順を変える (set_keyform_cell。セルは authored になる)。 */
async function setCell(field: "opacity" | "draw_order", value: number | null): Promise<void> {
  if (!mesh || !cell || !keyform || value === null || !Number.isFinite(value)) return;
  await doc.apply({
    op: "set_keyform_cell",
    target: { kind: "mesh", id: mesh.id },
    cell,
    keyform: { kind: "mesh", ...keyform, [field]: value },
  });
}
</script>

{#if mesh && info}
  <section class="draw-clone">
    <h3>Draw clone</h3>
    {#if info.role === "clone"}
      <p class="row">
        <span>Clone of</span>
        {#if info.original}
          <button onclick={() => info.original && select(info.original.id)}>
            {elementLabel(info.original.name, info.original.id)}
          </button>
        {/if}
      </p>
      <p class="muted">Shares the shape of the original. Give it its own draw order and opacity here, and masks in the Mesh section.</p>
      {#if !info.inSync}
        <p class="warning row">
          <span>Out of sync with the original</span>
          <button onclick={sync}>Resync</button>
        </p>
      {/if}
      <label class="row">
        <span>Opacity (this cell)</span>
        <input
          type="number"
          min="0"
          max="1"
          step="0.05"
          disabled={!keyform}
          bind:value={opacityInput}
          onchange={() => setCell("opacity", opacityInput)}
        />
      </label>
      <label class="row">
        <span>Draw order (this cell)</span>
        <input
          type="number"
          step="10"
          disabled={!keyform}
          bind:value={drawOrderInput}
          onchange={() => setCell("draw_order", drawOrderInput)}
        />
      </label>
      {#if !keyform}
        <p class="muted">Between keys: move the parameters onto a key to edit this cell.</p>
      {/if}
    {:else}
      {#if info.clones.length > 0}
        <ul>
          {#each info.clones as clone (clone.id)}
            <li class="row">
              <span>Clone</span>
              <button onclick={() => select(clone.id)}>{elementLabel(clone.name, clone.id)}</button>
            </li>
          {/each}
        </ul>
      {/if}
      <button onclick={createClone} title="Duplicate this mesh for another draw order or mask; it follows this mesh's shape">
        Create draw clone
      </button>
    {/if}
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

.row input {
  width: 6em;
}

.warning {
  padding: 4px 6px;
  border: 1px solid #d97706;
  border-radius: 4px;
  background: color-mix(in srgb, #d97706 12%, transparent);
}

ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

h3 {
  margin: 8px 0 4px;
}
</style>
