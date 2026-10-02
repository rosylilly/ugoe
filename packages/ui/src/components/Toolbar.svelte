<!--
  Toolbar: ボタンの並び (docs/ui.md#部品の一覧)。土台は Bits UI の Toolbar
  (全体で 1 つのタブ止まり。中は矢印で移る)。中には Button・IconButton を置く。
-->
<script lang="ts">
import { Toolbar } from "bits-ui";
import type { Snippet } from "svelte";
import { setInToolbar } from "../context";

interface Props {
  /** 読み上げの名前 (翻訳を通した文言)。 */
  label: string;
  orientation?: "horizontal" | "vertical";
  children: Snippet;
}

let { label, orientation = "horizontal", children }: Props = $props();

setInToolbar();
</script>

<Toolbar.Root class="ugoe-toolbar" {orientation} aria-label={label}>
  {@render children()}
</Toolbar.Root>

<style>
  :global {
    .ugoe-toolbar {
      display: flex;
      align-items: center;
      gap: var(--ugoe-space-1);
      padding: var(--ugoe-space-1);
    }

    .ugoe-toolbar[data-orientation="vertical"] {
      flex-direction: column;
    }
  }
</style>
