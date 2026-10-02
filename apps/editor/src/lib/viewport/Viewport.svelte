<!--
  ビューポート: ugoe-wasm でモデルを canvas に描き、その上にオーバーレイを重ねる。

  操作: ホイールでカーソル位置を中心にズーム、中ボタン (または Alt + 左ボタン) のドラッグでパン、
  「Fit」でキャンバス全体を表示。「Screenshot」で今の表示を PNG で保存する (saveScreenshot.ts。WebPuppet.read_pixels で
  取るので、オーバーレイは写らない。docs/editor.md#スクリーンショット)。左ボタンはツール (オーバーレイ) のために空けておく。

  オーバーレイ (ギズモ・選択枠・メッシュ編集など、T0606 以降):
  `overlay` スニペットに描く。ViewportController を受け取り、`controller.transform`
  (キャンバス座標 → CSS ピクセル) や `canvasToScreen` / `hitTest` を使って配置する。
  オーバーレイの層は pointer-events: none なので、入力を受ける要素には pointer-events: auto を付ける。
-->
<script lang="ts">
import { type Snippet, untrack } from "svelte";
import type { ViewportController, ViewportModel } from "./controller.svelte";
import { saveViewportScreenshot } from "./saveScreenshot";

interface Props {
  controller: ViewportController;
  /** 表示するモデル。変わるたびに読み直す。 */
  model: ViewportModel | null;
  overlay?: Snippet<[ViewportController]>;
  /** 中央に出す案内 (空のプロジェクトでの次の操作など)。 */
  hint?: string | null;
}

let { controller, model, overlay, hint = null }: Props = $props();

let container: HTMLDivElement;
let canvas = $state<HTMLCanvasElement>();
let panning: { id: number; x: number; y: number } | null = null;
/** スクリーンショットを保存している間 (ボタンを押せない)。 */
let capturing = $state(false);
/** 直近のスクリーンショットの結果 (保存した場所かエラー)。しばらくしたら消す。 */
let shotNotice = $state<{ text: string; error: boolean } | null>(null);
let shotNoticeTimer: ReturnType<typeof setTimeout> | undefined;

function showShotNotice(text: string, error: boolean) {
  shotNotice = { text, error };
  clearTimeout(shotNoticeTimer);
  shotNoticeTimer = setTimeout(() => (shotNotice = null), error ? 10000 : 5000);
}

/** 今の表示を PNG で保存する (オーバーレイは写らない)。 */
async function screenshot() {
  if (capturing) return;
  capturing = true;
  try {
    const path = await saveViewportScreenshot(controller);
    if (path !== null) showShotNotice(`Saved screenshot to ${path}`, false);
  } catch (e) {
    showShotNotice(
      `Cannot save the screenshot: ${e instanceof Error ? e.message : String(e)}`,
      true,
    );
  } finally {
    capturing = false;
  }
}

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
  {#if hint}<p class="hint muted">{hint}</p>{/if}
  <div class="hud">
    {#if controller.status !== "ready"}
      <span class={controller.status === "error" ? "error" : "muted"}>
        {controller.message ?? "Loading viewport…"}
      </span>
    {:else}
      <span class="muted">{controller.backend}</span>
    {/if}
    <button onclick={() => void screenshot()} disabled={controller.status !== "ready" || capturing}
      title="Save the current view as a PNG image (without gizmos and other overlays)">Screenshot</button>
    <button onclick={() => controller.fit()}>Fit</button>
  </div>
  {#if shotNotice}
    <p class="shot-notice" class:error={shotNotice.error} role="status">{shotNotice.text}</p>
  {/if}
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

.hint {
  position: absolute;
  inset: 50% 16px auto;
  transform: translateY(-50%);
  margin: 0;
  text-align: center;
  pointer-events: none;
}

.shot-notice {
  position: absolute;
  top: 40px;
  right: 8px;
  max-width: calc(100% - 16px);
  margin: 0;
  padding: 4px 8px;
  overflow-wrap: anywhere;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 4px;
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
