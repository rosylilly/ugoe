<!--
  ビューポート: ugoe-wasm でモデルを canvas に描き、その上にオーバーレイを重ねる。

  操作: ホイールでカーソル位置を中心にズーム、中ボタン (または Alt + 左ボタン) のドラッグでパン、
  「Fit」でキャンバス全体を表示。左ボタンはツール (オーバーレイ) のために空けておく。

  オーバーレイ (ギズモ・選択枠・メッシュ編集など、T0606 以降):
  `overlay` スニペットに描く。ViewportController を受け取り、`controller.transform`
  (キャンバス座標 → CSS ピクセル) や `canvasToScreen` / `hitTest` を使って配置する。
  オーバーレイの層は pointer-events: none なので、入力を受ける要素には pointer-events: auto を付ける。
-->
<script lang="ts">
import { type Snippet, untrack } from "svelte";
import type { ViewportController, ViewportModel } from "./controller.svelte";

interface Props {
  controller: ViewportController;
  /** 表示するモデル。変わるたびに読み直す。 */
  model: ViewportModel | null;
  overlay?: Snippet<[ViewportController]>;
}

let { controller, model, overlay }: Props = $props();

let container: HTMLDivElement;
let canvas = $state<HTMLCanvasElement>();
let panning: { id: number; x: number; y: number } | null = null;

$effect(() => {
  const m = model;
  if (m) untrack(() => controller.setModel(m));
});

// canvas は controller.generation が変わるたびに作り直される ({#key})。
$effect(() => {
  const target = canvas;
  if (!target) return;
  untrack(() => controller.attach(target));
  return () => controller.detach();
});

$effect(() => {
  const observer = new ResizeObserver(([entry]) => {
    if (!entry) return;
    const { width, height } = entry.contentRect;
    controller.resize(width, height, window.devicePixelRatio || 1);
  });
  observer.observe(container);
  applyBackground();
  const scheme = window.matchMedia("(prefers-color-scheme: dark)");
  scheme.addEventListener("change", applyBackground);
  return () => {
    observer.disconnect();
    scheme.removeEventListener("change", applyBackground);
  };
});

/** CSS の --viewport-bg を描画の背景色にする (canvas が不透明にしか合成できない環境でも同じ色になる)。 */
function applyBackground() {
  const color = getComputedStyle(container).backgroundColor;
  const m = color.match(/rgba?\(([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)/);
  if (!m) return;
  const [r, g, b] = [m[1], m[2], m[3]].map((v) => Number(v) / 255) as [number, number, number];
  controller.setBackground(r, g, b, m[4] === undefined ? 1 : Number(m[4]));
}

function onWheel(e: WheelEvent) {
  e.preventDefault();
  const rect = container.getBoundingClientRect();
  const delta = e.deltaMode === WheelEvent.DOM_DELTA_LINE ? e.deltaY * 16 : e.deltaY;
  controller.zoomAt(Math.exp(-delta * 0.0015), e.clientX - rect.left, e.clientY - rect.top);
}

function onPointerDown(e: PointerEvent) {
  if (e.button === 1 || (e.button === 0 && e.altKey)) {
    e.preventDefault();
    panning = { id: e.pointerId, x: e.clientX, y: e.clientY };
    container.setPointerCapture(e.pointerId);
  }
}

function onPointerMove(e: PointerEvent) {
  if (panning?.id !== e.pointerId) return;
  controller.panBy(e.clientX - panning.x, e.clientY - panning.y);
  panning.x = e.clientX;
  panning.y = e.clientY;
}

function onPointerUp(e: PointerEvent) {
  if (panning?.id === e.pointerId) panning = null;
}
</script>

<div
  class="viewport"
  bind:this={container}
  role="presentation"
  onwheel={onWheel}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
>
  {#key controller.generation}
    <canvas bind:this={canvas}></canvas>
  {/key}
  <div class="overlay">
    {#if overlay}{@render overlay(controller)}{/if}
  </div>
  <div class="hud">
    {#if controller.status !== "ready"}
      <span class={controller.status === "error" ? "error" : "muted"}>
        {controller.message ?? "Loading viewport…"}
      </span>
    {:else}
      <span class="muted">{controller.backend}</span>
    {/if}
    <button onclick={() => controller.fit()}>Fit</button>
  </div>
</div>

<style>
.viewport {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--viewport-bg);
  touch-action: none;
}

canvas {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  display: block;
}

.overlay {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.hud {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
