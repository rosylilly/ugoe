<!--
  ツリーの 1 行。状態 (選択・展開・名前の変更・ドロップの表示) は TreePanel が持ち、ここは表示と入力の中継だけ。
-->
<script lang="ts">
import { Icon, TextField } from "@ugoe/ui";
import { t } from "../../i18n";
import type { TreeRow } from "./build";
import type { DropMark } from "./edit";
import NodeIcon from "./NodeIcon.svelte";

interface Props {
  row: TreeRow;
  id: string;
  selected: boolean;
  /** 主な選択 (インスペクタに出すもの。ビューポートでは太い輪郭)。 */
  primary?: boolean;
  /** ポインタが乗っている (この行かビューポートのメッシュに)。 */
  hovered?: boolean;
  /** ポインタが乗っている要素 (ビューポートのメッシュ) が、畳んだこの行の下にある。 */
  hoveredWithin?: boolean;
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
  /** ポインタが行に乗った・離れた。 */
  onhover?: (on: boolean) => void;
}

let {
  row,
  id,
  selected,
  primary = false,
  hovered = false,
  hoveredWithin = false,
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
  onhover,
}: Props = $props();

const node = $derived(row.node);
/**
 * 行の札 (本体・マスク・非表示)。札が 2 つ以上並ぶと名前が 1 文字まで縮むので、見せる札は最後の 1 つ (非表示 > マスク > 本体) にし、
 * ほかの札は文字を見せずにその札の中で読ませる。まとめた本体は札の破線の枠、まとめたマスクは行の印と斜体の名前で見える
 * (docs/ui.md#エディタのツリーの札と選んだ行-t1654 と docs/ui.md#エディタのツリーの行の札と強制色-t1670)。
 */
const tag = $derived.by(() => {
  const list: { label: string; tooltip: string }[] = [];
  if (base) list.push({ label: t("tree.row.base.label"), tooltip: t("tree.row.base.tooltip") });
  if (node.maskOnly)
    list.push({ label: t("tree.row.mask.label"), tooltip: t("tree.row.mask.tooltip") });
  if (node.hidden)
    list.push({ label: t("tree.row.hidden.label"), tooltip: t("tree.row.hidden.tooltip") });
  const shown = list.at(-1);
  if (!shown) return null;
  const folded = list.slice(0, -1);
  return {
    label: shown.label,
    folded: folded.map((f) => `${f.label} `).join(""),
    tooltip: list.map((f) => f.tooltip).join("\n"),
    baseMerged: base && folded.length > 0,
  };
});
const hasChildren = $derived(node.children.length > 0);
let done = false;
/** 名前の変更の欄の文字と要素。 */
let draft = $state("");
let input = $state<HTMLInputElement | null>(null);

// 名前の変更を始めたら、今の名前を入れて全体を選ぶ (打てばそのまま置き換わる)。
$effect.pre(() => {
  if (renaming) {
    done = false;
    draft = node.name;
  }
});
$effect(() => {
  if (renaming && input) {
    input.focus();
    input.select();
  }
});

function finish(name: string | null) {
  // Enter の後の blur で 2 回呼ばれないようにする。
  if (done) return;
  done = true;
  onrename(name);
}

function onKeydown(e: KeyboardEvent) {
  // 木のキー操作 (矢印・Delete・F2) に渡さない。
  e.stopPropagation();
  if (e.key === "Enter") {
    e.preventDefault();
    finish(draft);
  } else if (e.key === "Escape") {
    // 部品の「確定前に戻す」ではなく、変更そのものをやめる。
    e.preventDefault();
    finish(null);
  }
}

/** 名前の欄の中の押下は行に渡さない (選択・ドラッグ・名前の変更の始まりにしない)。 */
function insideRename(e: Event): boolean {
  return e.target instanceof Element && e.target.closest(".rename") !== null;
}
</script>

