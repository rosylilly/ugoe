<!--
  Toggle: すぐに効く入・切 (docs/ui.md#部品の一覧)。決定を押すまで効かない選択肢には Checkbox を使う。
  土台は Bits UI の Switch (role="switch" のボタン。Space・Enter で切り替える)。
-->
<script lang="ts">
import { Switch } from "bits-ui";

interface Props {
  /** 入っているか。 */
  checked?: boolean;
  /** 見えるラベル。翻訳を通した文言を渡す。 */
  label: string;
  disabled?: boolean;
  /** 補足の説明を持つ要素の id。 */
  describedby?: string;
  onchange?: (checked: boolean) => void;
}

let {
  checked = $bindable(false),
  label,
  disabled = false,
  describedby,
  onchange,
}: Props = $props();

const id = $props.id();
</script>

<span class="ugoe-toggle" data-disabled={disabled ? "" : undefined}>
  <Switch.Root
    id={`${id}-switch`}
    class="ugoe-toggle__switch"
    bind:checked
    {disabled}
    aria-describedby={describedby}
    onCheckedChange={(value) => onchange?.(value)}
  >
    <Switch.Thumb class="ugoe-toggle__thumb" />
  </Switch.Root>
  <label class="ugoe-toggle__label" for={`${id}-switch`}>{label}</label>
</span>

<style>
  :global {
    .ugoe-toggle {
      display: inline-flex;
      align-items: center;
      gap: var(--ugoe-space-2);
      min-height: var(--ugoe-target-min);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
    }

    .ugoe-toggle[data-disabled] {
      opacity: 0.5;
    }

    .ugoe-toggle__switch {
      position: relative;
      display: inline-flex;
      flex: none;
      align-items: center;
      width: 32px;
      height: 18px;
      padding: 0 2px;
      border: 1px solid var(--ugoe-color-control-border);
      border-radius: var(--ugoe-radius-full);
      background: var(--ugoe-color-sunken);
      cursor: pointer;
      transition: background-color var(--ugoe-duration-fast);
    }

    .ugoe-toggle__switch:disabled {
      cursor: default;
    }

    .ugoe-toggle__switch[data-state="checked"] {
      border-color: var(--ugoe-color-accent);
      background: var(--ugoe-color-accent);
    }

    .ugoe-toggle__switch:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: 2px;
    }

    /* つまみの位置 (左右) でも状態がわかる */
    .ugoe-toggle__thumb {
      display: block;
      width: 12px;
      height: 12px;
      border-radius: var(--ugoe-radius-full);
      background: var(--ugoe-color-control-border);
      transition: transform var(--ugoe-duration-fast);
    }

    .ugoe-toggle__thumb[data-state="checked"] {
      background: var(--ugoe-color-on-accent);
      transform: translateX(14px);
    }

    .ugoe-toggle__label {
      cursor: pointer;
    }

    .ugoe-toggle[data-disabled] .ugoe-toggle__label {
      cursor: default;
    }
  }
</style>
