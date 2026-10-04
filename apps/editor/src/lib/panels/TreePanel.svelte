<!--
  左パネル: パーツ / デフォーマのツリー (docs/editor.md#画面構成-m2-の目標)。
  選択は Selection (lib/selection.svelte.ts)、編集は EditorDocument.apply(Command) で行う。
  ツリーの組み立てとコマンドへの変換は tree/ の純粋な TS (build.ts / edit.ts) に置く。
  デフォーマ・メッシュの親の付け替えは、見た目を保つ reparent_preserving_look で送る (tree/edit.ts)。
-->
<script lang="ts">
import { AlertDialog, Notice } from "@ugoe/ui";
import { tick, untrack } from "svelte";
import type { EditorDocument } from "../document.svelte";
import { emptyTreeHint } from "../emptyState";
import { t } from "../i18n";
import type { EditCommand } from "../ipc";
import { baseElements, isBaseElement } from "../moduleProject";
import type { ElementRef, Selection } from "../selection.svelte";
import TabStrip from "../ui/TabStrip.svelte";
import TipButton from "../ui/TipButton.svelte";
import {
  buildTree,
  exists,
  flatten,
  indexModel,
  pathTo,
  refKey,
  type TreeRow,
  type TreeView,
} from "./tree/build";
import {
  addPartCommand,
  type DropMark,
  type DropTarget,
  deleteCommand,
  dropCommand,
  type EditResult,
  parentPartFor,
  renameCommand,
} from "./tree/edit";
import { parseTreeModel } from "./tree/model";
import TreeItem from "./tree/TreeItem.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
}

let { doc, selection }: Props = $props();

let view = $state<TreeView>("parts");
let collapsed = $state(new Set<string>());
let renaming = $state<string | null>(null);
/** キーボード操作の起点の行 (最後にクリック・移動した行)。 */
let cursor = $state<string | null>(null);
let message = $state<{ text: string; error: boolean } | null>(null);
let confirmDelete = $state<ElementRef[] | null>(null);
let treeEl = $state<HTMLElement>();

const index = $derived.by(() => {
  if (!doc.model) return null;
  try {
    return indexModel(parseTreeModel(doc.model.json));
  } catch (e) {
    console.error("ugoe: cannot read the model for the tree:", e);
    return null;
  }
});
const roots = $derived(index ? buildTree(index, view) : []);
const rows = $derived(flatten(roots, collapsed));
const rowIndex = $derived(new Map(rows.map((r, i) => [r.node.key, i])));
const selectedKeys = $derived(new Set(selection.items.map(refKey)));
const rowId = (key: string) => `tree-row-${key}`;
/** モジュールのプロジェクトで、本体の要素 (読み取り専用)。 */
const base = $derived(baseElements(doc.module));
const isBase = (ref: ElementRef) => isBaseElement(base, ref);

// ドキュメントを読み直したら、消えた要素を選択から外す。
$effect(() => {
  const i = index;
  if (i) untrack(() => selection.retain((r) => exists(i, r)));
});

// 外 (ビューポートなど) で選択された要素を、祖先を開いて見える位置までスクロールする。
$effect(() => {
  const primary = selection.primary;
  const tree = roots;
  if (!primary) return;
  const key = refKey(primary);
  untrack(() => {
    const path = pathTo(tree, key);
    if (!path) return;
    if (path.some((k) => collapsed.has(k))) {
      const next = new Set(collapsed);
      for (const k of path) next.delete(k);
      collapsed = next;
    }
    if (!selectedKeys.has(cursor ?? "")) cursor = key;
    tick().then(() => {
      document.getElementById(rowId(key))?.scrollIntoView({ block: "nearest" });
    });
  });
});

async function run(command: EditCommand | null): Promise<boolean> {
  if (!command) return false;
  await doc.apply(command);
  // apply は失敗を doc.error に残し、成功したら読み直して消す。
  message = doc.error ? { text: doc.error, error: true } : null;
  return !doc.error;
}

function report(r: EditResult): EditCommand | null {
  if (r.ok) return r.command;
  if (!r.noop) message = { text: r.reason, error: true };
  return null;
}

