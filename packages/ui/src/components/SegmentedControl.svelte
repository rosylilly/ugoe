<!--
  SegmentedControl: 2〜4 個の選択肢から 1 つを選ぶ (すべて見せたいとき。docs/ui.md#部品の一覧)。
  土台は Bits UI の RadioGroup (全体で 1 つのタブ止まり。矢印で移って選ぶ)。
-->
<script lang="ts" generics="T extends string">
import { RadioGroup } from "bits-ui";
import type { Option } from "../types";

interface Props {
  value?: T | "";
  /** 選択肢。`label` は翻訳を通した文言。 */
  options: Option<T>[];
  /** まとまりの見えるラベル。`aria-label` を渡すときは省ける。 */
  label?: string;
  /** 見えるラベルを置けないときの読み上げの名前。 */
  "aria-label"?: string;
  disabled?: boolean;
  onchange?: (value: T) => void;
}

let {
  value = $bindable(""),
  options,
  label,
  "aria-label": ariaLabel,
  disabled = false,
  onchange,
}: Props = $props();

const id = $props.id();
</script>

<span class="ugoe-segmented" data-disabled={disabled ? "" : undefined}>
  {#if label}
    <span class="ugoe-segmented__label" id={`${id}-label`}>{label}</span>
  {/if}
  <RadioGroup.Root
    class="ugoe-segmented__group"
    orientation="horizontal"
    {value}
    {disabled}
    aria-labelledby={label ? `${id}-label` : undefined}
    aria-label={ariaLabel}
    onValueChange={(v) => {
      value = v as T;
      onchange?.(v as T);
    }}
  >
    {#each options as option (option.value)}
      <RadioGroup.Item class="ugoe-segmented__item" value={option.value} disabled={option.disabled}>
        {option.label}
      </RadioGroup.Item>
    {/each}
  </RadioGroup.Root>
</span>

<style>
  :global {
    .ugoe-segmented {
      display: inline-flex;
      align-items: center;
      gap: var(--ugoe-space-2);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
    }

    .ugoe-segmented[data-disabled] {
      opacity: var(--ugoe-opacity-disabled);
    }

    .ugoe-segmented__group {
      display: inline-flex;
      padding: 1px;
      gap: 1px;
      border: 1px solid var(--ugoe-color-control-border);
      border-radius: var(--ugoe-radius-md);
      background: var(--ugoe-color-sunken);
    }

    .ugoe-segmented__item {
      box-sizing: border-box;
      min-width: var(--ugoe-target-min);
      min-height: max(calc(var(--ugoe-control-height) - 4px), var(--ugoe-target-min));
      padding: 0 var(--ugoe-space-3);
      border: 1px solid transparent;
      border-radius: var(--ugoe-radius-sm);
      background: transparent;
      color: var(--ugoe-color-text);
      font: inherit;
      white-space: nowrap;
      cursor: pointer;
    }

    .ugoe-segmented__item:hover:not(:disabled) {
      background-image: var(--ugoe-overlay-hover);
    }

    .ugoe-segmented__item:active:not(:disabled) {
      background-image: var(--ugoe-overlay-active);
    }

    /* 選んだものは地の色に加えて枠と太さでも示す */
    .ugoe-segmented__item[data-state="checked"] {
      border-color: var(--ugoe-color-accent);
      background: var(--ugoe-color-surface);
      font-weight: var(--ugoe-font-weight-strong);
    }

    .ugoe-segmented__item:disabled {
      cursor: default;
      opacity: var(--ugoe-opacity-disabled);
    }

    /* 部品の全体が無効なら、全体に掛けた薄さだけにする (二重に薄くしない) */
    .ugoe-segmented[data-disabled] .ugoe-segmented__item:disabled {
      opacity: 1;
    }

    .ugoe-segmented__item:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: var(--ugoe-focus-offset);
    }
  }
</style>
