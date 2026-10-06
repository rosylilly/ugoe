<!--
  SecretField: API キーなどの秘密を入れる 1 行の欄 (docs/ui.md#部品の一覧)。自作 (`<input type="password">`)。
  - 見た目・ラベル・補足・誤りの文は TextField と同じ。
  - 値を Svelte の状態に持たない (props にも bind にも出さない)。呼び手は `filled` (空白を除いて空でないか) だけを受け、
    送るときに `take()` で値を取り出す (取り出すと欄を空にする)。捨てるときは `clear()`。
  - Esc で、入れかけの値を消す。消すものが無ければ外側 (ダイアログなど) に Esc を渡す。
  - Enter は標準のまま (`<form>` の中なら送る)。
-->
<script lang="ts">
import type { TriggerAttributes } from "../props";
import Tooltip from "./Tooltip.svelte";

interface Props {
  /** 空白を除いて空でないか (打つたびに更新される。`bind:filled`)。値そのものは渡さない。 */
  filled?: boolean;
  /** 見えるラベル。`aria-labelledby` か `aria-label` を渡すとき、または外の `<label for={id}>` を使うときは省ける。 */
  label?: string;
  /** 見えるラベルを別の要素に置くときの、その要素の id。 */
  "aria-labelledby"?: string;
  /** 見えるラベルを置けないときの読み上げの名前。 */
  "aria-label"?: string;
  /** 補足の説明を持つ、ほかの要素の id。 */
  "aria-describedby"?: string;
  /** 欄の下に出す補足 (翻訳を通した文言)。欄の説明として読み上げる。 */
  hint?: string;
  /** 誤りの文 (翻訳を通した文言)。渡すと欄を誤りの印にし、文を欄の下に出して読み上げる。 */
  error?: string;
  /** 誤りの印だけを付ける (誤りの文を呼び手がほかの場所に出し、`aria-describedby` で結ぶとき)。 */
  invalid?: boolean;
  /** 欄に出す説明 (Tooltip。翻訳を通した文言)。`title` 属性の代わり。 */
  tooltip?: string;
  placeholder?: string;
  disabled?: boolean;
  /** 欄の幅 (文字数)。省くと置いた場所の幅に広がる。 */
  size?: number;
  /** 欄の要素の id (外の `<label for>` と結ぶとき)。省くと作る。 */
  id?: string;
  class?: string;
}

let {
  filled = $bindable(false),
  label,
  "aria-labelledby": labelledby,
  "aria-label": ariaLabel,
  "aria-describedby": describedby,
  hint,
  error,
  invalid = false,
  tooltip,
  placeholder,
  disabled = false,
  size,
  id: inputId,
  class: className = "",
}: Props = $props();

const uid = $props.id();
const fieldId = $derived(inputId ?? `${uid}-input`);
const isInvalid = $derived(invalid || Boolean(error));
// 欄の要素 (値はここにだけある。$state にしない)
let inputEl: HTMLInputElement | null = null;

/** 入れた値を取り出し、欄を空にする。 */
export function take(): string {
  const value = inputEl?.value ?? "";
  clear();
  return value;
}

/** 入れた値を捨てる。 */
export function clear(): void {
  if (inputEl) inputEl.value = "";
  filled = false;
}

/** 欄にフォーカスを移す。 */
export function focus(): void {
  inputEl?.focus();
}

function describedBy(extra?: unknown): string | undefined {
  const ids = [
    typeof extra === "string" ? extra : undefined,
    describedby,
    hint ? `${uid}-hint` : undefined,
    error ? `${uid}-error` : undefined,
  ].filter(Boolean);
  return ids.length > 0 ? ids.join(" ") : undefined;
}

function handleKeydown(event: KeyboardEvent) {
  if (event.isComposing || event.key !== "Escape") return;
  if (inputEl && inputEl.value !== "") {
    event.preventDefault();
    event.stopPropagation();
    clear();
  }
}
</script>

{#snippet input(attrs: TriggerAttributes)}
  <input
    {...attrs}
    bind:this={inputEl}
    id={fieldId}
    class="ugoe-text-field__input"
    type="password"
    autocomplete="off"
    spellcheck="false"
    {placeholder}
    {disabled}
    {size}
    aria-labelledby={labelledby ?? (label ? `${uid}-label` : undefined)}
    aria-label={ariaLabel}
    aria-describedby={describedBy(attrs["aria-describedby"])}
    aria-invalid={isInvalid || undefined}
    aria-errormessage={error ? `${uid}-error` : undefined}
    oninput={(e) => (filled = e.currentTarget.value.trim() !== "")}
    onkeydown={handleKeydown}
  />
{/snippet}

<span
  class={["ugoe-text-field", "ugoe-secret-field", className]}
  data-disabled={disabled ? "" : undefined}
  data-invalid={isInvalid ? "" : undefined}
  data-sized={size !== undefined ? "" : undefined}
>
  {#if label}
    <label class="ugoe-text-field__label" id={`${uid}-label`} for={fieldId}>{label}</label>
  {/if}
  {#if tooltip}
    <Tooltip text={tooltip} id={fieldId} {disabled}>
      {#snippet trigger(attrs)}{@render input(attrs)}{/snippet}
    </Tooltip>
  {:else}
    {@render input({})}
  {/if}
  {#if hint}
    <span class="ugoe-text-field__hint" id={`${uid}-hint`}>{hint}</span>
  {/if}
  {#if error}
    <span class="ugoe-text-field__error" id={`${uid}-error`}>{error}</span>
  {/if}
</span>