function toggle(key: string) {
  const next = new Set(collapsed);
  if (!next.delete(key)) next.add(key);
  collapsed = next;
}

function select(row: TreeRow, additive: boolean) {
  cursor = row.node.key;
  if (additive) selection.toggle(row.node.ref);
  else selection.set([row.node.ref]);
}

function onRowClick(row: TreeRow, e: MouseEvent) {
  if (suppressClick) {
    suppressClick = false;
    return;
  }
  select(row, e.shiftKey || e.ctrlKey || e.metaKey);
  treeEl?.focus();
}

async function rename(ref: ElementRef, name: string | null) {
  renaming = null;
  treeEl?.focus();
  const trimmed = name?.trim();
  if (!trimmed || !index) return;
  const current =
    ref.kind === "part"
      ? index.parts.get(ref.id)?.name
      : ref.kind === "deformer"
        ? index.deformers.get(ref.id)?.name
        : index.meshes.get(ref.id)?.name;
  if (trimmed === current) return;
  await run(renameCommand(ref, trimmed));
}

async function addPart() {
  if (!index) return;
  const command = addPartCommand(index, parentPartFor(index, selection.primary));
  if (command.op !== "add_part") return;
  if (await run(command)) {
    const ref: ElementRef = { kind: "part", id: command.part.id };
    view = "parts";
    selection.set([ref]);
    cursor = refKey(ref);
    await tick();
    renaming = cursor;
  }
}

function askDelete() {
  const refs = selection.items.filter((r) => r.kind !== "parameter");
  if (refs.length > 0) confirmDelete = refs;
}

/** 消す確かめの文 (1 つなら種類と名前、複数なら数)。 */
const deleteDescription = $derived.by(() => {
  const refs = confirmDelete;
  if (!refs) return "";
  const [first] = refs;
  if (refs.length === 1 && first) {
    return t("tree.delete.one", { kind: kindLabel(first.kind), id: first.id });
  }
  return t("tree.delete.many", { count: refs.length });
});

function kindLabel(kind: ElementRef["kind"]): string {
  return {
    part: t("tree.kind.part"),
    deformer: t("tree.kind.deformer"),
    mesh: t("tree.kind.mesh"),
    parameter: t("tree.kind.parameter"),
  }[kind];
}

const viewTabs = $derived([
  { value: "parts" as const, label: t("tree.view.parts") },
  { value: "deformers" as const, label: t("tree.view.deformers") },
]);

async function doDelete() {
  const refs = confirmDelete;
  confirmDelete = null;
  treeEl?.focus();
  if (!refs || !index) return;
  await run(deleteCommand(index, refs));
}

// ---- キーボード ----

function moveTo(i: number, extend: boolean) {
  const row = rows[Math.max(0, Math.min(rows.length - 1, i))];
  if (!row) return;
  cursor = row.node.key;
  if (extend) {
    if (!selection.has(row.node.ref)) selection.toggle(row.node.ref);
  } else {
    selection.set([row.node.ref]);
  }
  document.getElementById(rowId(row.node.key))?.scrollIntoView({ block: "nearest" });
}

function onKeydown(e: KeyboardEvent) {
  if (renaming) return;
  const at = cursor !== null ? (rowIndex.get(cursor) ?? -1) : -1;
  const row = rows[at];
  switch (e.key) {
    case "ArrowDown":
      moveTo(at + 1, e.shiftKey);
      break;
    case "ArrowUp":
      moveTo(at < 0 ? 0 : at - 1, e.shiftKey);
      break;
    case "Home":
      moveTo(0, e.shiftKey);
      break;
    case "End":
      moveTo(rows.length - 1, e.shiftKey);
      break;
    case "ArrowRight":
      if (!row) moveTo(0, false);
      else if (row.node.children.length > 0 && collapsed.has(row.node.key)) toggle(row.node.key);
      else if (row.node.children.length > 0) moveTo(at + 1, false);
      break;
    case "ArrowLeft":
      if (!row) break;
      if (row.node.children.length > 0 && !collapsed.has(row.node.key)) toggle(row.node.key);
      else if (row.parent !== null) moveTo(rowIndex.get(row.parent) ?? at, false);
      break;
    case "Enter":
    case "F2":
      if (row) renaming = row.node.key;
      break;
    case "Delete":
    case "Backspace":
      askDelete();
      break;
    case "Escape":
      message = null;
      break;
    default:
      return;
  }
  e.preventDefault();
}

