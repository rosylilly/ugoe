<!--
  ツリーの 1 行。状態 (選択・展開・名前の変更・ドロップの表示) は TreePanel が持ち、ここは表示と入力の中継だけ。
-->
<script lang="ts">
import { Icon } from "@ugoe/ui";
import { t } from "../../i18n";
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
  style:padding-left="{row.depth * 14 + 7}px"
  onclick={onselect}
  ondblclick={onrenamestart}
  {onpointerdown}
>
  {#if hasChildren}
    <button
      class="twisty"
      tabindex="-1"
      aria-label={expanded ? t("tree.row.collapse") : t("tree.row.expand")}
      onclick={(e) => {
        e.stopPropagation();
        ontoggle();
      }}
      ondblclick={(e) => e.stopPropagation()}
      onpointerdown={(e) => e.stopPropagation()}
    ><Icon name={expanded ? "chevronDown" : "chevronRight"} /></button>
  {:else}
    <span class="twisty"></span>
  {/if}
  <NodeIcon icon={node.icon} />
  {#if renaming}
    <input
      class="rename"
      value={node.name}
      aria-label={t("tree.row.name")}
      use:focusInput
      onkeydown={onKeydown}
      onblur={(e) => finish(e.currentTarget.value)}
      onclick={(e) => e.stopPropagation()}
      onpointerdown={(e) => e.stopPropagation()}
    />
  {:else}
    <span class="name" title={node.ref.id}>{node.name}</span>
  {/if}
  <!-- 行はフォーカスを受けない (木が aria-activedescendant で受ける) ので、印の説明は Tooltip ではなく title で出す。 -->
  {#if base}<span class="badge" title={t("tree.row.base.tooltip")}>{t("tree.row.base.label")}</span>{/if}
  {#if node.maskOnly}<span class="badge" title={t("tree.row.mask.tooltip")}>{t("tree.row.mask.label")}</span>{/if}
  {#if node.hidden}<span class="badge" title={t("tree.row.hidden.tooltip")}>{t("tree.row.hidden.label")}</span>{/if}
  {#if node.drawOrder !== undefined}<span class="order" title={t("tree.row.drawOrder")}>{node.drawOrder}</span>{/if}
</div>

<style>
.row {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
  height: var(--ugoe-target-min);
  padding-right: var(--ugoe-space-2);
  border-radius: var(--ugoe-radius-sm);
  border: 1px solid transparent;
  cursor: default;
  user-select: none;
  white-space: nowrap;
}
.row:hover {
  background: color-mix(in srgb, var(--ugoe-color-accent) 8%, transparent);
}
.row.selected {
  background: color-mix(in srgb, var(--ugoe-color-accent) 22%, transparent);
}
.row.active {
  border-color: color-mix(in srgb, var(--ugoe-color-accent) 60%, transparent);
}
.row.base .name,
.row.hidden .name {
  /* 薄くするのは不透明度ではなく控えめな文字の色で (opacity では 4.5:1 に届かない。T0907) */
  color: var(--ugoe-color-text-muted);
}
.row.mask-only .name {
  font-style: italic;
}
.row.drop-into {
  border-color: var(--ugoe-color-accent);
  background: color-mix(in srgb, var(--ugoe-color-accent) 15%, transparent);
}
.row.drop-above {
  box-shadow: inset 0 2px 0 var(--ugoe-color-accent);
}
.row.drop-below {
  box-shadow: inset 0 -2px 0 var(--ugoe-color-accent);
}
.row.drop-bad {
  border-color: var(--ugoe-color-danger);
  box-shadow: none;
  background: color-mix(in srgb, var(--ugoe-color-danger) 10%, transparent);
}
/* 開閉の印: 並びの幅は 14px のまま、押せる範囲を 24×24 に広げる (左右に 5px ずつはみ出す。T0907) */
.twisty {
  flex: none;
  width: var(--ugoe-target-min);
  height: var(--ugoe-target-min);
  margin-inline: -5px;
  padding: 0;
  border: none;
  background: none;
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-xs);
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
  padding: 0 var(--ugoe-space-1);
}
.badge,
.order {
  flex: none;
  font-size: var(--ugoe-font-size-xs);
  color: var(--ugoe-color-text-muted);
}
.badge {
  padding: 0 var(--ugoe-space-1);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-sm);
}
</style>
