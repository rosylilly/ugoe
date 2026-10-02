<!--
  NumberField: 数値を正確に入れる欄 (docs/ui.md#部品の一覧)。自作 (WAI-ARIA APG の Spinbutton)。
  - 上下の矢印で `step` ずつ増減し、Shift を押しながらなら 10 倍。PageUp / PageDown も 10 倍。
  - Enter かフォーカスを外すと確定し、Esc で確定前の値に戻す。読めない入力は確定前の値に戻す。
  - 編集のための数値なので、桁の区切りを付けず小数点は "." に固定する。
-->
<script lang="ts">
import { clamp, decimalsOf, formatEditable, parseEditable, roundTo, stepValue } from "../number";

interface Props {
  /** 値。確定したときに更新される。 */
  value?: number;
  min?: number;
  max?: number;
  /** 増減の刻み。既定は 1。 */
  step?: number;
  /** 表示と丸めの小数の桁数。既定は `step` の桁数。 */
  decimals?: number;
  /** 後ろに出す単位 (翻訳を通した文言)。 */
  unit?: string;
  /** 見えるラベル。`labelledby` を渡すときは省ける。 */
  label?: string;
  /** 見えるラベルを別の要素に置くときの、その要素の id (Slider と組み合わせるときなど)。 */
  labelledby?: string;
  /** 補足の説明を持つ要素の id。 */
  describedby?: string;
  disabled?: boolean;
  /** 欄の幅 (文字数)。既定は 6。 */
  size?: number;
  /** 値を確定したとき。 */
  onchange?: (value: number) => void;
}

let {
  value = $bindable(0),
  min,
  max,
  step = 1,
  decimals,
  unit,
  label,
  labelledby,
  describedby,
  disabled = false,
  size = 6,
  onchange,
}: Props = $props();

const id = $props.id();
const digits = $derived(decimals ?? decimalsOf(step));
let text = $state("");
let editing = $state(false);

// 編集中でなければ、値の変化を欄に映す
$effect(() => {
  const shown = formatEditable(value, digits);
  if (!editing) text = shown;
});

function commit(next: number) {
  const v = roundTo(clamp(next, min, max), digits);
  text = formatEditable(v, digits);
  if (v !== value) {
    value = v;
    onchange?.(v);
  }
}

function commitText() {
  const parsed = parseEditable(text);
  if (parsed === null) text = formatEditable(value, digits);
  else commit(parsed);
}

function onkeydown(event: KeyboardEvent) {
  let steps = 0;
  const big = event.shiftKey ? 10 : 1;
  switch (event.key) {
    case "ArrowUp":
      steps = big;
      break;
    case "ArrowDown":
      steps = -big;
      break;
    case "PageUp":
      steps = 10;
      break;
    case "PageDown":
      steps = -10;
      break;
    case "Enter":
      commitText();
      return;
    case "Escape":
      // 確定前の値に戻す。戻すものが無ければ、外側 (ダイアログなど) に Esc を渡す
      if (text !== formatEditable(value, digits)) {
        event.preventDefault();
        event.stopPropagation();
        text = formatEditable(value, digits);
      }
      return;
    default:
      return;
  }
  event.preventDefault();
  const base = parseEditable(text) ?? value;
  commit(stepValue(base, steps, { min, max, step }));
}
</script>

<span class="ugoe-number-field" data-disabled={disabled ? "" : undefined}>
  {#if label}
    <label class="ugoe-number-field__label" id={`${id}-label`} for={`${id}-input`}>{label}</label>
  {/if}
  <span class="ugoe-number-field__box">
    <input
      id={`${id}-input`}
      class="ugoe-number-field__input"
      type="text"
      inputmode="decimal"
      role="spinbutton"
      autocomplete="off"
      spellcheck="false"
      {size}
      {disabled}
      aria-labelledby={labelledby ?? (label ? `${id}-label` : undefined)}
      aria-describedby={describedby}
      aria-valuenow={value}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuetext={unit ? `${text} ${unit}` : undefined}
      bind:value={text}
      onfocus={() => (editing = true)}
      onblur={() => {
        editing = false;
        commitText();
      }}
      {onkeydown}
    />
    {#if unit}
      <span class="ugoe-number-field__unit" aria-hidden="true">{unit}</span>
    {/if}
  </span>
</span>

<style>
  :global {
    .ugoe-number-field {
      display: inline-flex;
      align-items: center;
      gap: var(--ugoe-space-2);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
    }

    .ugoe-number-field[data-disabled] {
      opacity: 0.5;
    }

    .ugoe-number-field__box {
      display: inline-flex;
      align-items: center;
      gap: var(--ugoe-space-1);
      box-sizing: border-box;
      min-height: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 0 var(--ugoe-space-2);
      border: 1px solid var(--ugoe-color-control-border);
      border-radius: var(--ugoe-radius-md);
      background: var(--ugoe-color-surface);
    }

    .ugoe-number-field__box:focus-within {
      outline: var(--ugoe-focus-ring);
      outline-offset: 2px;
    }

    .ugoe-number-field__input {
      width: auto;
      min-width: 0;
      padding: 0;
      border: 0;
      background: transparent;
      color: inherit;
      font-family: var(--ugoe-font-mono);
      font-size: var(--ugoe-font-size-md);
      font-variant-numeric: tabular-nums;
      text-align: right;
    }

    /* 輪は外側の枠 (focus-within) に出す */
    .ugoe-number-field__input:focus {
      outline: none;
    }

    .ugoe-number-field__unit {
      color: var(--ugoe-color-text-muted);
      font-size: var(--ugoe-font-size-xs);
    }
  }
</style>
