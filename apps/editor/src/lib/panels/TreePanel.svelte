<!--
  左パネル: パーツ / デフォーマのツリー (docs/editor.md#画面構成)。
  選択は Selection (lib/selection.svelte.ts)、編集は EditorDocument.apply(Command) で行う。
  ツリーの組み立てとコマンドへの変換は tree/ の純粋な TS (build.ts / edit.ts) に置く。
  親の付け替えは生のコマンドを送るだけで、見た目は保たない (見た目を保つ付け替えは T0607)。
-->
<script lang="ts">
import { tick, untrack } from "svelte";
import type { EditorDocument } from "../document.svelte";
import { EMPTY_TREE_HINT } from "../emptyState";
import type { EditCommand } from "../ipc";
import type { ElementRef, Selection } from "../selection.svelte";
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
  if (!r) return drag ? "Drop onto a part, a deformer or between meshes" : null;
  return r.ok ? null : r.noop ? null : r.reason;
});
</script>

<div class="panel">
  <div class="tabs" role="tablist" aria-label="Tree">
    <button role="tab" aria-selected={view === "parts"} onclick={() => (view = "parts")}>Parts</button>
    <button role="tab" aria-selected={view === "deformers"} onclick={() => (view = "deformers")}>
      Deformers
    </button>
  </div>
  <div class="toolbar">
    <button onclick={addPart} disabled={!index} title="Add a part under the selected part">+ Part</button>
    <button
      onclick={() => {
        if (cursor && rowIndex.has(cursor)) renaming = cursor;
      }}
      disabled={!cursor || !rowIndex.has(cursor)}
      title="Rename (F2)">Rename</button>
    <button onclick={askDelete} disabled={selection.items.length === 0} title="Delete (Del)">Delete</button>
  </div>

  <div
    class="tree"
    class:drop-root={rootDrop}
    class:dragging={drag !== null}
    role="tree"
    aria-label={view === "parts" ? "Parts" : "Deformers"}
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
        onselect={(e) => onRowClick(row, e)}
        ontoggle={() => toggle(row.node.key)}
        onrenamestart={() => (renaming = row.node.key)}
        onrename={(name) => rename(row.node.ref, name)}
        onpointerdown={(e) => onRowPointerDown(row, e)}
      />
    {:else}
      <p class="muted empty">
        {!index
          ? "No model"
          : doc.empty
            ? EMPTY_TREE_HINT
            : view === "parts"
              ? "No parts"
              : "No deformers or meshes"}
      </p>
    {/each}
  </div>
  <!-- メッセージは木の下に出す (上に出すと行がずれ、ドラッグ中の落とし先が変わるため)。 -->
  <div class="footer">
    {#if confirmDelete}
      <div class="confirm" role="alertdialog" aria-label="Confirm delete">
        <span>
          Delete {confirmDelete.length === 1
            ? `${confirmDelete[0]?.kind} "${confirmDelete[0]?.id}"`
            : `${confirmDelete.length} elements`}?
        </span>
        <!-- svelte-ignore a11y_autofocus -->
        <button class="danger" autofocus onclick={doDelete}>Delete</button>
        <button onclick={() => (confirmDelete = null)}>Cancel</button>
      </div>
    {/if}
    {#if message}
      <p class:error={message.error} class="message" role="status">
        {message.text}
        <button class="dismiss" aria-label="Dismiss" onclick={() => (message = null)}>×</button>
      </p>
    {/if}
    {#if dragHint}<p class="hint" class:error={drag?.result && !drag.result.ok}>{dragHint}</p>{/if}
  </div>
</div>

<style>
.panel {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-height: 100%;
}
.tabs {
  display: flex;
  border-bottom: 1px solid var(--border);
}
.tabs button {
  flex: 1;
  padding: 4px;
  border: none;
  border-bottom: 2px solid transparent;
  background: none;
  color: var(--muted);
  font: inherit;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
.tabs button[aria-selected="true"] {
  color: var(--text);
  border-bottom-color: var(--accent);
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
  background: var(--panel);
}
.confirm {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  padding: 6px;
  border: 1px solid #dc2626;
  border-radius: 4px;
}
.confirm span {
  flex-basis: 100%;
}
.danger {
  color: #dc2626;
}
.message {
  display: flex;
  gap: 4px;
  align-items: flex-start;
  margin: 0;
  word-break: break-word;
}
.dismiss {
  margin-left: auto;
  padding: 0 4px;
  border: none;
  background: none;
  color: inherit;
}
.tree {
  flex: 1;
  min-height: 80px;
  outline: none;
  border: 1px solid transparent;
  border-radius: 4px;
}
.tree:focus-visible {
  border-color: color-mix(in srgb, var(--accent) 50%, transparent);
}
.tree.dragging {
  cursor: grabbing;
}
.tree.drop-root {
  border-color: var(--accent);
  border-style: dashed;
}
.empty {
  padding: 4px;
}
.hint {
  margin: 0;
  font-size: 11px;
  color: var(--muted);
}
.hint.error {
  color: #dc2626;
}
</style>