// ---- ドラッグ & ドロップ ----
// Tauri の WebView はファイルのドロップのために HTML5 の DnD を横取りすることがあるので、ポインタで実装する。

let press: { row: TreeRow; x: number; y: number; pointer: number } | null = null;
let drag = $state<{
  items: ElementRef[];
  over: string | null;
  drop: DropTarget | null;
  result: EditResult | null;
} | null>(null);
let suppressClick = false;

function onRowPointerDown(row: TreeRow, e: PointerEvent) {
  if (e.button !== 0 || renaming) return;
  press = { row, x: e.clientX, y: e.clientY, pointer: e.pointerId };
}

function startDrag(row: TreeRow) {
  const ref = row.node.ref;
  const items = selection.has(ref) ? selection.items.filter((r) => r.kind !== "parameter") : [ref];
  drag = { items, over: null, drop: null, result: null };
  message = null;
}

/** ポインタの下の行と、その行の中の位置から落とし先を決める。 */
function dropAt(x: number, y: number): { over: string | null; drop: DropTarget } | null {
  const el = document.elementFromPoint(x, y);
  if (!el || !treeEl?.contains(el)) return null;
  const rowEl = el.closest<HTMLElement>("[data-key]");
  const key = rowEl?.dataset.key;
  const row = key !== undefined ? rows[rowIndex.get(key) ?? -1] : undefined;
  if (!rowEl || !row) return { over: null, drop: { target: null, position: "into" } };
  const { ref } = row.node;
  if (ref.kind !== "mesh") return { over: row.node.key, drop: { target: ref, position: "into" } };
  const rect = rowEl.getBoundingClientRect();
  const parentRow = row.parent !== null ? rows[rowIndex.get(row.parent) ?? -1] : undefined;
  return {
    over: row.node.key,
    drop: {
      target: ref,
      position: y < rect.top + rect.height / 2 ? "above" : "below",
      container: parentRow?.node.ref ?? null,
    },
  };
}

function onPointerMove(e: PointerEvent) {
  if (!press || e.pointerId !== press.pointer) return;
  if (!drag) {
    if (Math.hypot(e.clientX - press.x, e.clientY - press.y) < 5) return;
    startDrag(press.row);
    treeEl?.setPointerCapture(e.pointerId);
  }
  if (!drag || !index) return;
  const at = dropAt(e.clientX, e.clientY);
  drag.over = at?.over ?? null;
  drag.drop = at?.drop ?? null;
  drag.result = at ? dropCommand(index, view, drag.items, at.drop) : null;
}

async function onPointerUp(e: PointerEvent) {
  if (!press || e.pointerId !== press.pointer) return;
  press = null;
  const d = drag;
  if (!d) return;
  drag = null;
  suppressClick = true;
  // クリックが発生しない場合 (ポインタを取り込んだ要素の外で離したとき) に備えて戻す。
  setTimeout(() => {
    suppressClick = false;
  });
  if (d.result) await run(report(d.result));
}

function cancelDrag() {
  press = null;
  drag = null;
}

function dropMark(key: string): DropMark {
  if (!drag || drag.over !== key || !drag.drop || !drag.result) return null;
  if (!drag.result.ok && drag.result.noop) return null;
  return { position: drag.drop.position, ok: drag.result.ok };
}

const rootDrop = $derived(
  drag !== null &&
    drag.over === null &&
    drag.drop !== null &&
    drag.result !== null &&
    !(!drag.result.ok && drag.result.noop),
);
const dragHint = $derived.by(() => {
  const r = drag?.result;
  if (!r) return drag ? t("tree.drop.hint") : null;
  return r.ok ? null : r.noop ? null : r.reason;
});
</script>

