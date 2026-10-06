<!--
  インスペクタ: 描画用の複製 (docs/editor.md#描画用の複製)。選択中のメッシュが複製なら元へ、元なら複製へ
  たどれるようにし、複製を作る・元に合わせ直す操作と、複製の今のセルの不透明度・描画順を出す
  (描画順のキーを作るため。すべてのセルをそろえるのは KeyformInspector)。マスクは MeshProperties で編集する。
-->
<script lang="ts">
import { Button, Notice, NumberField } from "@ugoe/ui";
import type { EditorDocument } from "../../../document.svelte";
import { t } from "../../../i18n";
import { elementLabel } from "../../../labels";
import type { Selection } from "../../../selection.svelte";
import TipButton from "../../../ui/TipButton.svelte";
import type { ViewportController } from "../../../viewport/controller.svelte";
import { cellIndex, currentCell } from "../keyforms/keyforms";
import { createCommand, drawCloneInfo, newClone } from "./drawClone";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
}

let { doc, selection, viewport }: Props = $props();

const model = $derived(doc.parsedModel);
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
const ids = $props.id();

function select(id: string): void {
  selection.set([{ kind: "mesh", id }]);
}

async function createClone(): Promise<void> {
  if (!mesh || !model) return;
  const source = mesh.id;
  const before = model;
  if (!(await doc.apply(createCommand(source)))) return;
  const after = doc.parsedModel;
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
    <h3>{t("inspector.drawClone.title")}</h3>
    {#if info.role === "clone"}
      <p class="row">
        <span>{t("inspector.drawClone.cloneOf")}</span>
        {#if info.original}
          <Button small onclick={() => info.original && select(info.original.id)}>
            {elementLabel(info.original.name, info.original.id)}
          </Button>
        {/if}
      </p>
      <p class="muted">{t("inspector.drawClone.description")}</p>
      {#if !info.inSync}
        <Notice tone="warning">
          {t("inspector.drawClone.outOfSync")}
          {#snippet actions()}
            <Button small onclick={sync}>{t("inspector.drawClone.resync")}</Button>
          {/snippet}
        </Notice>
      {/if}
      <div class="ed-field">
        <span id="{ids}-opacity">{t("inspector.drawClone.opacity")}</span>
        <NumberField
          aria-labelledby="{ids}-opacity"
          min={0}
          max={1}
          step={0.05}
          disabled={!keyform}
          value={keyform?.opacity ?? 0}
          onchange={(v) => setCell("opacity", v)}
        />
      </div>
      <div class="ed-field">
        <span id="{ids}-draw-order">{t("inspector.drawClone.drawOrder")}</span>
        <NumberField
          aria-labelledby="{ids}-draw-order"
          step={10}
          disabled={!keyform}
          value={keyform?.draw_order ?? 0}
          onchange={(v) => setCell("draw_order", v)}
        />
      </div>
      {#if !keyform}
        <p class="muted">{t("inspector.drawClone.betweenKeys")}</p>
      {/if}
    {:else}
      {#if info.clones.length > 0}
        <ul>
          {#each info.clones as clone (clone.id)}
            <li class="row">
              <span>{t("inspector.drawClone.clone")}</span>
              <Button small onclick={() => select(clone.id)}>{elementLabel(clone.name, clone.id)}</Button>
            </li>
          {/each}
        </ul>
      {/if}
      <TipButton small onclick={createClone} tooltip={t("inspector.drawClone.create.tooltip")}>
        {t("inspector.drawClone.create.label")}
      </TipButton>
    {/if}
  </section>
{/if}

<style>
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ugoe-space-3);
  margin: var(--ugoe-space-2) 0;
}

ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

h3 {
  margin: var(--ugoe-space-3) 0 var(--ugoe-space-2);
}
</style>
