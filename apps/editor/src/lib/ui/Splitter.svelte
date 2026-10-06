<!--
  領域の大きさを変える区切り (APG の Window Splitter。docs/ui.md#レイアウト の「分割」、docs/editor.md#画面の配置)。
  - ポインタでドラッグ、キーボードで矢印 (線の向きに沿う 2 つ)・Home (最小)・End (最大)、ダブルクリックと Enter で既定の大きさに戻す。
  - 変えた大きさは手を離したとき・キーを押したときに覚える (lib/layout.ts の saveSize)。
  - `value` は領域の大きさ (px か割合)。null は CSS の既定の大きさで、動かし始めに `measure` で実際の大きさを測る。
-->
<script lang="ts">
import { type SizeRange, saveSize, sizeByKey } from "../layout";

interface Props {
  /** 読み上げの名前 (何と何の間か)。 */
  label: string;
  /** 線の向き。列の間は `vertical` (左右に動く)、上下の間は `horizontal`。 */
  orientation: "vertical" | "horizontal";
  range: SizeRange;
  value: number | null;
  /** 区切りが領域の手前 (右の列の左端・タイムラインの上端) にある。線を左 (上) へ動かすと大きくなる。 */
  reverse?: boolean;
  /** ポインタが 1 CSS px 動いたときの大きさの変化 (割合なら 1 / 列の高さ)。既定は 1。 */
  scale?: () => number;
  /** `value` が null のときの今の大きさ。 */
  measure?: () => number;
  /** 区切りが変える領域の id (`aria-controls`)。 */
  controls?: string;
}

let {
  label,
  orientation,
  range,
  value = $bindable(),
  reverse = false,
  scale = () => 1,
  measure = () => range.default ?? range.min,
  controls,
}: Props = $props();

let drag = $state<{ id: number; from: number; start: number } | null>(null);

const current = $derived(value ?? measure());
/** 割合は百分率で読み上げる。 */
const percent = $derived(range.max <= 1);
const shown = (v: number) => Math.round(percent ? v * 100 : v);

function position(e: PointerEvent): number {
  return orientation === "vertical" ? e.clientX : e.clientY;
}

function onpointerdown(e: PointerEvent): void {
  if (e.button !== 0) return;
  e.preventDefault();
  (e.currentTarget as HTMLElement).focus();
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  drag = { id: e.pointerId, from: position(e), start: value ?? measure() };
}

function onpointermove(e: PointerEvent): void {
  if (drag?.id !== e.pointerId) return;
  const moved = (position(e) - drag.from) * scale() * (reverse ? -1 : 1);
  value = Math.min(range.max, Math.max(range.min, drag.start + moved));
}

function onpointerup(e: PointerEvent): void {
  if (drag?.id !== e.pointerId) return;
  drag = null;
  saveSize(range, value ?? null);
}

function reset(): void {
  value = range.default;
  saveSize(range, null);
}

function onkeydown(e: KeyboardEvent): void {
  if (e.key === "Enter") {
    e.preventDefault();
    reset();
    return;
  }
  const next = sizeByKey(range, value ?? measure(), e.key, orientation, reverse);
  if (next === null) return;
  e.preventDefault();
  value = next;
  saveSize(range, next);
}
</script>

<!-- フォーカスできる区切り (ARIA の separator) はキー操作を受ける部品。 -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="splitter {orientation}"
  class:dragging={drag !== null}
  role="separator"
  aria-orientation={orientation}
  aria-label={label}
  aria-controls={controls}
  aria-valuemin={shown(range.min)}
  aria-valuemax={shown(range.max)}
  aria-valuenow={shown(current)}
  tabindex="0"
  {onpointerdown}
  {onpointermove}
  {onpointerup}
  onpointercancel={onpointerup}
  ondblclick={reset}
  {onkeydown}
></div>

<style>
  /* 見た目は細い線。押せる範囲は前後に広げる (::before)。 */
  .splitter {
    position: relative;
    z-index: 2;
    flex: none;
    background: var(--ugoe-color-border);
    touch-action: none;
  }
  .splitter.vertical {
    width: 1px;
    cursor: col-resize;
  }
  .splitter.horizontal {
    height: 1px;
    cursor: row-resize;
  }
  .splitter::before {
    content: "";
    position: absolute;
  }
  .splitter.vertical::before {
    inset: 0 calc(-1 * var(--ugoe-space-1));
  }
  .splitter.horizontal::before {
    inset: calc(-1 * var(--ugoe-space-1)) 0;
  }
  .splitter:hover,
  .splitter.dragging {
    background: var(--ugoe-color-accent);
    box-shadow: 0 0 0 1px var(--ugoe-color-accent);
  }
  .splitter:focus-visible {
    background: var(--ugoe-color-accent);
    outline: var(--ugoe-focus-ring);
    outline-offset: var(--ugoe-focus-offset);
  }
</style>
