<!--
  Kbd: ショートカットの表記 (docs/ui.md#部品の一覧)。自作 (<kbd>)。
  `keys` は `"Mod+Shift+Z"` のように書き、macOS では ⌘⇧Z、ほかでは Ctrl+Shift+Z と出す (src/shortcut.ts)。
  キーの組み合わせは、HTML の決まりに従って <kbd> の中に 1 キーずつ <kbd> を入れる。
-->
<script lang="ts">
import { currentPlatform, type Platform, shortcutKeys } from "../shortcut";

interface Props {
  /** ショートカット (`"Mod+S"`・`"Shift+ArrowUp"` など。キーの名前は KeyboardEvent.key の値)。 */
  keys: string;
  /** 表記の区分。既定は今の環境。 */
  platform?: Platform;
}

let { keys, platform }: Props = $props();

const resolved = $derived(platform ?? currentPlatform());
const parts = $derived(shortcutKeys(keys, resolved));
</script>

<kbd class="ugoe-kbd" data-platform={resolved}
  >{#each parts as part, i (i)}{#if i > 0 && resolved !== "mac"}<span
        class="ugoe-kbd__plus"
        aria-hidden="true">+</span
      >{/if}<kbd class="ugoe-kbd__key">{part}</kbd>{/each}</kbd
>

<style>
  :global {
    .ugoe-kbd {
      display: inline-flex;
      align-items: center;
      gap: var(--ugoe-space-1);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-xs);
      line-height: var(--ugoe-line-height-control);
      white-space: nowrap;
    }

    .ugoe-kbd__key {
      display: inline-block;
      min-width: 1.5em;
      box-sizing: border-box;
      padding: 1px var(--ugoe-space-1);
      border: 1px solid var(--ugoe-color-control-border);
      border-bottom-width: 2px;
      border-radius: var(--ugoe-radius-sm);
      background: var(--ugoe-color-sunken);
      color: var(--ugoe-color-text);
      font: inherit;
      text-align: center;
    }

    .ugoe-kbd__plus {
      color: var(--ugoe-color-text-muted);
    }
  }
</style>
