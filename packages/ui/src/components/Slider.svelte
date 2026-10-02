<!--
  Slider: 範囲のある数値 (docs/ui.md#部品の一覧)。土台は Bits UI の Slider
  (矢印・PageUp / PageDown・Home / End)。NumberField とリセット (既定値に戻す) を並べて出せる (この組み合わせは自作)。
-->
<script lang="ts">
import { Slider } from "bits-ui";
import { t } from "../i18n";
import { decimalsOf, roundTo } from "../number";
import IconButton from "./IconButton.svelte";
import NumberField from "./NumberField.svelte";

interface Props {
  value?: number;
  min?: number;
  max?: number;
  step?: number;
  /** 表示と丸めの小数の桁数。既定は `step` の桁数。 */
  decimals?: number;
  /** 見えるラベル。翻訳を通した文言を渡す。 */
  label: string;
  /** 単位 (翻訳を通した文言)。数値の欄の後ろに出す。 */
  unit?: string;
  /** 数値の欄を並べるか。既定は true。 */
  field?: boolean;
  /** 既定値。渡すとリセットのボタンを出す。 */
  defaultValue?: number;
  /** リセットのボタンの読み上げの名前。省くと @ugoe/ui のカタログの「既定値に戻す」(`slider.reset.aria`)。 */
  resetLabel?: string;
  disabled?: boolean;
  /** 値が変わるたび (つまみを動かしている間も)。 */
  onchange?: (value: number) => void;
  /** 値を確定したとき (つまみを離した・キーで動かした・欄で確定した・リセットした)。 */
  oncommit?: (value: number) => void;
}

let {
  value = $bindable(0),
  min = 0,
  max = 100,
  step = 1,
  decimals,
  label,
  unit,
  field = true,
  defaultValue,
  resetLabel,
  disabled = false,
  onchange,
  oncommit,
}: Props = $props();

const id = $props.id();
const digits = $derived(decimals ?? decimalsOf(step));

function set(next: number, commit: boolean) {
  const v = roundTo(next, digits);
  if (v !== value) {
    value = v;
    onchange?.(v);
  }
  if (commit) oncommit?.(v);
}
</script>

<div class="ugoe-slider" data-disabled={disabled ? "" : undefined}>
  <span class="ugoe-slider__label" id={`${id}-label`}>{label}</span>
  <Slider.Root
    type="single"
    class="ugoe-slider__root"
    {value}
    {min}
    {max}
    {step}
    {disabled}
    onValueChange={(v) => set(v, false)}
    onValueCommit={(v) => set(v, true)}
  >
    <span class="ugoe-slider__track">
      <Slider.Range class="ugoe-slider__range" />
    </span>
    <Slider.Thumb index={0} class="ugoe-slider__thumb" aria-labelledby={`${id}-label`} />
  </Slider.Root>
  {#if field}
    <NumberField
      {value}
      {min}
      {max}
      {step}
      decimals={digits}
      {unit}
      {disabled}
      labelledby={`${id}-label`}
      onchange={(v) => {
        set(v, true);
      }}
    />
  {/if}
  {#if defaultValue !== undefined}
    <IconButton
      label={resetLabel ?? t("slider.reset.aria")}
      small
      {disabled}
      class="ugoe-slider__reset"
      onclick={() => set(defaultValue, true)}
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M3 8a5 5 0 1 0 1.5-3.5M3 3v2.5h2.5" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </IconButton>
  {/if}
</div>

<style>
  :global {
    .ugoe-slider {
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto auto;
      grid-template-areas: "label label label" "slider field reset";
      align-items: center;
      column-gap: var(--ugoe-space-2);
      row-gap: var(--ugoe-space-1);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
    }

    .ugoe-slider[data-disabled] {
      opacity: 0.5;
    }

    .ugoe-slider__label {
      grid-area: label;
    }

    .ugoe-slider__root {
      grid-area: slider;
      position: relative;
      display: flex;
      align-items: center;
      min-width: 80px;
      height: var(--ugoe-target-min);
      touch-action: none;
      cursor: pointer;
    }

    .ugoe-slider[data-disabled] .ugoe-slider__root {
      cursor: default;
    }

    .ugoe-slider__track {
      position: relative;
      flex: 1;
      height: 4px;
      border-radius: var(--ugoe-radius-full);
      background: var(--ugoe-color-control-border);
    }

    .ugoe-slider__range {
      position: absolute;
      height: 100%;
      border-radius: var(--ugoe-radius-full);
      background: var(--ugoe-color-accent);
    }

    .ugoe-slider__thumb {
      display: block;
      width: 14px;
      height: 14px;
      border: 2px solid var(--ugoe-color-accent);
      border-radius: var(--ugoe-radius-full);
      background: var(--ugoe-color-surface);
      box-sizing: border-box;
    }

    .ugoe-slider__thumb:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: 2px;
    }

    .ugoe-slider .ugoe-number-field {
      grid-area: field;
    }

    .ugoe-slider__reset {
      grid-area: reset;
    }
  }
</style>
