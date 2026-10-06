<!--
  NumberField: 数値を正確に入れる欄 (docs/ui.md#部品の一覧)。自作 (WAI-ARIA APG の Spinbutton)。
  - 上下の矢印で `step` ずつ増減し、Shift を押しながらなら 10 倍。PageUp / PageDown も 10 倍。
  - Enter かフォーカスを外すと確定し、Esc で確定前の値に戻す。読めない入力は確定前の値に戻す。
  - 編集のための数値なので、桁の区切りを付けず小数点は "." に固定する。
  - onchange は利用者の操作 (キー・入力の確定) で値が変わったときだけ呼ぶ。外から渡した値が `decimals` の桁に乗らなくても、
    欄を書き換えずにフォーカスを外したり Enter を押したりしただけでは呼ばない (docs/ui.md#数値の部品と-onchange)。
  - onchange を呼んでいる間は、`numberFieldCommit()` が欄ごとに一意な印を返す (同じ欄の続けての確定をまとめるため)。
  - `tooltip` を渡すと、欄に Tooltip で説明を出す (`title` 属性の代わり)。
  - `optional` なら値に空 (`null`。既定を使うなど) を持てる (docs/ui.md#空を持てる数値の欄)。空の欄には `placeholder`
    (省くと `emptyValue` を書式にしたもの) を出し、欄を空にして確定すると `null` を送る。空から矢印キーで動かすときは
    `emptyValue` (省くと範囲に収めた 0) から動かす。
-->
<script lang="ts" generics="Optional extends boolean = false">
import { withNumberFieldCommit } from "../commit";
import { clamp, decimalsOf, formatEditable, parseEditable, roundTo, stepValue } from "../number";
import type { TriggerAttributes } from "../props";
import Tooltip from "./Tooltip.svelte";

interface CommonProps {
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
  /** 見えるラベルを置けないときの読み上げの名前。 */
  "aria-label"?: string;
  /** 補足の説明を持つ要素の id。 */
  "aria-describedby"?: string;
  disabled?: boolean;
  /** 入力の誤り (範囲の外など)。危険の色の枠と `aria-invalid` にする。誤りの文は `aria-describedby` で結ぶ。 */
  invalid?: boolean;
  /** 欄に出す説明 (Tooltip。翻訳を通した文言)。`title` 属性の代わり。 */
  tooltip?: string;
  /** 欄の幅 (文字数)。既定は 6。 */
  size?: number;
  /** 空の欄に出す文字 (翻訳を通した文言)。 */
  placeholder?: string;
}

/** 値の型。`optional` なら空 (`null`) を持てる。 */
type Value = Optional extends true ? number | null : number;

interface Props extends CommonProps {
  /** 値に空 (`null`。既定を使うなど) を持てる。 */
  optional?: Optional;
  /** 値。確定したときに更新される。`optional` なら `null` は空。 */
  value?: Value;
  /** 空が表す値 (既定値。`optional` のとき)。空の欄の `placeholder` の既定と、空から矢印キーで動かすときの元にする。 */
  emptyValue?: number;
  /** 打つたびに、読めた値 (範囲に収め、丸めない)。`optional` で欄が空なら `null`。読めない文字の間は呼ばない。 */
  oninput?: (value: Value) => void;
  /** 利用者の操作で値を確定し、値が変わったとき (`optional` で空にしたら `null`)。外から渡した値では呼ばない。 */
  onchange?: (value: Value) => void;
}

let {
  value = $bindable(0 as Value),
  min,
  max,
  step = 1,
  decimals,
  unit,
  label,
  "aria-labelledby": labelledby,
  "aria-label": ariaLabel,
  "aria-describedby": describedby,
  disabled = false,
  invalid = false,
  tooltip,
  size = 6,
  placeholder,
  optional = false as Optional,
  emptyValue,
  oninput,
  onchange,
}: Props = $props();

const id = $props.id();
let inputEl = $state<HTMLInputElement | null>(null);
const digits = $derived(decimals ?? decimalsOf(step));
let text = $state("");
// 利用者が欄を書き換えて、まだ確定も取り消しもしていないか
let dirty = $state(false);
// optional で値が空か (null・undefined)
const empty = $derived(optional && (value === null || value === undefined));
const shownPlaceholder = $derived(
  placeholder ??
    (optional && emptyValue !== undefined ? formatEditable(emptyValue, digits) : undefined),
);

// 書き換えの途中でなければ、値の変化を欄に映す
$effect(() => {
  const shown = format(value);
  if (!dirty) text = shown;
});

function format(v: number | null | undefined): string {
  return typeof v === "number" ? formatEditable(v, digits) : "";
}

function revert() {
  dirty = false;
  text = format(value);
}

/** 値を確定する。`null` は空 (optional のときだけ渡す)。 */
function commit(next: number | null) {
  const v = next === null ? null : roundTo(clamp(next, min, max), digits);
  dirty = false;
  text = format(v);
  if (v !== (value ?? null)) {
    value = v as Value;
    const call = onchange as ((v: number | null) => void) | undefined;
    if (call) withNumberFieldCommit(id, () => call(v));
  }
}

function commitText() {
  // 書き換えていなければ確定しない (表示の丸めや古い表示で、外から渡した値を書き換えないため)
  if (!dirty) return;
  if (optional && text.trim() === "") {
    commit(null);
    return;
  }
  const parsed = parseEditable(text);
  if (parsed === null) revert();
  else commit(parsed);
}

function handleInput(event: Event) {
  dirty = true;
  const call = oninput as ((v: number | null) => void) | undefined;
  if (!call) return;
  // bind:value の更新と順が決まっていないので、欄の要素から読む
  const typed = (event.currentTarget as HTMLInputElement).value;
  if (optional && typed.trim() === "") {
    call(null);
    return;
  }
  const parsed = parseEditable(typed);
  if (parsed !== null) call(clamp(parsed, min, max));
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
  // 値が空 (optional) のときは空が表す値から、数でない (NaN など) ときは範囲に収めた 0 から動かす (NaN を onchange に渡さない)
  const fallback =
    empty && emptyValue !== undefined && Number.isFinite(emptyValue)
      ? emptyValue
      : clamp(0, min, max);
  const base =
    parseEditable(text) ?? (typeof value === "number" && Number.isFinite(value) ? value : fallback);
  commit(stepValue(base, steps, { min, max, step }));
}
</script>

{#snippet input(attrs: TriggerAttributes)}
  <input
    {...attrs}
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
    placeholder={shownPlaceholder}
    aria-labelledby={labelledby ?? (label ? `${id}-label` : undefined)}
    aria-label={ariaLabel}
    aria-describedby={[attrs["aria-describedby"], describedby].filter(Boolean).join(" ") || undefined}
    aria-invalid={invalid || undefined}
    aria-valuenow={typeof value === "number" ? value : undefined}
    aria-valuemin={min}
    aria-valuemax={max}
    aria-valuetext={unit && text !== "" ? `${text} ${unit}` : undefined}
    bind:value={text}
    oninput={handleInput}
    onblur={(e) => {
      commitText();
      (attrs.onblur as ((e: FocusEvent) => void) | undefined)?.(e);
    }}
    {onkeydown}
  />
{/snippet}

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
    {#if tooltip}
      <Tooltip text={tooltip} id={`${id}-input`} {disabled}>
        {#snippet trigger(attrs)}{@render input(attrs)}{/snippet}
      </Tooltip>
    {:else}
      {@render input({})}
    {/if}
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

    .ugoe-number-field__input::placeholder {
      color: var(--ugoe-color-text-muted);
      opacity: 1;
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
