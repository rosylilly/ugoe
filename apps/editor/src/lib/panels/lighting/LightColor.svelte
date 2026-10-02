<!--
  光源の色の欄 (PreviewLightsSection)。@ugoe/ui の ColorPicker を包み、色を選んでいる間は表示だけを変え (`onpreview`)、
  選ぶ枠を閉じたときにプロジェクトに残す (`oncommit`)。前の `<input type="color">` の input / change と同じ分け方。
-->
<script lang="ts">
import { ColorPicker } from "@ugoe/ui";
import { hexToRgb, type Rgb, rgbToHex } from "./lighting";

interface Props {
  value: Rgb;
  /** 読み上げの名前 (翻訳を通した文言)。 */
  ariaLabel: string;
  onpreview: (color: Rgb) => void;
  oncommit: (color: Rgb) => void;
}

let { value, ariaLabel, onpreview, oncommit }: Props = $props();

let open = $state(false);
/** 枠を開いている間に選んだ、まだ残していない色。 */
let pending: Rgb | null = null;

$effect(() => {
  if (!open && pending) {
    const color = pending;
    pending = null;
    oncommit(color);
  }
});

function change(hex: string): void {
  const color = hexToRgb(hex);
  if (!color) return;
  pending = color;
  if (open) onpreview(color);
  else {
    // 枠を通さずに変わったとき (無いはずだが) はすぐ残す。
    pending = null;
    oncommit(color);
  }
}
</script>

<!-- 行が狭いので、色の値 (#rrggbb) は見せずに見本の色だけを出す (読み上げの名前には値が入る)。 -->
<span class="light-color">
  <ColorPicker value={rgbToHex(value)} {ariaLabel} bind:open onchange={change} />
</span>

<style>
.light-color :global(.ugoe-color-picker__value) {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
</style>
