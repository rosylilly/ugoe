<!--
  パネルの上のタブの並び (docs/ui.md#エディタの中の共通の形)。中身は呼び手が `value` で切り替えて出す
  (右の列のように、タブの状態が別々の表示の状態に分かれているとき)。中身をこの部品が持つなら @ugoe/ui の Tabs を使う。
  - APG の Tabs (自動で選ぶ形): 並び全体で 1 つのタブ止まり。左右の矢印・Home・End で移って選ぶ。
  - タブごとの説明は Tooltip で出す (`title` 属性は使わない。ポインタを乗せる・フォーカスで出る)。`attention` のタブは色と太さと印 (•) で示し、読み上げにも足す。
  - `icons` にすると、タブをアイコンだけにする (右の列。T1632)。名前は読み上げの名前 (`aria-label`) と Tooltip で出し、
    数 (`badge`) はアイコンの右に、`attention` はアイコンの右上の点で示す。
-->
<script lang="ts" module>
import type { PaneIconName } from "./PaneIcon.svelte";

export interface TabItem<V extends string> {
  value: V;
  /** ラベル (翻訳を通した文言)。`icons` のときは読み上げの名前。 */
  label: string;
  /** 説明 (翻訳を通した文言)。`icons` のときは名前も含めて書く (無ければ `label` を出す)。 */
  tooltip?: string;
  /** 見てほしい変化がある (読み上げの言葉は `attentionLabel`)。 */
  attention?: boolean;
  /** `icons` のときに出すアイコン。 */
  icon?: PaneIconName;
  /** `icons` のときにアイコンに添える数 (AI の提案の数)。読み上げは `label` に含める。 */
  badge?: number;
}
</script>

<script lang="ts" generics="T extends string">
import { Tooltip } from "@ugoe/ui";
import PaneIcon from "./PaneIcon.svelte";

interface Props {
  tabs: TabItem<T>[];
  value: T;
  /** 並びの読み上げの名前。 */
  "aria-label": string;
  /** `attention` のタブの読み上げに足す言葉。 */
  attentionLabel?: string;
  onselect: (value: T) => void;
  /** タブをアイコンだけにする (`icon` の無いタブは文字のまま)。 */
  icons?: boolean;
}

let { tabs, value, "aria-label": ariaLabel, attentionLabel, onselect, icons = false }: Props = $props();

let list = $state<HTMLElement>();

function onkeydown(e: KeyboardEvent) {
  const index = tabs.findIndex((tab) => tab.value === value);
  let next: number;
  if (e.key === "ArrowRight") next = (index + 1) % tabs.length;
  else if (e.key === "ArrowLeft") next = (index - 1 + tabs.length) % tabs.length;
  else if (e.key === "Home") next = 0;
  else if (e.key === "End") next = tabs.length - 1;
  else return;
  e.preventDefault();
  const tab = tabs[next];
  if (!tab) return;
  onselect(tab.value);
  list?.querySelectorAll<HTMLElement>('[role="tab"]')[next]?.focus();
}
</script>

<!-- フォーカスを受けるのは中のタブ (roving tabindex)。並びは矢印キーを受けるだけ。 -->
<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="tab-strip" role="tablist" aria-label={ariaLabel} bind:this={list} {onkeydown}>
  {#each tabs as tab (tab.value)}
    {@const selected = tab.value === value}
    {@const icon = icons ? tab.icon : undefined}
    {#snippet button(attrs: Record<string, unknown>)}
      <button
        {...attrs}
        type="button"
        role="tab"
        aria-selected={selected}
        aria-label={icon ? [tab.label, tab.attention ? attentionLabel : undefined].filter(Boolean).join(" ") : undefined}
        tabindex={selected ? 0 : -1}
        class:current={selected}
        class:attention={tab.attention}
        class:icon={icon !== undefined}
        onclick={(e) => {
          (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
          onselect(tab.value);
        }}
      >
        {#if icon}
          <PaneIcon name={icon} />
          {#if tab.badge !== undefined}<span class="badge" aria-hidden="true">{tab.badge}</span>{/if}
          {#if tab.attention}<span class="dot" aria-hidden="true"></span>{/if}
        {:else}
          {tab.label}{#if tab.attention}<span aria-hidden="true"> •</span><span class="ugoe-visually-hidden"
              >{attentionLabel}</span
            >{/if}
        {/if}
      </button>
    {/snippet}
    {#if icon || tab.tooltip}
      <Tooltip text={tab.tooltip ?? tab.label}>
        {#snippet trigger(attrs)}{@render button(attrs)}{/snippet}
      </Tooltip>
    {:else}
      {@render button({})}
    {/if}
  {/each}
</div>

<style>
  .tab-strip {
    display: flex;
    /* タブが列の幅に収まらなければ折り返す (横にはみ出すと列ごと横に送られて本文の左が切れる)。 */
    flex-wrap: wrap;
    gap: var(--ugoe-space-1);
    border-bottom: 1px solid var(--ugoe-color-border);
  }
  button {
    box-sizing: border-box;
    min-height: max(var(--ugoe-control-height), var(--ugoe-target-min));
    padding: 0 var(--ugoe-space-3);
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--ugoe-color-text-muted);
    font: inherit;
    white-space: nowrap;
    cursor: pointer;
  }
  button:hover {
    color: var(--ugoe-color-text);
  }
  /* 選んだタブは色に加えて下線と太さでも示す (@ugoe/ui の Tabs と同じ見た目) */
  button.current {
    border-bottom-color: var(--ugoe-color-accent);
    color: var(--ugoe-color-text);
    font-weight: var(--ugoe-font-weight-strong);
  }
  button.attention {
    color: var(--ugoe-color-accent);
    font-weight: var(--ugoe-font-weight-strong);
  }
  /* アイコンのタブ: 8 つが 200% の拡大の列 (約 235px) にも 1 行で入る幅 (24px と間 2px)。 */
  button.icon {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 1px;
    min-width: var(--ugoe-target-min);
    padding: 0 var(--ugoe-space-2);
  }
  .badge {
    font-size: var(--ugoe-font-size-xs);
  }
  /* 見てほしい変化の印 (文字のタブの「•」の代わり) */
  .dot {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ugoe-color-accent);
  }
  button:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: -2px;
  }
</style>
