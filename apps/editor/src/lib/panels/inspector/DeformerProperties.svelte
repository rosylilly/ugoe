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
import { formatList } from "@ugoe/i18n";
import { Button, NumberField, TextField } from "@ugoe/ui";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { ChildRef } from "../../ipc";
import { labelOf } from "../../labels";
import type { Selection } from "../../selection.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  /**
   * 出す節: 選んだデフォーマの概要 (`summary`。インスペクタの頭) か、選んだものを包む操作 (`wrap`。キーフォームの後)。
   * インスペクタはよく触る欄を上に並べる (docs/editor.md#インスペクタの並び)。
   */
  show: "summary" | "wrap";
}

let { doc, selection, show }: Props = $props();

/** 行数・列数の上限 (ugoe_editor_core の MAX_WARP_DIVISIONS)。 */
const MAX_DIVISIONS = 64;

let rows = $state(3);
let cols = $state(3);
/** 作るデフォーマの名前 (空なら子の名前から作る)。作るたびに空に戻す。 */
let name = $state("");
/** 編集を送っている間はボタンを無効にする (docs/editor.md#パネルからの編集)。 */
const busy = $derived(doc.pending > 0);

const model = $derived(doc.parsedModel);

const children = $derived(
  selection.items.flatMap((r): ChildRef[] =>
    r.kind === "mesh" || r.kind === "deformer" ? [{ kind: r.kind, id: r.id }] : [],
  ),
);

const parts = $derived(selection.items.filter((r) => r.kind === "part").map((r) => r.id));

/** 包むものの説明 (`2 meshes, 1 part`)。 */
const summary = $derived.by(() => {
  const meshes = children.filter((c) => c.kind === "mesh").length;
  const deformers = children.length - meshes;
  const items: string[] = [];
  if (meshes > 0) items.push(t("inspector.deformer.wrap.meshes", { count: meshes }));
  if (deformers > 0) items.push(t("inspector.deformer.wrap.deformers", { count: deformers }));
  if (parts.length > 0) items.push(t("inspector.deformer.wrap.parts", { count: parts.length }));
  return formatList(items, { type: "unit" });
});

const ids = $props.id();

const deformer = $derived(
  selection.primary?.kind === "deformer"
    ? (model?.deformers.find((d) => d.id === selection.primary?.id) ?? null)
    : null,
);

const divisionsValid = $derived(
  [rows, cols].every((n) => Number.isInteger(n) && n >= 1 && n <= MAX_DIVISIONS),
);

async function create(kind: "warp" | "rotation"): Promise<void> {
  if (children.length === 0 && parts.length === 0) return;
  const common = { name: name.trim(), children, parts };
  const [r, c] = [rows, cols];
  let before = new Set<string>();
  const report = await doc.send(() => {
    // 前の編集で増えたデフォーマを、作ったものと取り違えないよう送る直前に数える。
    before = new Set(model?.deformers.map((d) => d.id));
    return kind === "warp"
      ? { op: "create_warp_deformer", ...common, rows: r, cols: c }
      : { op: "create_rotation_deformer", ...common };
  });
  if (!report) return;
  name = "";
  const created = model?.deformers.find((d) => !before.has(d.id));
  if (created) selection.set([{ kind: "deformer", id: created.id }]);
}

async function unwrap(id: string): Promise<void> {
  if (!model) return;
  let kids: ChildRef[] = [];
  const report = await doc.send(() => {
    if (!model) return null;
    // 外した後に選ぶ子は、送る直前のモデルから集める。
    kids = [
      ...model.deformers
        .filter((d) => d.parent === id)
        .map((d) => ({ kind: "deformer" as const, id: d.id })),
      ...model.meshes
        .filter((m) => m.parent === id)
        .map((m) => ({ kind: "mesh" as const, id: m.id })),
    ];
    return { op: "unwrap_deformer", id };
  });
  if (report) selection.set(kids);
}
</script>

{#if show === "summary" && deformer}
  <section>
    <h3>{t("inspector.deformer.title")}</h3>
    <p class="muted">
      {#if deformer.kind.type === "warp"}
        {t("inspector.deformer.summary.warp", {
          name: deformer.name,
          cols: deformer.kind.cols,
          rows: deformer.kind.rows,
          keyforms: t("inspector.deformer.keyforms", {
            count: deformer.kind.keyforms.keyforms.length,
          }),
          parent: deformer.parent
            ? labelOf(model?.deformers, deformer.parent)
            : t("inspector.deformer.noParent"),
        })}
      {:else}
        {t("inspector.deformer.summary.rotation", {
          name: deformer.name,
          keyforms: t("inspector.deformer.keyforms", {
            count: deformer.kind.keyforms.keyforms.length,
          }),
          parent: deformer.parent
            ? labelOf(model?.deformers, deformer.parent)
            : t("inspector.deformer.noParent"),
        })}
      {/if}
    </p>
    <p class="muted hint">{t("inspector.deformer.gizmoHint")}</p>
    <div class="row">
      <span class="muted hint">{t("inspector.deformer.unwrap.description")}</span>
      <Button small disabled={busy} onclick={() => unwrap(deformer.id)}
        >{t("inspector.deformer.unwrap.label")}</Button
      >
    </div>
  </section>
{/if}

{#if show === "wrap" && (children.length > 0 || parts.length > 0)}
  <section>
    <h3>{t("inspector.deformer.wrap.title")}</h3>
    <p class="muted hint">
      {parts.length > 0
        ? t("inspector.deformer.wrap.summaryWithParts", { items: summary })
        : t("inspector.deformer.wrap.summary", { items: summary })}
    </p>
    <div class="ed-field">
      <label for="{ids}-name">{t("inspector.deformer.wrap.name.label")}</label>
      <TextField id="{ids}-name" bind:value={name} placeholder={t("inspector.deformer.wrap.name.placeholder")} />
    </div>
    <div class="row">
      <NumberField label={t("inspector.deformer.wrap.rows")} min={1} max={MAX_DIVISIONS} size={3} bind:value={rows} />
      <NumberField label={t("inspector.deformer.wrap.cols")} min={1} max={MAX_DIVISIONS} size={3} bind:value={cols} />
      <Button small disabled={busy || !divisionsValid} onclick={() => create("warp")}
        >{t("inspector.deformer.wrap.warp")}</Button
      >
    </div>
    <div class="row">
      <span class="muted hint">{t("inspector.deformer.wrap.rotationOrigin")}</span>
      <Button small disabled={busy} onclick={() => create("rotation")}
        >{t("inspector.deformer.wrap.rotation")}</Button
      >
    </div>
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

.row :global(.ugoe-button) {
  flex-shrink: 0;
}

h3 {
  margin: var(--ugoe-space-3) 0 var(--ugoe-space-2);
}

.hint {
  font-size: var(--ugoe-font-size-sm);
}
</style>
