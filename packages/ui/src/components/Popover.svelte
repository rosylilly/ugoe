<!--
  Popover: 部品の近くに出す小さな編集の枠 (色の選択など) (docs/ui.md#部品の一覧)。土台は Bits UI の Popover
  (Esc と外のクリックで閉じ、トリガーへフォーカスを戻す)。中にフォーカスを閉じ込めない (モーダルでない)。
  中身の読み上げの名前は `label`。

  トリガーは `trigger` のスニペットで描き、受け取った属性を、フォーカスを受ける要素 (ボタン) に広げる:

    <Popover label={t("light.color.label")}>
      {#snippet trigger(attrs)}<Button {...attrs}>…</Button>{/snippet}
      …中身…
    </Popover>
-->
<script lang="ts">
import { Popover } from "bits-ui";
import type { Snippet } from "svelte";
import type { TriggerAttributes } from "../props";
import type { Side } from "../types";

interface Props {
  /** 枠の読み上げの名前 (翻訳を通した文言)。 */
  label: string;
  open?: boolean;
  disabled?: boolean;
  side?: Side;
  align?: "start" | "center" | "end";
  /** 閉じたとき (どの方法でも)。 */
  onclose?: () => void;
  trigger: Snippet<[TriggerAttributes]>;
  children: Snippet;
}

let {
  label,
  open = $bindable(false),
  disabled = false,
  side = "bottom",
  align = "start",
  onclose,
  trigger,
  children,
}: Props = $props();
</script>

<Popover.Root
  bind:open
  onOpenChange={(v) => {
    if (!v) onclose?.();
  }}
>
  <Popover.Trigger {disabled}>
    {#snippet child({ props })}{@render trigger(props)}{/snippet}
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content
      class="ugoe-popover"
      {side}
      {align}
      sideOffset={4}
      collisionPadding={8}
      aria-label={label}
    >
      {@render children()}
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global {
    .ugoe-popover {
      z-index: var(--ugoe-z-popover);
      box-sizing: border-box;
      max-width: calc(100vw - 16px);
      padding: var(--ugoe-space-4);
      border: 1px solid var(--ugoe-color-border);
      border-radius: var(--ugoe-radius-lg);
      background: var(--ugoe-color-surface);
      box-shadow: var(--ugoe-shadow-overlay);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height);
      animation: ugoe-fade-in var(--ugoe-duration-fast) ease-out;
    }

    .ugoe-popover:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: 2px;
    }
  }
</style>
