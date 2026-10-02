<!--
  Menu: 操作の一覧をボタンから開く (ファイルのメニュー・「その他」のボタン) (docs/ui.md#部品の一覧)。
  土台は Bits UI の DropdownMenu (上下の矢印・Enter・Esc・頭文字で移る。閉じたらトリガーへフォーカスを戻す)。
  選択肢から 1 つを選ぶなら Select を使う。右クリックのメニューは ContextMenu。

  トリガーは次のどれか:
  - 既定: `label` を出すボタン (下向きの印付き)
  - `icon` を渡す: IconButton (`label` は読み上げの名前とツールチップ)
  - `trigger` を渡す: 自前の要素。受け取った属性を、フォーカスを受ける要素に広げる
  一覧 (role="menu") の読み上げの名前も `label` にする (Bits UI 2.19.4 はトリガーと結び付けないため)。
-->
<script lang="ts">
import { DropdownMenu } from "bits-ui";
import type { Snippet } from "svelte";
import type { TriggerAttributes } from "../props";
import type { ButtonBaseProps, ButtonVariant, MenuEntry, Side } from "../types";
import Button from "./Button.svelte";
import Icon from "./Icon.svelte";
import IconButton from "./IconButton.svelte";
import MenuEntries from "./MenuEntries.svelte";

interface Props {
  /** 項目。 */
  entries: MenuEntry[];
  /** トリガーの文言 (翻訳を通した文言)。`icon` のときは読み上げの名前。 */
  label: string;
  /** トリガーを IconButton にするときのアイコン。 */
  icon?: Snippet;
  /** 自前のトリガー。 */
  trigger?: Snippet<[TriggerAttributes]>;
  /** トリガーのボタンの種類。 */
  variant?: ButtonVariant;
  open?: boolean;
  disabled?: boolean;
  /** トリガーのどちら側に出すか。既定は下。 */
  side?: Side;
  /** トリガーのどちらの端にそろえるか。既定は始まり。 */
  align?: "start" | "center" | "end";
}

let {
  entries,
  label,
  icon,
  trigger,
  variant,
  open = $bindable(false),
  disabled = false,
  side = "bottom",
  align = "start",
}: Props = $props();
</script>

<DropdownMenu.Root bind:open>
  <DropdownMenu.Trigger {disabled}>
    {#snippet child({ props })}
      {#if trigger}
        {@render trigger(props)}
      {:else if icon}
        <IconButton {...props as ButtonBaseProps} {label} {variant}>{@render icon()}</IconButton>
      {:else}
        <Button {...props as ButtonBaseProps} {variant}>
          {label}
          <Icon name="chevronDown" />
        </Button>
      {/if}
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <DropdownMenu.Content
      class="ugoe-menu"
      aria-label={label}
      {side}
      {align}
      sideOffset={4}
      collisionPadding={8}
    >
      <MenuEntries {entries} />
    </DropdownMenu.Content>
  </DropdownMenu.Portal>
</DropdownMenu.Root>
