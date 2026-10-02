<!--
  IconButton: 文字を出す場所が無いときのボタン (docs/ui.md#部品の一覧)。
  `label` は読み上げの名前 (aria-label) で、必ず渡す。ツールチップは T0903 で足す。
  アイコンは children に SVG などで渡す (飾りなので aria-hidden にする)。
-->
<script lang="ts">
import type { Snippet } from "svelte";
import type { ButtonBaseProps } from "../types";
import Button from "./Button.svelte";

interface Props extends ButtonBaseProps {
  /** 読み上げの名前。翻訳を通した文言を渡す。空にしない。 */
  label: string;
  children: Snippet;
}

let { label, variant = "ghost", class: className = "", children, ...rest }: Props = $props();

$effect(() => {
  if (label.trim() === "")
    console.error("@ugoe/ui IconButton: label が空 (aria-label を必ず付ける)");
});
</script>

<Button {...rest} {variant} class={`ugoe-icon-button ${className}`} aria-label={label}>
  <span class="ugoe-icon-button__icon" aria-hidden="true">{@render children()}</span>
</Button>

<style>
  :global {
    .ugoe-button.ugoe-icon-button {
      width: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 0;
    }

    .ugoe-icon-button__icon {
      display: inline-flex;
      width: 1.15em;
      height: 1.15em;
      align-items: center;
      justify-content: center;
    }

    .ugoe-icon-button__icon > svg {
      width: 100%;
      height: 100%;
    }
  }
</style>
