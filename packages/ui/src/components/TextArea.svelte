<!--
  TextArea: 複数行の文字の欄 (メタデータの一覧・AI への注文など) (docs/ui.md#部品の一覧)。自作 (`<textarea>`)。
  - ラベル・補足・誤りの文・確定の約束は TextField と同じ (`label`・`hint`・`error`・`invalid`・`tooltip`)。
  - Enter は改行にする。確定はフォーカスを外したときと、Mod+Enter (macOS は ⌘、ほかは Ctrl)。値が変わったときだけ `onchange` を呼ぶ。
  - Esc で、確定していない書き換えを確定した値 (フォーカスを受けたとき・最後の確定) に戻す。戻すものが無ければ外側 (ダイアログなど) に Esc を渡す。
-->
<script lang="ts">
import type { TriggerAttributes } from "../props";
import Tooltip from "./Tooltip.svelte";

interface Props {
  /** 値。打つたびに更新される (`bind:value`)。 */
  value?: string;
  /** 見える行数。既定は 3。 */
  rows?: number;
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
  /** 等幅の字で出す (パス・URL の一覧など)。 */
  mono?: boolean;
  maxlength?: number;
  /** 既定は true (文章を書く欄なので綴りの印を出す)。 */
  spellcheck?: boolean;
  /** 欄の要素の id (外の `<label for>` と結ぶとき)。省くと作る。 */
  id?: string;
  class?: string;
  /** 欄の要素 (フォーカスを移すときなど)。 */
  ref?: HTMLTextAreaElement | null;
  /** 打つたび。 */
  oninput?: (value: string) => void;
  /** 確定 (フォーカスを外す・Mod+Enter) で値が変わったとき。外から渡した値では呼ばない。 */
  onchange?: (value: string) => void;
  /** キーを押したとき (部品の処理の前に呼ぶ。`preventDefault()` すると部品は確定も Esc での戻しもしない)。 */
  onkeydown?: (event: KeyboardEvent) => void;
  onblur?: (event: FocusEvent) => void;
}

let {
  value = $bindable(""),
  rows = 3,
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
  maxlength,
  spellcheck = true,
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
  if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
    event.preventDefault();
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
  <textarea
    {...attrs}
    bind:this={ref}
    bind:value
    id={fieldId}
    class={["ugoe-text-field__input", "ugoe-text-field__input--multiline", mono && "ugoe-text-field__input--mono"]}
    {rows}
    {placeholder}
    {disabled}
    {readonly}
    {required}
    {maxlength}
    {spellcheck}
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
  ></textarea>
{/snippet}

<span
  class={["ugoe-text-field", className]}
  data-disabled={disabled ? "" : undefined}
  data-invalid={isInvalid ? "" : undefined}
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
