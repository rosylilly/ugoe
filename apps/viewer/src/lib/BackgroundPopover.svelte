<!--
  ツールバーの「背景」ボタン。背景の色と「透明」は、まれな操作なので Popover の奥に置く
  (docs/ui.md#ビューアの見直し-t0725。T0737)。ボタンには今の背景 (透明なら市松) の見本を出す。
-->
<script lang="ts">
import { Button, type ButtonBaseProps, ColorPicker, Popover, Toggle } from "@ugoe/ui";
import { t } from "./i18n";

interface Props {
  /** 背景の色 (`#rrggbb`)。 */
  color: string;
  /** 背景を透明にするか。 */
  transparent: boolean;
  /** 色か「透明」が変わったとき。 */
  onchange: (color: string, transparent: boolean) => void;
}

let { color, transparent, onchange }: Props = $props();
</script>

<Popover label={t("toolbar.background.label")}>
  {#snippet trigger(attrs)}
    <Button {...attrs as ButtonBaseProps}>
      <span class="swatch" class:transparent style:background={transparent ? undefined : color} aria-hidden="true"
      ></span>
      {t("toolbar.background.label")}
    </Button>
  {/snippet}
  <div class="background">
    <ColorPicker
      label={t("toolbar.background.color")}
      value={color}
      onchange={(next) => onchange(next, transparent)}
    />
    <Toggle label={t("toolbar.transparent.label")} checked={transparent} onchange={(on) => onchange(color, on)} />
  </div>
</Popover>

<style>
  .swatch {
    width: var(--ugoe-icon-size);
    height: var(--ugoe-icon-size);
    box-sizing: border-box;
    border: 1px solid var(--ugoe-color-control-border);
    border-radius: var(--ugoe-radius-sm);
  }
  /* 透明は市松で表す */
  .swatch.transparent {
    background: repeating-conic-gradient(var(--ugoe-color-border) 0 25%, var(--ugoe-color-surface) 0 50%) 50% / 8px 8px;
  }
  .background {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--ugoe-space-3);
  }
</style>
