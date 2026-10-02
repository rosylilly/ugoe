<!--
  Checkbox: 決定を押すまで効かない選択肢、または一覧の中の選択 (docs/ui.md#部品の一覧)。
  すぐに効く入・切には Toggle を使う。土台は Bits UI の Checkbox (role="checkbox" のボタン。Space で切り替える)。
-->
<script lang="ts">
import { Checkbox } from "bits-ui";

interface Props {
  /** 入っているか。 */
  checked?: boolean;
  /** どちらとも言えない状態 (一覧の一部だけが選ばれている)。`aria-checked="mixed"` になる。 */
  indeterminate?: boolean;
  /** 見えるラベル。翻訳を通した文言を渡す。 */
  label: string;
  disabled?: boolean;
  /** フォームで送るときの名前。 */
  name?: string;
  /** 補足の説明を持つ要素の id。 */
  describedby?: string;
  onchange?: (checked: boolean) => void;
}

let {
  checked = $bindable(false),
  indeterminate = $bindable(false),
  label,
  disabled = false,
  name,
  describedby,
  onchange,
}: Props = $props();

const id = $props.id();
</script>

<span class="ugoe-checkbox" data-disabled={disabled ? "" : undefined}>
  <Checkbox.Root
    id={`${id}-box`}
    class="ugoe-checkbox__box"
    bind:checked
    bind:indeterminate
    {disabled}
    {name}
    aria-describedby={describedby}
    onCheckedChange={(value) => onchange?.(value)}
  >
    {#snippet children({ checked, indeterminate })}
      <svg viewBox="0 0 16 16" aria-hidden="true">
        {#if indeterminate}
          <path d="M4 8h8" />
        {:else if checked}
          <path d="M3.5 8.5l3 3 6-7" />
        {/if}
      </svg>
    {/snippet}
  </Checkbox.Root>
  <label class="ugoe-checkbox__label" for={`${id}-box`}>{label}</label>
</span>

<style>
  :global {
    .ugoe-checkbox {
      display: inline-flex;
      align-items: center;
      gap: var(--ugoe-space-2);
      min-height: var(--ugoe-target-min);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
    }

    .ugoe-checkbox[data-disabled] {
      opacity: 0.5;
    }

    /* 見た目の箱は 16px だが、押せる範囲はラベルを含めて 24px 以上ある */
    .ugoe-checkbox__box {
      display: inline-flex;
      flex: none;
      align-items: center;
      justify-content: center;
      width: 16px;
      height: 16px;
      padding: 0;
      border: 1px solid var(--ugoe-color-control-border);
      border-radius: var(--ugoe-radius-sm);
      background: var(--ugoe-color-surface);
      color: var(--ugoe-color-on-accent);
      cursor: pointer;
    }

    .ugoe-checkbox__box:disabled {
      cursor: default;
    }

    .ugoe-checkbox__box[data-state="checked"],
    .ugoe-checkbox__box[data-state="indeterminate"] {
      border-color: var(--ugoe-color-accent);
      background: var(--ugoe-color-accent);
    }

    .ugoe-checkbox__box:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: 2px;
    }

    .ugoe-checkbox__box svg {
      width: 14px;
      height: 14px;
      fill: none;
      stroke: currentColor;
      stroke-width: 2;
      stroke-linecap: round;
      stroke-linejoin: round;
    }

    .ugoe-checkbox__label {
      cursor: pointer;
    }

    .ugoe-checkbox[data-disabled] .ugoe-checkbox__label {
      cursor: default;
    }
  }
</style>
