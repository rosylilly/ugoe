<!--
  パネルの上のタブの並び (docs/ui.md#エディタの中の共通の形)。中身は呼び手が `value` で切り替えて出す
  (右の列のように、タブの状態が別々の表示の状態に分かれているとき)。中身をこの部品が持つなら @ugoe/ui の Tabs を使う。
  - APG の Tabs (自動で選ぶ形): 並び全体で 1 つのタブ止まり。左右の矢印・Home・End で移って選ぶ。
  - タブごとの説明は Tooltip で出す (`title` 属性は使わない)。`attention` のタブは色と太さと印 (•) で示し、読み上げにも足す。
-->
<script lang="ts" module>
export interface TabItem<V extends string> {
  value: V;
  /** 見えるラベル (翻訳を通した文言)。 */
  label: string;
  /** 説明 (翻訳を通した文言)。 */
  tooltip?: string;
  /** 見てほしい変化がある (読み上げの言葉は `attentionLabel`)。 */
  attention?: boolean;
}
</script>

<script lang="ts" generics="T extends string">
import { Tooltip } from "@ugoe/ui";

interface Props {
  tabs: TabItem<T>[];
  value: T;
  /** 並びの読み上げの名前。 */
  ariaLabel: string;
  /** `attention` のタブの読み上げに足す言葉。 */
  attentionLabel?: string;
  onselect: (value: T) => void;
}

let { tabs, value, ariaLabel, attentionLabel, onselect }: Props = $props();

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
    {#snippet button(attrs: Record<string, unknown>)}
      <button
        {...attrs}
        type="button"
        role="tab"
        aria-selected={selected}
        tabindex={selected ? 0 : -1}
        class:current={selected}
        class:attention={tab.attention}
        onclick={(e) => {
          (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
          onselect(tab.value);
        }}
        >{tab.label}{#if tab.attention}<span aria-hidden="true"> •</span><span class="ugoe-visually-hidden"
            >{attentionLabel}</span
          >{/if}</button
      >
    {/snippet}
    {#if tab.tooltip}
      <Tooltip text={tab.tooltip}>
        {#snippet children(attrs)}{@render button(attrs)}{/snippet}
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
  button:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: -2px;
  }
</style>
