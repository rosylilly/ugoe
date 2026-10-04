<!--
  ライティングのパネル (docs/editor.md#ライティングの画面)。右の列の「Lighting」タブ。
  - モデル全体のライティングの有無 (set_lighting) と、メッシュごとの有無 (set_mesh_properties の lit)。
    モジュールのプロジェクトでは、本体のメッシュの有無は変えられない (欄を無効にし、まとめての操作からも除く)。
  - 選んだメッシュのテクスチャの法線マップ (NormalMapSection)。
  - プレビューの光源 (PreviewLightsSection)。パネルを開いている間、ビューポートに光源の取っ手を重ねる。
  モデルがライティングを受けないときは、光源を動かしても見た目が変わらない理由を出す。
-->
<script lang="ts">
import { Button, Toggle, Tooltip } from "@ugoe/ui";
import BaseLock from "../../BaseLock.svelte";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { Command } from "../../ipc";
import { elementLabel } from "../../labels";
import { isBaseMesh, isBaseTexture } from "../../moduleProject";
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

const model = $derived(doc.parsedModel);
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
    void doc.apply({
      op: "batch",
      label: lit ? t("lighting.meshes.lightBatch") : t("lighting.meshes.unlightBatch"),
      commands,
    });
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

const meshesId = $props.id();
const selectedCount = $derived(selection.items.filter((r) => r.kind === "mesh").length);
</script>

<h2>{t("lighting.title")}</h2>
{#if !model || meshes.length === 0}
  <p class="ed-hint">{t("lighting.empty")}</p>
{:else}
  <section aria-label={t("lighting.model.aria")}>
    <BaseLock locked={doc.module !== null} hint={t("lighting.model.baseHint")}>
      <div class="row">
        <Toggle
          label={t("lighting.model.receive.label")}
          checked={model.lighting === true}
          onchange={(enabled) => void doc.apply({ op: "set_lighting", enabled })}
        />
      </div>
    </BaseLock>
    {#if hint}<p class="hint">{hint}</p>{/if}
  </section>

  <section aria-labelledby={meshesId}>
    <div class="row">
      <h3 id={meshesId}>{t("lighting.meshes.title")}</h3>
      <span class="ed-buttons">
        <Button small disabled={editable.length === 0} onclick={() => setLitForSelection(true)}
          >{selectedCount > 0 ? t("lighting.meshes.lightSelected") : t("lighting.meshes.allOn")}</Button
        >
        <Button small disabled={editable.length === 0} onclick={() => setLitForSelection(false)}
          >{selectedCount > 0 ? t("lighting.meshes.unlightSelected") : t("lighting.meshes.allOff")}</Button
        >
      </span>
    </div>
    <p class="ed-hint note">
      {t("lighting.meshes.description")}
      {#if doc.module}{t("lighting.meshes.moduleNote")}{/if}
    </p>
    <ul class="meshes">
      {#each meshes as mesh (mesh.id)}
        {@const base = isBaseMesh(doc.module, mesh.id)}
        <li class:selected={selection.has({ kind: "mesh", id: mesh.id })} class:base>
          <!-- 行の名前のボタンは選ぶ操作なので、見えるラベルを持つ Checkbox にせず、名前を aria-label で付ける。 -->
          <input
            type="checkbox"
            aria-label={t("lighting.meshes.lit.aria", { name: elementLabel(mesh.name, mesh.id) })}
            disabled={base}
            checked={isLit(mesh)}
            onchange={(e) => setLit([mesh.id], (e.currentTarget as HTMLInputElement).checked)}
          />
          <button class="link" onclick={() => selection.set([{ kind: "mesh", id: mesh.id }])}>
            {elementLabel(mesh.name, mesh.id)}
          </button>
          {#if base}
            <Tooltip text={t("lighting.meshes.base.tooltip")}>
              {#snippet children(attrs)}<span {...attrs} class="badge">{t("lighting.meshes.base.label")}</span>{/snippet}
            </Tooltip>
          {/if}
          {#if model.textures[mesh.texture]?.normal_map}
            <Tooltip text={t("lighting.meshes.normalMap.tooltip")}>
              {#snippet children(attrs)}<span {...attrs} class="badge"
                  >{t("lighting.meshes.normalMap.label")}</span
                >{/snippet}
            </Tooltip>
          {/if}
        </li>
      {/each}
    </ul>
  </section>

  {#if selectedMesh}
    <BaseLock locked={isBaseTexture(doc.module, model.textures[selectedMesh.texture]?.name ?? "")}>
      <NormalMapSection {doc} {model} texture={selectedMesh.texture} />
    </BaseLock>
  {:else}
    <h3>{t("lighting.normalMap.title")}</h3>
    <p class="ed-hint">{t("lighting.normalMap.empty")}</p>
  {/if}
{/if}

<PreviewLightsSection {view} {canvas} />

<style>
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ugoe-space-3);
  margin: var(--ugoe-space-2) 0;
}

.row h3 {
  margin: 0;
}

.hint {
  margin: var(--ugoe-space-2) 0;
  padding: var(--ugoe-space-2) var(--ugoe-space-3);
  border-left: 3px solid var(--ugoe-color-accent);
  font-size: var(--ugoe-font-size-sm);
}

.meshes {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 180px;
  overflow-y: auto;
  border: 1px solid var(--ugoe-color-border);
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
  background: color-mix(in srgb, var(--ugoe-color-accent) 15%, transparent);
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
  font-size: var(--ugoe-font-size-xs);
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-sm);
}

.note {
  margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
}
</style>