<!-- キーボード操作は親の tree (aria-activedescendant) が受け持つので、行自身はキーを受けない。 -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  {id}
  class="row"
  class:selected
  class:primary
  class:hovered
  class:hovered-within={hoveredWithin}
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
  onclick={(e) => {
    if (!insideRename(e)) onselect(e);
  }}
  ondblclick={(e) => {
    if (!insideRename(e)) onrenamestart();
  }}
  onpointerdown={(e) => {
    if (!insideRename(e)) onpointerdown(e);
  }}
  onpointerenter={() => onhover?.(true)}
  onpointerleave={() => onhover?.(false)}
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
    <TextField
      class="rename"
      bind:value={draft}
      bind:ref={input}
      aria-label={t("tree.row.name")}
      onkeydown={onKeydown}
      onblur={() => finish(draft)}
    />
  {:else}
    <span class="name" title={node.ref.id}>{node.name}</span>
  {/if}
  <!-- 行はフォーカスを受けない (木が aria-activedescendant で受ける) ので、印の説明は Tooltip ではなく title で出す。 -->
  {#if tag}
    <span class="ed-tag" class:base-merged={tag.baseMerged} title={tag.tooltip}
      >{#if tag.folded}<span class="ugoe-visually-hidden">{tag.folded}</span>{/if}{tag.label}</span
    >
  {/if}
  {#if node.drawOrder !== undefined}<span class="order" title={t("tree.row.drawOrder")}>{node.drawOrder}</span>{/if}
</div>

<style>
/*
 * 行の中の控えめな文字 (非表示・本体の名前・札・描く順) の色。選んだ行は accent-soft (本体は sunken) の地に hover の重ねが乗っても
 * 4.5:1 に届くように、本文の色へ寄せる (docs/ui.md#エディタのツリーの札と選んだ行-t1654)
 */
.row {
  --tree-muted: var(--ugoe-color-text-muted);
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
/* 状態の見せ方は docs/ui.md#デザイン、ビューポートとの対応は docs/editor.md#選択とホバーの見せ方 */
.row:hover,
.row.hovered {
  background-image: var(--ugoe-overlay-hover);
}
/* 畳んだ中のメッシュに乗っている: hover の地に、左端の縦の線を足す (行そのものに乗っているのと見分ける) */
.row.hovered-within {
  background-image: var(--ugoe-overlay-hover);
  box-shadow: inset 3px 0 0 var(--ugoe-color-accent);
}
.row.selected {
  --tree-muted: color-mix(in srgb, var(--ugoe-color-text) 30%, var(--ugoe-color-text-muted));
  background-color: var(--ugoe-color-accent-soft);
}
/* 主な選択は太字 (ビューポートの太い輪郭に当たる) */
.row.primary .name {
  font-weight: var(--ugoe-font-weight-strong);
}
.row.active {
  border-color: var(--ugoe-color-accent);
}
/* 本体の要素の選択は、ビューポートの輪郭と同じく灰色の破線 */
.row.base.selected {
  background-color: var(--ugoe-color-sunken);
  border-color: var(--ugoe-color-text-muted);
  border-style: dashed;
}
.row.base .name,
.row.hidden .name {
  /* 薄くするのは不透明度ではなく控えめな文字の色で (opacity では 4.5:1 に届かない。T0907) */
  color: var(--tree-muted);
}
/*
 * 札は、中の見せない文字 (まとめた札) の位置の基準にもなる (送る枠の外を基準にしない。docs/ui.md#レイアウト の「送り」。T1667)
 */
.ed-tag {
  position: relative;
  color: var(--tree-muted);
}
/* 本体の札をまとめた札は、本体の要素の選択と同じ破線の枠 */
.ed-tag.base-merged {
  border-style: dashed;
  border-color: var(--tree-muted);
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
/*
 * 強制色: 地の色・影は消え、透明な枠は文字の色の線になる (docs/ui.md#強制色)。行の枠は地の色にして、印は系統の色の地と枠で描き直す。
 * 選んだ行は Highlight の地、キーボードの位置は CanvasText の枠 (選んだ行では内側の線も)、ドロップの位置は Highlight の破線の枠と上下の太い線、
 * 落とせない所は GrayText の点線 (理由の文も出る) (docs/ui.md#エディタのツリーの行の札と強制色-t1670)
 */
@media (forced-colors: active) {
  .row {
    border-color: Canvas;
  }
  /*
   * 選んだ行: 強制色のままでは文字の後ろに地の色の板 (backplate) が敷かれて Highlight の地の上で黒い箱になるので、
   * 強制をやめて (`forced-color-adjust: none`。子孫も受け継ぐ) 中のものすべてを系統の色で描く。名前の変更の欄は強制に戻す
   */
  .row.selected {
    --tree-muted: HighlightText;
    forced-color-adjust: none;
    background-color: Highlight;
    background-image: none;
    box-shadow: none;
    color: HighlightText;
  }
  .row.selected :is(.twisty, .ed-tag, .order) {
    color: HighlightText;
    border-color: HighlightText;
  }
  .row.selected :global(svg.icon) {
    color: HighlightText;
  }
  .row.selected :global(.rename) {
    forced-color-adjust: auto;
  }
  .row.base.selected {
    background-color: Highlight;
    border-color: HighlightText;
  }
  .row.active,
  .row.base.selected.active {
    border-color: CanvasText;
  }
  /* 選んだ行のキーボードの位置は、Highlight の地と CanvasText の枠の差が小さい (ライト) ので、内側に HighlightText の線も引く */
  .row.selected.active {
    outline: 1px solid HighlightText;
    outline-offset: -3px;
  }
  .row.drop-into {
    border-style: dashed;
    border-color: Highlight;
  }
  .row.drop-above {
    border-top: 2px solid Highlight;
  }
  .row.drop-below {
    border-bottom: 2px solid Highlight;
  }
  .row.drop-bad {
    border-width: 1px;
    border-style: dotted;
    border-color: GrayText;
  }
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
/* 名前の変更の欄は行の高さ (24px) に収める */
.row :global(.rename) {
  flex: 1;
  min-width: 0;
}
.row :global(.rename .ugoe-text-field__input) {
  height: var(--ugoe-target-min);
  min-height: 0;
  padding: 0 var(--ugoe-space-1);
}
.order {
  flex: none;
  font-size: var(--ugoe-font-size-xs);
  color: var(--tree-muted);
  font-variant-numeric: tabular-nums;
}
</style>
