<!--
  Slider: 範囲のある数値 (docs/ui.md#部品の一覧)。土台は Bits UI の Slider
  (矢印・PageUp / PageDown・Home / End)。NumberField とリセット (既定値に戻す) を並べて出せる (この組み合わせは自作)。
  onchange と oncommit は利用者の操作 (ドラッグ・キー・欄・リセット) のときだけ呼ぶ。外から渡した値を Bits UI が刻みの格子に
  寄せても呼ばない (docs/ui.md#数値の部品と-onchange)。
  `tooltip` を渡すと、つまみに Tooltip で説明を出す (`title` 属性の代わり)。
  `hideLabel` で名前を見せずに読み上げにだけ使う (行が狭く、名前を行の頭などに別に見せているとき)。
-->
<script lang="ts">
import { Slider } from "bits-ui";
import { t } from "../i18n";
import { decimalsOf, roundTo } from "../number";
import type { TriggerAttributes } from "../props";
import IconButton from "./IconButton.svelte";
import NumberField from "./NumberField.svelte";
import Tooltip from "./Tooltip.svelte";

interface Props {
  value?: number;
  min?: number;
  max?: number;
  step?: number;
  /** 表示と丸めの小数の桁数。既定は `step` の桁数。 */
  decimals?: number;
  /** 見えるラベル。翻訳を通した文言を渡す。 */
  label: string;
  /** ラベルを見せない (読み上げの名前には使う)。既定は false。 */
  hideLabel?: boolean;
  /** 単位 (翻訳を通した文言)。数値の欄の後ろに出す。 */
  unit?: string;
  /** 数値の欄を並べるか。既定は true。 */
  field?: boolean;
  /** 既定値。渡すとリセットのボタンを出す。 */
  defaultValue?: number;
  /** リセットのボタンの読み上げの名前。省くと @ugoe/ui のカタログの「既定値に戻す」(`slider.reset.aria`)。 */
  resetLabel?: string;
  disabled?: boolean;
  /** 補足の説明を持つ要素の id。つまみと数値の欄の両方に結ぶ。 */
  "aria-describedby"?: string;
  /** つまみに出す説明 (Tooltip。翻訳を通した文言)。`title` 属性の代わり。 */
  tooltip?: string;
  /** 利用者の操作で値が変わるたび (つまみを動かしている間も)。外から渡した値では呼ばない。 */
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
  hideLabel = false,
  unit,
  field = true,
  defaultValue,
  resetLabel,
  disabled = false,
  "aria-describedby": describedby,
  tooltip,
  onchange,
  oncommit,
}: Props = $props();

const id = $props.id();
const digits = $derived(decimals ?? decimalsOf(step));

// 利用者がつまみを操作している間 (ポインタを押している間と、キーの処理の間) だけ true。
// Bits UI は外から渡した値が刻みの格子に乗らないと、格子に寄せて onValueChange を呼ぶ。それを利用者の操作と区別する。
let interacting = false;

function endPointer() {
  interacting = false;
}

function set(next: number, commit: boolean) {
  const v = roundTo(next, digits);
  if (v !== value) {
    value = v;
    onchange?.(v);
  }
  if (commit) oncommit?.(v);
}
</script>

{#snippet thumb(attrs: TriggerAttributes)}
  <Slider.Thumb
    {...attrs}
    id={`${id}-thumb`}
    index={0}
    class="ugoe-slider__thumb"
    aria-labelledby={`${id}-label`}
    aria-describedby={[attrs["aria-describedby"], describedby].filter(Boolean).join(" ") || undefined}
  />
{/snippet}

<svelte:document onpointerup={endPointer} onpointercancel={endPointer} onpointerleave={endPointer} />

<div
  class="ugoe-slider"
  data-disabled={disabled ? "" : undefined}
  data-label-hidden={hideLabel ? "" : undefined}
>
  <span class={["ugoe-slider__label", hideLabel && "ugoe-visually-hidden"]} id={`${id}-label`}>{label}</span>
  <Slider.Root
    type="single"
    class="ugoe-slider__root"
    {value}
    {min}
    {max}
    {step}
    {disabled}
    onValueChange={(v) => {
      if (interacting) set(v, false);
    }}
    onValueCommit={(v) => set(v, true)}
    onpointerdowncapture={(e: PointerEvent) => {
      if (e.button === 0 && !disabled) interacting = true;
    }}
    onkeydowncapture={() => (interacting = true)}
    onkeydown={() => (interacting = false)}
  >
    <span class="ugoe-slider__track">
      <Slider.Range class="ugoe-slider__range" />
    </span>
    {#if tooltip}
      <Tooltip text={tooltip} id={`${id}-thumb`} {disabled}>
        {#snippet trigger(attrs)}{@render thumb(attrs)}{/snippet}
      </Tooltip>
    {:else}
      {@render thumb({})}
    {/if}
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
      aria-labelledby={`${id}-label`}
      aria-describedby={describedby}
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
      <svg class="ugoe-icon" viewBox="0 0 16 16" aria-hidden="true">
        <path d="M3 8a5 5 0 1 0 1.5-3.5M3 3v2.5h2.5" />
      </svg>
    </IconButton>
  {/if}
</div>

<style>
  :global {
    /* 見せないラベル (.ugoe-visually-hidden) の位置の基準にもなる (docs/ui.md#レイアウト の「送り」) */
    .ugoe-slider {
      position: relative;
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

    .ugoe-slider[data-label-hidden] {
      grid-template-areas: "slider field reset";
    }

    .ugoe-slider[data-disabled] {
      opacity: var(--ugoe-opacity-disabled);
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

    .ugoe-slider:not([data-disabled]) .ugoe-slider__thumb:hover {
      background-image: var(--ugoe-overlay-hover);
    }

    .ugoe-slider:not([data-disabled]) .ugoe-slider__thumb:active {
      background-image: var(--ugoe-overlay-active);
    }

    .ugoe-slider__thumb:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: var(--ugoe-focus-offset);
    }

    .ugoe-slider .ugoe-number-field {
      grid-area: field;
    }

    .ugoe-slider__reset {
      grid-area: reset;
    }
  }
</style>
