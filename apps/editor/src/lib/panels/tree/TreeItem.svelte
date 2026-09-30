<!--
  ツリーの 1 行。状態 (選択・展開・名前の変更・ドロップの表示) は TreePanel が持ち、ここは表示と入力の中継だけ。
-->
<script lang="ts">
import type { TreeRow } from "./build";
import type { DropMark } from "./edit";
import NodeIcon from "./NodeIcon.svelte";

interface Props {
  row: TreeRow;
  id: string;
  selected: boolean;
  active: boolean;
  expanded: boolean;
  renaming: boolean;
  drop: DropMark;
  /** モジュールのプロジェクトで、本体の要素 (読み取り専用の下敷き)。 */
  base?: boolean;
  onselect: (e: MouseEvent) => void;
  ontoggle: () => void;
  onrenamestart: () => void;
  onrename: (name: string | null) => void;
  onpointerdown: (e: PointerEvent) => void;
}

let {
  row,
  id,
  selected,
  active,
  expanded,
  renaming,
  drop,
  base = false,
  onselect,
  ontoggle,
  onrenamestart,
  onrename,
  onpointerdown,
}: Props = $props();

const node = $derived(row.node);
const hasChildren = $derived(node.children.length > 0);
let done = false;

function focusInput(el: HTMLInputElement) {
  done = false;
  el.focus();
  el.select();
}

function finish(name: string | null) {
  // Enter の後の blur で 2 回呼ばれないようにする。
  if (done) return;
  done = true;
  onrename(name);
}

function onKeydown(e: KeyboardEvent) {
  e.stopPropagation();
  if (e.key === "Enter") finish((e.currentTarget as HTMLInputElement).value);
  else if (e.key === "Escape") finish(null);
}
</script>

<!-- キーボード操作は親の tree (aria-activedescendant) が受け持つので、行自身はキーを受けない。 -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  {id}
  class="row"
  class:selected
  class:active
  class:hidden={node.hidden}
  class:mask-only={node.maskOnly}
  class:base
  class:drop-into={drop?.position === "into"}
  class:drop-above={drop?.position === "above"}
  class:drop-below={drop?.position === "below"}
  class:drop-bad={drop && !drop.ok}
  role="treeitem"
  tabindex="-1"
  aria-selected={selected}
  aria-level={row.depth + 1}
  aria-expanded={hasChildren ? expanded : undefined}
  data-key={node.key}
  style:padding-left="{row.depth * 14 + 2}px"
  onclick={onselect}
  ondblclick={onrenamestart}
  {onpointerdown}
>
  {#if hasChildren}
    <button
      class="twisty"
      tabindex="-1"
      aria-label={expanded ? "Collapse" : "Expand"}
      onclick={(e) => {
        e.stopPropagation();
        ontoggle();
      }}
      ondblclick={(e) => e.stopPropagation()}
      onpointerdown={(e) => e.stopPropagation()}
    >{expanded ? "▾" : "▸"}</button>
  {:else}
    <span class="twisty"></span>
  {/if}
  <NodeIcon icon={node.icon} />
  {#if renaming}
    <input
      class="rename"
      value={node.name}
      aria-label="Name"
      use:focusInput
      onkeydown={onKeydown}
      onblur={(e) => finish(e.currentTarget.value)}
      onclick={(e) => e.stopPropagation()}
      onpointerdown={(e) => e.stopPropagation()}
    />
  {:else}
    <span class="name" title={node.ref.id}>{node.name}</span>
  {/if}
  {#if base}<span class="badge" title="Part of the base model: read-only in a module project">base</span>{/if}
  {#if node.maskOnly}<span class="badge" title="Mask only: not drawn as color">mask</span>{/if}
  {#if node.hidden}<span class="badge" title="Opacity is 0 at the default parameter values">hidden</span>{/if}
  {#if node.drawOrder !== undefined}<span class="order" title="Draw order">{node.drawOrder}</span>{/if}
</div>

<style>
.row {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 22px;
  padding-right: 4px;
  border-radius: 3px;
  border: 1px solid transparent;
  cursor: default;
  user-select: none;
  white-space: nowrap;
}
.row:hover {
  background: color-mix(in srgb, var(--accent) 8%, transparent);
}
.row.selected {
  background: color-mix(in srgb, var(--accent) 22%, transparent);
}
.row.active {
  border-color: color-mix(in srgb, var(--accent) 60%, transparent);
}
.row.base .name {
  opacity: 0.55;
}
.row.hidden .name {
  opacity: 0.5;
}
.row.mask-only .name {
  font-style: italic;
}
.row.drop-into {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 15%, transparent);
}
.row.drop-above {
  box-shadow: inset 0 2px 0 var(--accent);
}
.row.drop-below {
  box-shadow: inset 0 -2px 0 var(--accent);
}
.row.drop-bad {
  border-color: #dc2626;
  box-shadow: none;
  background: color-mix(in srgb, #dc2626 10%, transparent);
}
.twisty {
  flex: none;
  width: 14px;
  padding: 0;
  border: none;
  background: none;
  color: var(--muted);
  font-size: 10px;
  text-align: center;
}
.name {
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}
.rename {
  flex: 1;
  min-width: 0;
  font: inherit;
  padding: 0 2px;
}
.badge,
.order {
  flex: none;
  font-size: 10px;
  color: var(--muted);
}
.badge {
  padding: 0 3px;
  border: 1px solid var(--border);
  border-radius: 3px;
}
</style>
