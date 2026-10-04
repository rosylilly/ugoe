<!--
  Tooltip: ボタンやアイコンの短い説明 (docs/ui.md#部品の一覧)。土台は Bits UI の Tooltip
  (ポインタを乗せる・フォーカスで出る、Esc とポインタの押下で消える)。
  `title` 属性の代わりに使う。中に操作できるものは置かない (置くなら Popover)。

  トリガーは `trigger` のスニペットで描き、受け取った属性を、フォーカスを受ける要素 (ボタンなど) に広げる:

    <Tooltip text={t("view.fit.tooltip")}>
      {#snippet trigger(attrs)}<button {...attrs}>…</button>{/snippet}
    </Tooltip>

  開いている間、トリガーの aria-describedby が説明を指す (IconButton は説明がラベルと同じなので外す)。
  Bits UI 2.19.4 の Tooltip.Content は role="tooltip" を付けないので、ここで付ける (APG の Tooltip パターン)。
-->
<script lang="ts">
import { Tooltip } from "bits-ui";
import type { Snippet } from "svelte";
import { isInTooltipProvider } from "../context";
import type { TriggerAttributes } from "../props";
import type { Side } from "../types";
import Kbd from "./Kbd.svelte";

interface Props {
  /** 説明 (翻訳を通した文言)。 */
  text: string;
  /** 説明の後ろに出すショートカット (Kbd の書き方)。 */
  shortcut?: string;
  /** トリガーのどちら側に出すか。既定は下。 */
  side?: Side;
  /** 出さない。 */
  disabled?: boolean;
  /** トリガーの要素の id (省くと作る)。 */
  id?: string;
  /** 出るまでの時間 (ミリ秒)。省くと TooltipProvider の値 (無ければ 500)。 */
  delay?: number;
  /** トリガー。受け取った属性を、フォーカスを受ける要素に広げる。 */
  trigger: Snippet<[TriggerAttributes]>;
}

let { text, shortcut, side = "bottom", disabled = false, id, delay, trigger }: Props = $props();

const inProvider = isInTooltipProvider();
</script>

{#snippet tooltip()}
  <Tooltip.Root {disabled} delayDuration={delay}>
    <Tooltip.Trigger {id} {disabled}>
      {#snippet child({ props })}
        {@render trigger(props)}
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Portal>
      <Tooltip.Content
        class="ugoe-tooltip"
        role="tooltip"
        {side}
        sideOffset={6}
        collisionPadding={8}
      >
        <span>{text}</span>
        {#if shortcut}<Kbd keys={shortcut} />{/if}
      </Tooltip.Content>
    </Tooltip.Portal>
  </Tooltip.Root>
{/snippet}

{#if inProvider}
  {@render tooltip()}
{:else}
  <Tooltip.Provider delayDuration={500}>{@render tooltip()}</Tooltip.Provider>
{/if}

<style>
  :global {
    .ugoe-tooltip {
      z-index: var(--ugoe-z-popover);
      display: inline-flex;
      align-items: center;
      gap: var(--ugoe-space-2);
      max-width: min(320px, calc(100vw - 16px));
      box-sizing: border-box;
      padding: var(--ugoe-space-1) var(--ugoe-space-2);
      border: 1px solid var(--ugoe-color-border);
      border-radius: var(--ugoe-radius-md);
      background: var(--ugoe-color-surface);
      box-shadow: var(--ugoe-shadow-overlay);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-sm);
      line-height: var(--ugoe-line-height);
      animation: ugoe-fade-in var(--ugoe-duration-fast) ease-out;
    }

    @keyframes ugoe-fade-in {
      from {
        opacity: 0;
      }
    }
  }
</style>
