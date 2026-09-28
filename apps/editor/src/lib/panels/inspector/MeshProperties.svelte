<!--
  インスペクタ: 選択中のメッシュの属性 (テクスチャ・合成・カリング・マスク)。
  変更は 1 項目ずつ set_mesh_properties で送る (指定したフィールドだけが変わる)。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import type { EditCommand } from "../../ipc";
import { elementLabel, textureLabel } from "../../labels";
import type { Selection } from "../../selection.svelte";
import {
  type BlendMode,
  type MaskMode,
  type MaskRef,
  summarizeModel,
} from "../../viewport/tools/model";

interface Props {
  doc: EditorDocument;
  selection: Selection;
}

let { doc, selection }: Props = $props();

const model = $derived(summarizeModel(doc.model?.json));
const mesh = $derived(
  selection.primary?.kind === "mesh"
    ? (model.meshes.find((m) => m.id === selection.primary?.id) ?? null)
    : null,
);
const maskCandidates = $derived(model.meshes.filter((m) => m.id !== mesh?.id));
/** テクスチャの表記 (名前と、それを使うメッシュの表示名)。 */
const textureLabels = $derived(
  model.textures.map((name, i) =>
    textureLabel(
      name,
      model.meshes.filter((m) => m.texture === i).map((m) => m.name),
    ),
  ),
);

type Changes = Omit<Extract<EditCommand, { op: "set_mesh_properties" }>, "op" | "id">;

function set(changes: Changes): void {
  if (mesh) doc.apply({ op: "set_mesh_properties", id: mesh.id, ...changes });
}

function value(e: Event): string {
  return (e.currentTarget as HTMLInputElement | HTMLSelectElement).value;
}

function checked(e: Event): boolean {
  return (e.currentTarget as HTMLInputElement).checked;
}

function setMask(index: number, change: Partial<MaskRef>): void {
  if (!mesh) return;
  set({ masks: mesh.masks.map((m, i) => (i === index ? { ...m, ...change } : m)) });
}

function addMask(): void {
  const first = maskCandidates.find((c) => !mesh?.masks.some((m) => m.mesh === c.id));
  if (mesh && first) set({ masks: [...mesh.masks, { mesh: first.id, invert: false }] });
}

function removeMask(index: number): void {
  if (mesh) set({ masks: mesh.masks.filter((_, i) => i !== index) });
}
</script>

{#if mesh}
  <section>
    <h3>Mesh</h3>
    <p class="muted">{mesh.name} · {mesh.vertices} vertices · {mesh.indices.length / 3} triangles</p>
    <label class="row">
      <span>Texture</span>
      <select value={String(mesh.texture)} onchange={(e) => set({ texture: Number(value(e)) })}>
        {#each textureLabels as label, i (i)}
          <option value={String(i)}>{label}</option>
        {/each}
      </select>
    </label>
    <label class="row">
      <span>Blend</span>
      <select value={mesh.blend} onchange={(e) => set({ blend: value(e) as BlendMode })}>
        <option value="normal">Normal</option>
        <option value="additive">Additive</option>
        <option value="multiply">Multiply</option>
      </select>
    </label>
    <label class="row">
      <span>Back-face culling</span>
      <input type="checkbox" checked={mesh.culling} onchange={(e) => set({ culling: checked(e) })} />
    </label>
    <label class="row">
      <span>Mask only</span>
      <input
        type="checkbox"
        checked={mesh.maskOnly}
        onchange={(e) => set({ mask_only: checked(e) })}
      />
    </label>

    <h4 class="row">
      <span>Masks</span>
      <button onclick={addMask} disabled={maskCandidates.length === 0}>Add</button>
    </h4>
    {#if mesh.masks.length > 0}
      <label class="row">
        <span>Combine</span>
        <select value={mesh.maskMode} onchange={(e) => set({ mask_mode: value(e) as MaskMode })}>
          <option value="union">Union</option>
          <option value="intersection">Intersection</option>
        </select>
      </label>
    {/if}
    <ul>
      {#each mesh.masks as mask, i (i)}
        <li class="row">
          <select value={mask.mesh} onchange={(e) => setMask(i, { mesh: value(e) })}>
            {#each maskCandidates as c (c.id)}
              <option value={c.id}>{elementLabel(c.name, c.id)}</option>
            {/each}
          </select>
          <label>
            <input
              type="checkbox"
              checked={mask.invert}
              onchange={(e) => setMask(i, { invert: checked(e) })}
            />
            Invert
          </label>
          <button onclick={() => removeMask(i)} aria-label="Remove mask">×</button>
        </li>
      {/each}
    </ul>
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

.row select {
  min-width: 0;
  max-width: 100%;
  flex: 0 1 auto;
}

ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

h3,
h4 {
  margin: 8px 0 4px;
}
</style>
