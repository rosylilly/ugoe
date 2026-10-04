<!--
  説明 (Tooltip) 付きの文字のボタン (docs/ui.md#エディタの中の共通の形)。`title` 属性の代わりに使う。
  `tooltip` を省くと @ugoe/ui の Button と同じ。無効のボタンには説明が出ないので、押せない理由は近くに書く。
  Popover・Menu のトリガーの属性 (rest に入る) も、Tooltip の属性と重ねて渡す (同じイベントは両方呼ぶ)。
-->
<script lang="ts">
import { Button, type ButtonBaseProps, mergeAttributes, Tooltip } from "@ugoe/ui";
import type { Snippet } from "svelte";

interface Props extends ButtonBaseProps {
  /** 説明 (翻訳を通した文言)。 */
  tooltip?: string;
  /** 説明の後ろに出すショートカット (Kbd の書き方。`Mod+Z`)。 */
  shortcut?: string;
  children: Snippet;
}

let { tooltip, shortcut, onclick, children, ...rest }: Props = $props();
</script>

{#if tooltip}
  <Tooltip text={tooltip} {shortcut}>
    {#snippet trigger(attrs)}
      <Button {...mergeAttributes(attrs, rest, { onclick }) as ButtonBaseProps}>{@render label()}</Button>
    {/snippet}
  </Tooltip>
{:else}
  <Button {...rest} {onclick}>{@render label()}</Button>
{/if}

{#snippet label()}{@render children()}{/snippet}
