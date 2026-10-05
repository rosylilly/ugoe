<!--
  IconButton: 文字を出す場所が無いときのボタン (docs/ui.md#部品の一覧)。
  `label` は読み上げの名前 (aria-label) で、必ず渡す。同じ文言を Tooltip で出す (`tooltip` で変えられる)。
  ツールチップは名前と同じ文言なので、aria-describedby で二重に読ませない。
  アイコンは children に SVG などで渡す (飾りなので aria-hidden にする)。
-->
<script lang="ts">
import type { Snippet } from "svelte";
import { mergeAttributes, type TriggerAttributes } from "../props";
import { ariaKeyShortcuts } from "../shortcut";
import type { ButtonBaseProps, Side } from "../types";
import Button from "./Button.svelte";
import Tooltip from "./Tooltip.svelte";

interface Props extends ButtonBaseProps {
  /** 読み上げの名前。翻訳を通した文言を渡す。空にしない。 */
  label: string;
  /** ツールチップの文言。既定は `label`。`false` で出さない (近くに同じ文言が見えているときなど)。 */
  tooltip?: string | false;
  /** ショートカット (Kbd の書き方)。ツールチップに出し、aria-keyshortcuts にする。 */
  shortcut?: string;
  /** ツールチップをどちら側に出すか。 */
  tooltipSide?: Side;
  children: Snippet;
}

let {
  label,
  tooltip,
  shortcut,
  tooltipSide,
  variant = "ghost",
  class: className = "",
  children,
  ...rest
}: Props = $props();

$effect(() => {
  if (label.trim() === "")
    console.error("@ugoe/ui IconButton: label が空 (aria-label を必ず付ける)");
});

const tip = $derived(tooltip === false ? "" : (tooltip ?? label));

/** ボタンに渡す属性。Menu・Popover のトリガーにしたときは、その属性 (rest に入る) も重ねる。 */
function attributes(trigger: TriggerAttributes): ButtonBaseProps {
  const merged = mergeAttributes(trigger, rest as TriggerAttributes, {
    variant,
    class: `ugoe-icon-button ${className}`,
    "aria-label": label,
    "aria-keyshortcuts": shortcut ? ariaKeyShortcuts(shortcut) : undefined,
  });
  // 説明がラベルと同じなので、ツールチップの aria-describedby は外す (呼び手が渡したものは残す)
  merged["aria-describedby"] = rest["aria-describedby"];
  return merged as ButtonBaseProps;
}
</script>

{#snippet button(trigger: TriggerAttributes)}
  <Button {...attributes(trigger)}>
    <span class="ugoe-icon-button__icon" aria-hidden="true">{@render children()}</span>
  </Button>
{/snippet}

{#if tip}
  <Tooltip text={tip} {shortcut} side={tooltipSide} id={rest.id} disabled={rest.disabled || rest.busy}>
    {#snippet trigger(attrs)}{@render button(attrs)}{/snippet}
  </Tooltip>
{:else}
  {@render button({})}
{/if}

<style>
  :global {
    .ugoe-button.ugoe-icon-button {
      width: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 0;
    }

    .ugoe-icon-button__icon {
      display: inline-flex;
      width: var(--ugoe-icon-size);
      height: var(--ugoe-icon-size);
      align-items: center;
      justify-content: center;
    }

    .ugoe-icon-button__icon > svg {
      width: 100%;
      height: 100%;
    }
  }
</style>
