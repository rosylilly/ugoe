<!--
  ライティングのパネル (docs/editor.md#ライティングの画面)。右の列の「Lighting」タブ。
  - モデル全体のライティングの有無 (set_lighting) と、メッシュごとの有無 (set_mesh_properties の lit)。
    モジュールのプロジェクトでは、本体のメッシュの有無は変えられない (欄を無効にし、まとめての操作からも除く)。
  - 選んだメッシュのテクスチャの法線マップ (NormalMapSection)。
  - プレビューの光源 (PreviewLightsSection)。パネルを開いている間、ビューポートに光源の取っ手を重ねる。
  モデルがライティングを受けないときは、光源を動かしても見た目が変わらない理由を出す。
-->
<script lang="ts">
import BaseLock from "../../BaseLock.svelte";
import type { EditorDocument } from "../../document.svelte";
import type { Command, Model } from "../../ipc";
import { elementLabel } from "../../labels";
import { BASE_EDIT_HINT, isBaseMesh, isBaseTexture } from "../../moduleProject";
import type { Selection } from "../../selection.svelte";
import { isLit, lightingHint, litCandidates } from "./lighting";
import type { LightingView } from "./lightingView.svelte";
import NormalMapSection from "./NormalMapSection.svelte";
import PreviewLightsSection from "./PreviewLightsSection.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  view: LightingView;
}

let { doc, selection, view }: Props = $props();

const model = $derived.by((): Model | null => {
  try {
    return doc.model ? (JSON.parse(doc.model.json) as Model) : null;
  } catch {
    return null;
  }
});
const meshes = $derived(model ? litCandidates(model) : []);
const hint = $derived(model && meshes.length > 0 ? lightingHint(model) : null);
const selectedMesh = $derived(
  selection.primary?.kind === "mesh"
    ? (model?.meshes.find((m) => m.id === selection.primary?.id) ?? null)
    : null,
);
const canvas = $derived(model?.meta.canvas ?? { width: 1024, height: 1024 });

function setLit(ids: string[], lit: boolean): void {
  const commands: Command[] = ids.map((id) => ({ op: "set_mesh_properties", id, lit }));
  if (commands.length === 1 && commands[0]) void doc.apply(commands[0]);
  else if (commands.length > 1)
    void doc.apply({ op: "batch", label: lit ? "Light meshes" : "Unlight meshes", commands });
}

/** 変えられるメッシュ (モジュールのプロジェクトでは、本体のメッシュを除く)。 */
const editable = $derived(meshes.filter((m) => !isBaseMesh(doc.module, m.id)));

/** 選んでいるメッシュ (なければすべて) の lit を変える。 */
function setLitForSelection(lit: boolean): void {
  const selected = selection.items.filter((r) => r.kind === "mesh").map((r) => r.id);
  const targets = selected.length > 0 ? editable.filter((m) => selected.includes(m.id)) : editable;
  setLit(
    targets.filter((m) => isLit(m) !== lit).map((m) => m.id),
    lit,
  );
}

const selectedCount = $derived(selection.items.filter((r) => r.kind === "mesh").length);
</script>

<h2>Lighting</h2>
{#if !model || meshes.length === 0}
  <p class="muted">Import a PSD first. Lighting shades meshes with the lights below.</p>
{:else}
  <section aria-label="Model lighting">
    <BaseLock
      locked={doc.module !== null}
      hint="Whether the model receives lighting is set in the base project."
    >
    <label class="row">
      <span>Receive lighting (model)</span>
      <input
        type="checkbox"
        checked={model.lighting === true}
        onchange={(e) => void doc.apply({ op: "set_lighting", enabled: (e.currentTarget as HTMLInputElement).checked })}
      />
    </label>
    </BaseLock>
    {#if hint}<p class="hint">{hint}</p>{/if}
  </section>

  <section aria-label="Lit meshes">
    <h3 class="row">
      <span>Lit meshes</span>
      <span class="buttons">
        <button disabled={editable.length === 0} onclick={() => setLitForSelection(true)}
          >{selectedCount > 0 ? "Light selected" : "All on"}</button
        >
        <button disabled={editable.length === 0} onclick={() => setLitForSelection(false)}
          >{selectedCount > 0 ? "Unlight selected" : "All off"}</button
        >
      </span>
    </h3>
    <p class="muted note">
      Turn lighting off for parts that should stay flat, such as eye highlights.
      {#if doc.module}The base model's meshes are set in the base project; the buttons change only the module's
        meshes.{/if}
    </p>
    <ul class="meshes">
      {#each meshes as mesh (mesh.id)}
        {@const base = isBaseMesh(doc.module, mesh.id)}
        <li class:selected={selection.has({ kind: "mesh", id: mesh.id })} class:base>
          <input
            type="checkbox"
            aria-label="Lit: {elementLabel(mesh.name, mesh.id)}"
            title={base ? BASE_EDIT_HINT : undefined}
            disabled={base}
            checked={isLit(mesh)}
            onchange={(e) => setLit([mesh.id], (e.currentTarget as HTMLInputElement).checked)}
          />
          <button class="link" onclick={() => selection.set([{ kind: "mesh", id: mesh.id }])}>
            {elementLabel(mesh.name, mesh.id)}
          </button>
          {#if base}<span class="badge" title="Part of the base model: read-only in a module project">base</span>{/if}
          {#if model.textures[mesh.texture]?.normal_map}<span class="badge" title="Has a normal map">N</span>{/if}
        </li>
      {/each}
    </ul>
  </section>

  {#if selectedMesh}
    <BaseLock locked={isBaseTexture(doc.module, model.textures[selectedMesh.texture]?.name ?? "")}>
      <NormalMapSection {doc} {model} texture={selectedMesh.texture} />
    </BaseLock>
  {:else}
    <h3>Normal map</h3>
    <p class="muted">Select a mesh to generate or assign a normal map for its texture.</p>
  {/if}
{/if}

<PreviewLightsSection {view} {canvas} />

<style>
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 4px 0;
}

.buttons {
  display: flex;
  gap: 4px;
  font-weight: normal;
}

.hint {
  margin: 4px 0;
  padding: 4px 8px;
  border-left: 3px solid var(--accent);
  font-size: 12px;
}

.meshes {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 180px;
  overflow-y: auto;
  border: 1px solid var(--border);
}

.meshes li {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 1px 4px;
}

.meshes li.base {
  opacity: 0.65;
}

.meshes li.selected {
  background: color-mix(in srgb, var(--accent) 15%, transparent);
}

.link {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
  border: none;
  background: none;
  padding: 0;
  color: inherit;
}

.badge {
  font-size: 10px;
  padding: 0 4px;
  border: 1px solid var(--border);
  border-radius: 3px;
}

.note {
  margin: 2px 0 6px;
  font-size: 11px;
}

h3 {
  margin: 8px 0 4px;
}
</style>