<div class="panel">
  <TabStrip
    tabs={viewTabs}
    value={view}
    ariaLabel={t("tree.view.aria")}
    onselect={(v) => (view = v)}
  />
  <div class="toolbar">
    <TipButton small onclick={addPart} disabled={!index} tooltip={t("tree.addPart.tooltip")}
      >{t("tree.addPart.label")}</TipButton
    >
    <TipButton
      small
      onclick={() => {
        if (cursor && rowIndex.has(cursor)) renaming = cursor;
      }}
      disabled={!cursor || !rowIndex.has(cursor)}
      tooltip={t("tree.rename.tooltip")}
      shortcut="F2">{t("tree.rename.label")}</TipButton
    >
    <TipButton
      small
      onclick={askDelete}
      disabled={selection.items.length === 0}
      tooltip={t("tree.delete.tooltip")}
      shortcut="Delete">{t("tree.delete.label")}</TipButton
    >
  </div>

  <div
    class="tree"
    class:drop-root={rootDrop}
    class:dragging={drag !== null}
    role="tree"
    aria-label={view === "parts" ? t("tree.view.parts") : t("tree.view.deformers")}
    aria-multiselectable="true"
    aria-activedescendant={cursor && rowIndex.has(cursor) ? rowId(cursor) : undefined}
    tabindex="0"
    bind:this={treeEl}
    onkeydown={onKeydown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={cancelDrag}
    onlostpointercapture={() => {
      if (drag) cancelDrag();
    }}
  >
    {#each rows as row (row.node.key)}
      <TreeItem
        {row}
        id={rowId(row.node.key)}
        selected={selectedKeys.has(row.node.key)}
        active={cursor === row.node.key}
        expanded={!collapsed.has(row.node.key)}
        renaming={renaming === row.node.key}
        drop={dropMark(row.node.key)}
        base={isBase(row.node.ref)}
        onselect={(e) => onRowClick(row, e)}
        ontoggle={() => toggle(row.node.key)}
        onrenamestart={() => (renaming = row.node.key)}
        onrename={(name) => rename(row.node.ref, name)}
        onpointerdown={(e) => onRowPointerDown(row, e)}
      />
    {:else}
      <p class="ed-hint empty">
        {!index
          ? t("tree.empty.noModel")
          : doc.empty
            ? emptyTreeHint()
            : view === "parts"
              ? t("tree.empty.parts")
              : t("tree.empty.deformers")}
      </p>
    {/each}
  </div>
  <!-- メッセージは木の下に出す (上に出すと行がずれ、ドラッグ中の落とし先が変わるため)。 -->
  <div class="footer">
    {#if message}
      <Notice tone={message.error ? "danger" : "info"} ondismiss={() => (message = null)}>
        {message.text}
      </Notice>
    {/if}
    {#if dragHint}<p class="hint" class:error={drag?.result && !drag.result.ok} class:ugoe-selectable={drag?.result && !drag.result.ok}>{dragHint}</p>{/if}
  </div>
</div>

<AlertDialog
  open={confirmDelete !== null}
  title={t("tree.delete.title")}
  description={deleteDescription}
  confirmLabel={t("tree.delete.confirm")}
  onconfirm={doDelete}
  oncancel={() => {
    confirmDelete = null;
    treeEl?.focus();
  }}
/>

<style>
.panel {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-height: 100%;
}
.toolbar {
  display: flex;
  gap: 4px;
}
.footer {
  position: sticky;
  bottom: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: var(--ugoe-color-surface);
}
.tree {
  flex: 1;
  min-height: 80px;
  outline: none;
  border: 1px solid transparent;
  border-radius: 4px;
}
.tree:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: -2px;
}
.tree.dragging {
  cursor: grabbing;
}
.tree.drop-root {
  border-color: var(--ugoe-color-accent);
  border-style: dashed;
}
.empty {
  padding: 4px;
}
.hint {
  margin: 0;
  font-size: var(--ugoe-font-size-sm);
  color: var(--ugoe-color-text-muted);
}
.hint.error {
  color: var(--ugoe-color-danger);
}
</style>
