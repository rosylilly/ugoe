<!--
  NumberField: 数値を正確に入れる欄 (docs/ui.md#部品の一覧)。自作 (WAI-ARIA APG の Spinbutton)。
  - 上下の矢印で `step` ずつ増減し、Shift を押しながらなら 10 倍。PageUp / PageDown も 10 倍。
  - Enter かフォーカスを外すと確定し、Esc で確定前の値に戻す。読めない入力は確定前の値に戻す。
  - 編集のための数値なので、桁の区切りを付けず小数点は "." に固定する。
  - onchange は利用者の操作 (キー・入力の確定) で値が変わったときだけ呼ぶ。外から渡した値が `decimals` の桁に乗らなくても、
    欄を書き換えずにフォーカスを外したり Enter を押したりしただけでは呼ばない (docs/ui.md#数値の部品と-onchange)。
  - onchange を呼んでいる間は、`numberFieldCommit()` が欄ごとに一意な印を返す (同じ欄の続けての確定をまとめるため)。
-->
<script lang="ts">
import { withNumberFieldCommit } from "../commit";
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
  /** 見えるラベル。`aria-labelledby` を渡すときは省ける。 */
  label?: string;
  /** 見えるラベルを別の要素に置くときの、その要素の id (Slider と組み合わせるときなど)。 */
  "aria-labelledby"?: string;
  /** 補足の説明を持つ要素の id。 */
  "aria-describedby"?: string;
  disabled?: boolean;
  /** 入力の誤り (範囲の外など)。危険の色の枠と `aria-invalid` にする。誤りの文は `aria-describedby` で結ぶ。 */
  invalid?: boolean;
  /** 欄の幅 (文字数)。既定は 6。 */
  size?: number;
  /** 利用者の操作で値を確定し、値が変わったとき。外から渡した値では呼ばない。 */
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
  "aria-labelledby": labelledby,
  "aria-describedby": describedby,
  disabled = false,
  invalid = false,
  size = 6,
  onchange,
}: Props = $props();

const id = $props.id();
let inputEl = $state<HTMLInputElement | null>(null);
const digits = $derived(decimals ?? decimalsOf(step));
let text = $state("");
// 利用者が欄を書き換えて、まだ確定も取り消しもしていないか
let dirty = $state(false);

// 書き換えの途中でなければ、値の変化を欄に映す
$effect(() => {
  const shown = formatEditable(value, digits);
  if (!dirty) text = shown;
});

function revert() {
  dirty = false;
  text = formatEditable(value, digits);
}

function commit(next: number) {
  const v = roundTo(clamp(next, min, max), digits);
  dirty = false;
  text = formatEditable(v, digits);
  if (v !== value) {
    value = v;
    if (onchange) withNumberFieldCommit(id, () => onchange(v));
  }
}

function commitText() {
  // 書き換えていなければ確定しない (表示の丸めや古い表示で、外から渡した値を書き換えないため)
  if (!dirty) return;
  const parsed = parseEditable(text);
  if (parsed === null) revert();
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
      if (dirty) {
        event.preventDefault();
        event.stopPropagation();
        revert();
      }
      return;
    default:
      return;
  }
  event.preventDefault();
  // 値が数でない (NaN など) ときは、範囲に収めた 0 から動かす (NaN を onchange に渡さない)
  const base = parseEditable(text) ?? (Number.isFinite(value) ? value : clamp(0, min, max));
  commit(stepValue(base, steps, { min, max, step }));
}
</script>

<span
  class="ugoe-number-field"
  data-disabled={disabled ? "" : undefined}
  data-invalid={invalid ? "" : undefined}
>
  {#if label}
    <label class="ugoe-number-field__label" id={`${id}-label`} for={`${id}-input`}>{label}</label>
  {/if}
  <!-- 枠の全体 (24px 以上) を押せる範囲にする: 欄の外の余白を押しても欄にフォーカスを移す (T0907) -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span
    class="ugoe-number-field__box"
    onpointerdown={(e) => {
      if (e.target === e.currentTarget && !disabled) {
        e.preventDefault();
        inputEl?.focus();
      }
    }}
  >
    <input
      bind:this={inputEl}
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
      aria-invalid={invalid || undefined}
      aria-valuenow={value}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuetext={unit ? `${text} ${unit}` : undefined}
      bind:value={text}
      oninput={() => (dirty = true)}
      onblur={commitText}
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
      opacity: var(--ugoe-opacity-disabled);
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

    .ugoe-number-field:not([data-disabled]) .ugoe-number-field__box:hover {
      background-image: var(--ugoe-overlay-hover);
    }

    /* 誤りは色に加えて枠の太さでも示す (文は呼び手が aria-describedby で結ぶ) */
    .ugoe-number-field[data-invalid] .ugoe-number-field__box {
      border-color: var(--ugoe-color-danger);
      box-shadow: inset 0 0 0 1px var(--ugoe-color-danger);
    }

    .ugoe-number-field__box:focus-within {
      outline: var(--ugoe-focus-ring);
      outline-offset: var(--ugoe-focus-offset);
    }

    .ugoe-number-field__input {
      align-self: stretch;
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

    /* 誤りの印は外側の枠に出すので、tokens.css の文字の欄の印を打ち消す */
    .ugoe-number-field__input[aria-invalid] {
      box-shadow: none;
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
