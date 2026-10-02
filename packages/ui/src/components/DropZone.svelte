<!--
  DropZone: ファイルを落とす領域 (docs/ui.md#部品の一覧)。自作。
  ドロップは見えるポインタの操作なので、同じことができる「ファイル…」のボタンを必ず並べる (キーボードと読み上げはこちらを使う)。
  - `accept` (`<input type="file">` の書き方) に合わないファイルは `onreject` に渡す。
  - Tauri のように、ファイルのドロップを WebView の外で受けるアプリは、`active` でドロップを待つ見た目を出し、
    `onbrowse` で自前のファイルのダイアログを開く (渡さなければ <input type="file"> を開く)。
-->
<script lang="ts">
import type { Snippet } from "svelte";
import { partitionFiles } from "../files";
import { t } from "../i18n";
import Button from "./Button.svelte";
import Icon from "./Icon.svelte";

interface Props {
  /** 受け付けるファイル (`.ugoe,.psd`・`image/*`)。省くとすべて。 */
  accept?: string;
  /** 複数のファイルを受け付けるか。 */
  multiple?: boolean;
  disabled?: boolean;
  /** 領域の案内。省くと @ugoe/ui のカタログの「ここにファイルを落とす」。 */
  prompt?: string;
  /** ボタンの文言。省くと @ugoe/ui のカタログの「ファイルを選ぶ…」。 */
  buttonLabel?: string;
  /** 外から、ファイルを落とそうとしている見た目にする。 */
  active?: boolean;
  /** 受け付けたファイル。 */
  onfiles?: (files: File[]) => void;
  /** `accept` に合わなかったファイル (`multiple` でないときの 2 つ目以降も)。 */
  onreject?: (files: File[]) => void;
  /** ボタンで自前のファイルのダイアログを開くとき。 */
  onbrowse?: () => void;
  /** 案内の下に足す中身 (形式の説明など)。 */
  children?: Snippet;
}

let {
  accept,
  multiple = false,
  disabled = false,
  prompt,
  buttonLabel,
  active = false,
  onfiles,
  onreject,
  onbrowse,
  children,
}: Props = $props();

const id = $props.id();
let input = $state<HTMLInputElement>();
/** 中に入っているドラッグの数 (子の要素への出入りで dragenter と dragleave が重なるため数える)。 */
let depth = $state(0);
const dragging = $derived(!disabled && (active || depth > 0));

function hasFiles(event: DragEvent): boolean {
  return [...(event.dataTransfer?.types ?? [])].includes("Files");
}

function take(files: File[]) {
  const { accepted, rejected } = partitionFiles(files, accept);
  const kept = multiple ? accepted : accepted.slice(0, 1);
  const extra = multiple ? [] : accepted.slice(1);
  if (kept.length > 0) onfiles?.(kept);
  if (rejected.length + extra.length > 0) onreject?.([...rejected, ...extra]);
}
</script>

<div
  class="ugoe-drop-zone"
  data-dragging={dragging ? "" : undefined}
  data-disabled={disabled ? "" : undefined}
  role="group"
  aria-labelledby={`${id}-prompt`}
  ondragenter={(e) => {
    if (disabled || !hasFiles(e)) return;
    e.preventDefault();
    depth++;
  }}
  ondragover={(e) => {
    if (disabled || !hasFiles(e)) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
  }}
  ondragleave={() => {
    if (depth > 0) depth--;
  }}
  ondrop={(e) => {
    if (disabled) return;
    e.preventDefault();
    depth = 0;
    take([...(e.dataTransfer?.files ?? [])]);
  }}
>
  <span class="ugoe-drop-zone__icon"><Icon name="upload" /></span>
  <p class="ugoe-drop-zone__prompt" id={`${id}-prompt`}>
    {prompt ?? t(multiple ? "dropZone.prompt.multiple" : "dropZone.prompt.single")}
  </p>
  {#if children}<div class="ugoe-drop-zone__hint">{@render children()}</div>{/if}
  <Button
    {disabled}
    onclick={() => {
      if (onbrowse) onbrowse();
      else input?.click();
    }}>{buttonLabel ?? t("dropZone.browse.label")}</Button
  >
  {#if !onbrowse}
    <input
      bind:this={input}
      class="ugoe-drop-zone__input"
      type="file"
      tabindex="-1"
      aria-hidden="true"
      {accept}
      {multiple}
      {disabled}
      onchange={(e) => {
        take([...(e.currentTarget.files ?? [])]);
        e.currentTarget.value = "";
      }}
    />
  {/if}
</div>

<style>
  :global {
    .ugoe-drop-zone {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: var(--ugoe-space-2);
      box-sizing: border-box;
      padding: var(--ugoe-space-5);
      border: 2px dashed var(--ugoe-color-control-border);
      border-radius: var(--ugoe-radius-lg);
      background: var(--ugoe-color-surface);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height);
      text-align: center;
      transition:
        background-color var(--ugoe-duration-fast),
        border-color var(--ugoe-duration-fast);
    }

    /* 落とせる状態は色に加えて枠の線 (破線 → 実線) でも示す */
    .ugoe-drop-zone[data-dragging] {
      border-style: solid;
      border-color: var(--ugoe-color-accent);
      background: var(--ugoe-color-accent-soft);
    }

    .ugoe-drop-zone[data-disabled] {
      opacity: 0.5;
    }

    .ugoe-drop-zone__icon {
      display: inline-flex;
      color: var(--ugoe-color-text-muted);
      font-size: var(--ugoe-font-size-xl);
    }

    .ugoe-drop-zone__prompt {
      margin: 0;
    }

    .ugoe-drop-zone__hint {
      color: var(--ugoe-color-text-muted);
      font-size: var(--ugoe-font-size-sm);
    }

    .ugoe-drop-zone__input {
      display: none;
    }
  }
</style>
