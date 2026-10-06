<!--
  TextField: 1 行の文字の欄 (名前・ID・URL・検索など) (docs/ui.md#部品の一覧)。自作 (`<input>`)。
  - 見えるラベル (`label`)・補足 (`hint`)・誤りの文 (`error`) を欄と結んで出す (`aria-describedby`・`aria-invalid`・`aria-errormessage`)。
  - `oninput` は打つたび、`onchange` は確定 (Enter・フォーカスを外す) で値が変わったときに呼ぶ。外から渡した値では呼ばない。
  - Esc で、確定していない書き換えを確定した値 (フォーカスを受けたとき・最後の確定) に戻す。戻すものが無ければ外側 (ダイアログなど) に Esc を渡す。
-->
<script lang="ts">
import type { TriggerAttributes } from "../props";
import Tooltip from "./Tooltip.svelte";

interface Props {
  /** 値。打つたびに更新される (`bind:value`)。 */
  value?: string;
  /** 欄の種類。既定は `text`。秘密 (API キーなど) は `SecretField` を使う。 */
  type?: "text" | "search" | "url";
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
  readonly?: boolean;
  required?: boolean;
  /** 等幅の字で出す (ID・パス・URL など)。 */
  mono?: boolean;
  /** 欄の幅 (文字数)。省くと置いた場所の幅に広がる。 */
  size?: number;
  maxlength?: number;
  /** 既定は `off` (編集の道具なので、ブラウザの入力の候補を出さない)。 */
  autocomplete?: HTMLInputElement["autocomplete"];
  /** 既定は false (名前や ID に綴りの印を出さない)。 */
  spellcheck?: boolean;
  inputmode?: "text" | "numeric" | "decimal" | "url" | "search" | "email" | "tel" | "none";
  /** 欄の要素の id (外の `<label for>` と結ぶとき)。省くと作る。 */
  id?: string;
  class?: string;
  /** 欄の要素 (フォーカスを移すときなど)。 */
  ref?: HTMLInputElement | null;
  /** 打つたび。 */
  oninput?: (value: string) => void;
  /** 確定 (Enter・フォーカスを外す) で値が変わったとき。外から渡した値では呼ばない。 */
  onchange?: (value: string) => void;
  /** キーを押したとき (部品の処理の前に呼ぶ。`preventDefault()` すると部品は Esc で戻さない)。 */
  onkeydown?: (event: KeyboardEvent) => void;
  onblur?: (event: FocusEvent) => void;
}

let {
  value = $bindable(""),
  type = "text",
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
  readonly = false,
  required = false,
  mono = false,
  size,
  maxlength,
  autocomplete = "off",
  spellcheck = false,
  inputmode,
  id: inputId,
  class: className = "",
  ref = $bindable(null),
  oninput,
  onchange,
  onkeydown,
  onblur,
}: Props = $props();

const uid = $props.id();
const fieldId = $derived(inputId ?? `${uid}-input`);
const isInvalid = $derived(invalid || Boolean(error));
// 確定した値 (フォーカスを受けたとき・最後の確定)。Esc で戻す先と、onchange を呼ぶかの比べる元
let committed = "";
// 利用者が書き換えて、まだ確定も取り消しもしていないか (外から渡した値の変化では立てない)
let dirty = false;

function describedBy(extra?: unknown): string | undefined {
  const ids = [
    typeof extra === "string" ? extra : undefined,
    describedby,
    hint ? `${uid}-hint` : undefined,
    error ? `${uid}-error` : undefined,
  ].filter(Boolean);
  return ids.length > 0 ? ids.join(" ") : undefined;
}

function commit() {
  if (!dirty) return;
  dirty = false;
  if (value !== committed) {
    committed = value;
    onchange?.(value);
  }
}

function handleKeydown(event: KeyboardEvent) {
  onkeydown?.(event);
  if (event.defaultPrevented || event.isComposing) return;
  if (event.key === "Enter") {
    commit();
  } else if (event.key === "Escape" && dirty) {
    event.preventDefault();
    event.stopPropagation();
    dirty = false;
    if (value !== committed) {
      value = committed;
      oninput?.(value);
    }
  }
}
</script>

{#snippet input(attrs: TriggerAttributes)}
  <input
    {...attrs}
    bind:this={ref}
    bind:value
    id={fieldId}
    class={["ugoe-text-field__input", mono && "ugoe-text-field__input--mono"]}
    {type}
    {placeholder}
    {disabled}
    {readonly}
    {required}
    {size}
    {maxlength}
    {autocomplete}
    {spellcheck}
    {inputmode}
    aria-labelledby={labelledby ?? (label ? `${uid}-label` : undefined)}
    aria-label={ariaLabel}
    aria-describedby={describedBy(attrs["aria-describedby"])}
    aria-invalid={isInvalid || undefined}
    aria-errormessage={error ? `${uid}-error` : undefined}
    onfocus={(e) => {
      committed = value;
      dirty = false;
      (attrs.onfocus as ((e: FocusEvent) => void) | undefined)?.(e);
    }}
    oninput={() => {
      dirty = true;
      oninput?.(value);
    }}
    onchange={commit}
    onblur={(e) => {
      commit();
      (attrs.onblur as ((e: FocusEvent) => void) | undefined)?.(e);
      onblur?.(e);
    }}
    onkeydown={handleKeydown}
  />
{/snippet}

<span
  class={["ugoe-text-field", className]}
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
