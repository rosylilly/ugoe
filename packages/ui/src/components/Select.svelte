<!--
  Select: 5 個以上の選択肢から 1 つを選ぶ (docs/ui.md#部品の一覧)。2〜4 個なら SegmentedControl を使う。
  土台は Bits UI の Select (上下の矢印・Enter・頭文字で移る・Esc で閉じる)。
  一覧は body に出す (Portal) ので、テーマと密度は <html> の属性に従う。
-->
<script lang="ts" generics="T extends string">
import { Select } from "bits-ui";
import type { Option } from "../types";

interface Props {
  /** 選んでいる値。何も選んでいなければ空文字列。 */
  value?: T | "";
  /** 選択肢。`label` は翻訳を通した文言。 */
  options: Option<T>[];
  /** 見えるラベル。`aria-labelledby` か `aria-label` を渡すときは省ける。 */
  label?: string;
  /** 見えるラベルを別の要素に置くときの、その要素の id。 */
  "aria-labelledby"?: string;
  /** 見えるラベルを置けないときの読み上げの名前。 */
  "aria-label"?: string;
  /** 何も選んでいないときに出す文言。 */
  placeholder?: string;
  disabled?: boolean;
  onchange?: (value: T) => void;
}

let {
  value = $bindable(""),
  options,
  label,
  "aria-labelledby": labelledby,
  "aria-label": ariaLabel,
  placeholder = "",
  disabled = false,
  onchange,
}: Props = $props();

const id = $props.id();
const selected = $derived(options.find((o) => o.value === value));
const items = $derived(
  options.map((o) => ({ value: o.value, label: o.label, disabled: o.disabled })),
);
</script>

<span class="ugoe-select" data-disabled={disabled ? "" : undefined}>
  {#if label}
    <span class="ugoe-select__label" id={`${id}-label`}>{label}</span>
  {/if}
  <Select.Root
    type="single"
    {value}
    {items}
    {disabled}
    onValueChange={(v) => {
      value = v as T;
      onchange?.(v as T);
    }}
  >
    <Select.Trigger
      class="ugoe-select__trigger"
      aria-labelledby={labelledby ?? (label ? `${id}-label ${id}-value` : undefined)}
      aria-label={ariaLabel}
    >
      <span
        class="ugoe-select__value"
        id={`${id}-value`}
        data-placeholder={selected ? undefined : ""}>{selected?.label ?? placeholder}</span
      >
      <svg class="ugoe-select__chevron" viewBox="0 0 16 16" aria-hidden="true">
        <path d="M4 6l4 4 4-4" />
      </svg>
    </Select.Trigger>
    <Select.Portal>
      <Select.Content class="ugoe-select__content" sideOffset={4}>
        <Select.Viewport class="ugoe-select__viewport">
          {#each options as option (option.value)}
            <Select.Item
              class="ugoe-select__item"
              value={option.value}
              label={option.label}
              disabled={option.disabled}
              aria-disabled={option.disabled ? "true" : undefined}
            >
              {#snippet children({ selected })}
                <svg class="ugoe-select__check" viewBox="0 0 16 16" aria-hidden="true">
                  {#if selected}<path d="M3.5 8.5l3 3 6-7" />{/if}
                </svg>
                <span>{option.label}</span>
              {/snippet}
            </Select.Item>
          {/each}
        </Select.Viewport>
      </Select.Content>
    </Select.Portal>
  </Select.Root>
</span>

<style>
  :global {
    .ugoe-select {
      display: inline-flex;
      align-items: center;
      gap: var(--ugoe-space-2);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
    }

    .ugoe-select[data-disabled] {
      opacity: 0.5;
    }

    .ugoe-select__trigger {
      display: inline-flex;
      align-items: center;
      justify-content: space-between;
      gap: var(--ugoe-space-2);
      box-sizing: border-box;
      min-width: 8em;
      min-height: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 0 var(--ugoe-space-2) 0 var(--ugoe-space-3);
      border: 1px solid var(--ugoe-color-control-border);
      border-radius: var(--ugoe-radius-md);
      background: var(--ugoe-color-surface);
      color: var(--ugoe-color-text);
      font: inherit;
      text-align: left;
      cursor: pointer;
    }

    .ugoe-select__trigger:disabled {
      cursor: default;
    }

    .ugoe-select__trigger:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: 2px;
    }

    .ugoe-select__value[data-placeholder] {
      color: var(--ugoe-color-text-muted);
    }

    .ugoe-select__chevron,
    .ugoe-select__check {
      flex: none;
      width: 14px;
      height: 14px;
      fill: none;
      stroke: currentColor;
      stroke-width: 1.75;
      stroke-linecap: round;
      stroke-linejoin: round;
    }

    .ugoe-select__content {
      z-index: var(--ugoe-z-popover);
      box-sizing: border-box;
      min-width: var(--bits-select-anchor-width);
      max-height: var(--bits-select-content-available-height);
      padding: var(--ugoe-space-1);
      overflow: auto;
      border: 1px solid var(--ugoe-color-border);
      border-radius: var(--ugoe-radius-lg);
      background: var(--ugoe-color-surface);
      box-shadow: var(--ugoe-shadow-overlay);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
    }

    .ugoe-select__item {
      display: flex;
      align-items: center;
      gap: var(--ugoe-space-2);
      min-height: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 0 var(--ugoe-space-3) 0 var(--ugoe-space-2);
      border-radius: var(--ugoe-radius-sm);
      cursor: pointer;
      user-select: none;
    }

    .ugoe-select__item[data-highlighted] {
      background: var(--ugoe-color-accent-soft);
      outline: 1px solid var(--ugoe-color-accent);
      outline-offset: -1px;
    }

    .ugoe-select__item[data-disabled] {
      color: var(--ugoe-color-text-muted);
      cursor: default;
    }
  }
</style>
