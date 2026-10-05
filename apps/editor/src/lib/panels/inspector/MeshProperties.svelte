<!--
  インスペクタ: 選択中のメッシュの属性 (テクスチャ・合成・カリング・マスク)。
  変更は 1 項目ずつ set_mesh_properties で送る (指定したフィールドだけが変わる)。
-->
<script lang="ts">
import { Button, Checkbox, Icon, IconButton, Select } from "@ugoe/ui";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
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

const model = $derived(summarizeModel(doc.parsedModel));
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

// テクスチャは名前で送る (番号は `remove_texture` で詰まるので使わない。docs/editor.md#コマンド一覧)。
const textureOptions = $derived(
  textureLabels.map((label, i) => ({ value: model.textures[i] ?? "", label })),
);
const blendOptions = $derived<{ value: BlendMode; label: string }[]>([
  { value: "normal", label: t("inspector.mesh.blend.normal") },
  { value: "additive", label: t("inspector.mesh.blend.additive") },
  { value: "multiply", label: t("inspector.mesh.blend.multiply") },
]);
const maskModeOptions = $derived<{ value: MaskMode; label: string }[]>([
  { value: "union", label: t("inspector.mesh.maskMode.union") },
  { value: "intersection", label: t("inspector.mesh.maskMode.intersection") },
]);
const maskOptions = $derived(
  maskCandidates.map((c) => ({ value: c.id, label: elementLabel(c.name, c.id) })),
);
const ids = $props.id();

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
    <h3>{t("inspector.mesh.title")}</h3>
    <p class="ed-hint">
      {t("inspector.mesh.summary", {
        name: mesh.name,
        vertices: t("inspector.mesh.vertices", { count: mesh.vertices }),
        triangles: t("inspector.mesh.triangles", { count: mesh.indices.length / 3 }),
      })}
    </p>
    <div class="ed-field">
      <span id="{ids}-texture">{t("inspector.mesh.texture.label")}</span>
      <Select
        aria-labelledby="{ids}-texture"
        value={model.textures[mesh.texture] ?? ""}
        options={textureOptions}
        onchange={(v) => set({ texture_name: v })}
      />
    </div>
    <div class="ed-field">
      <span id="{ids}-blend">{t("inspector.mesh.blend.label")}</span>
      <Select
        aria-labelledby="{ids}-blend"
        value={mesh.blend}
        options={blendOptions}
        onchange={(v) => set({ blend: v })}
      />
    </div>
    <div class="check">
      <Checkbox
        label={t("inspector.mesh.culling.label")}
        checked={mesh.culling}
        onchange={(v) => set({ culling: v })}
      />
    </div>
    <div class="check">
      <Checkbox
        label={t("inspector.mesh.maskOnly.label")}
        checked={mesh.maskOnly}
        onchange={(v) => set({ mask_only: v })}
      />
    </div>

    <div class="masks-head">
      <h4>{t("inspector.mesh.masks.title")}</h4>
      <Button small onclick={addMask} disabled={maskCandidates.length === 0}
        >{t("inspector.mesh.masks.add")}</Button
      >
    </div>
    {#if mesh.masks.length > 0}
      <div class="ed-field">
        <span id="{ids}-combine">{t("inspector.mesh.maskMode.label")}</span>
        <Select
          aria-labelledby="{ids}-combine"
          value={mesh.maskMode}
          options={maskModeOptions}
          onchange={(v) => set({ mask_mode: v })}
        />
      </div>
    {/if}
    <ul>
      {#each mesh.masks as mask, i (i)}
        <li class="mask">
          <Select
            aria-label={t("inspector.mesh.masks.mesh.aria", { index: i + 1 })}
            value={mask.mesh}
            options={maskOptions}
            onchange={(v) => setMask(i, { mesh: v })}
          />
          <Checkbox
            label={t("inspector.mesh.masks.invert")}
            checked={mask.invert}
            onchange={(v) => setMask(i, { invert: v })}
          />
          <IconButton small label={t("inspector.mesh.masks.remove")} onclick={() => removeMask(i)}
            ><Icon name="close" /></IconButton
          >
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
.check {
  margin: var(--ugoe-space-2) 0;
}

.masks-head,
.mask {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ugoe-space-3);
  margin: var(--ugoe-space-2) 0;
}

.mask :global(.ugoe-select) {
  min-width: 0;
  flex: 1 1 auto;
}

ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

h4 {
  margin: var(--ugoe-space-3) 0 var(--ugoe-space-2);
}
</style>
