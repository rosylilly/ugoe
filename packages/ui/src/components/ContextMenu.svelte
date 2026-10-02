<!--
  ContextMenu: 右クリック (と Shift+F10・メニューキー) で開く操作の一覧 (docs/ui.md#部品の一覧)。土台は Bits UI の ContextMenu。
  `children` が右クリックを受ける領域。右クリックでしかできない操作を作らない (同じ操作をボタンやメニューにも置く)。
-->
<script lang="ts">
import { ContextMenu } from "bits-ui";
import type { Snippet } from "svelte";
import type { MenuEntry } from "../types";
import MenuEntries from "./MenuEntries.svelte";

interface Props {
  entries: MenuEntry[];
  /** 一覧 (role="menu") の読み上げの名前 (翻訳を通した文言。「レイヤーの操作」など)。 */
  label: string;
  open?: boolean;
  disabled?: boolean;
  /** 右クリックを受ける領域の中身。 */
  children: Snippet;
  class?: string;
}

let {
  entries,
  label,
  open = $bindable(false),
  disabled = false,
  children,
  class: className = "",
}: Props = $props();
</script>

<ContextMenu.Root bind:open>
  <ContextMenu.Trigger class={`ugoe-context-menu__area ${className}`} {disabled}>
    {@render children()}
  </ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content class="ugoe-menu" aria-label={label} collisionPadding={8}>
      <MenuEntries {entries} />
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>
