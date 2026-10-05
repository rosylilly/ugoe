<!--
  Tabs: 同じ場所の中身を切り替える (docs/ui.md#部品の一覧)。土台は Bits UI の Tabs
  (タブの並びで 1 つのタブ止まり。左右の矢印・Home / End で移り、移ると選ぶ)。
  中身は `panel` の snippet に選んだタブの値を渡して描く。
-->
<script lang="ts" generics="T extends string">
import { Tabs } from "bits-ui";
import type { Snippet } from "svelte";
import type { Option } from "../types";

interface Props {
  value?: T;
  /** タブ。`label` は翻訳を通した文言。 */
  tabs: Option<T>[];
  /** タブの並びの読み上げの名前。 */
  "aria-label"?: string;
  /** 中身。選んでいるタブの値を受け取る。 */
  panel: Snippet<[T]>;
  onchange?: (value: T) => void;
}

let { value = $bindable(), tabs, "aria-label": ariaLabel, panel, onchange }: Props = $props();

const current = $derived(value ?? tabs[0]?.value);
</script>

<Tabs.Root
  class="ugoe-tabs"
  value={current}
  onValueChange={(v) => {
    value = v as T;
    onchange?.(v as T);
  }}
>
  <Tabs.List class="ugoe-tabs__list" aria-label={ariaLabel}>
    {#each tabs as tab (tab.value)}
      <Tabs.Trigger class="ugoe-tabs__trigger" value={tab.value} disabled={tab.disabled}>
        {tab.label}
      </Tabs.Trigger>
    {/each}
  </Tabs.List>
  {#each tabs as tab (tab.value)}
    <Tabs.Content class="ugoe-tabs__panel" value={tab.value}>
      {#if tab.value === current}
        {@render panel(tab.value)}
      {/if}
    </Tabs.Content>
  {/each}
</Tabs.Root>

<style>
  :global {
    .ugoe-tabs {
      display: flex;
      flex-direction: column;
      min-width: 0;
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
    }

    .ugoe-tabs__list {
      display: flex;
      gap: var(--ugoe-space-1);
      border-bottom: 1px solid var(--ugoe-color-border);
    }

    .ugoe-tabs__trigger {
      position: relative;
      box-sizing: border-box;
      min-height: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 0 var(--ugoe-space-3);
      border: 0;
      border-bottom: 2px solid transparent;
      border-radius: var(--ugoe-radius-md) var(--ugoe-radius-md) 0 0;
      background: transparent;
      color: var(--ugoe-color-text-muted);
      font: inherit;
      white-space: nowrap;
      cursor: pointer;
    }

    .ugoe-tabs__trigger:hover:not(:disabled) {
      background-image: var(--ugoe-overlay-hover);
      color: var(--ugoe-color-text);
    }

    .ugoe-tabs__trigger:active:not(:disabled) {
      background-image: var(--ugoe-overlay-active);
    }

    /* 選んだタブは色に加えて下線と太さでも示す */
    .ugoe-tabs__trigger[data-state="active"] {
      border-bottom-color: var(--ugoe-color-accent);
      color: var(--ugoe-color-text);
      font-weight: var(--ugoe-font-weight-strong);
    }

    .ugoe-tabs__trigger:disabled {
      cursor: default;
      opacity: var(--ugoe-opacity-disabled);
    }

    .ugoe-tabs__trigger:focus-visible,
    .ugoe-tabs__panel:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: var(--ugoe-focus-offset);
    }

    .ugoe-tabs__panel {
      padding: var(--ugoe-space-3) 0;
    }
  }
</style>
