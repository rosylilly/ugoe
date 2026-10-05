<!--
  ビューポート: ugoe-wasm でモデルを canvas に描き、その上にオーバーレイを重ねる。

  操作: ホイールでカーソル位置を中心にズーム、中ボタン (または Alt + 左ボタン) のドラッグでパン、
  「Fit」でキャンバス全体を表示。「Screenshot」で今の表示を PNG で保存する (saveScreenshot.ts。WebPuppet.read_pixels で
  取るので、オーバーレイは写らない。docs/editor.md#スクリーンショット)。左ボタンはツール (オーバーレイ) のために空けておく。
  GPU を失って戻るのを待つ間は右上に「GPU の復元を待っています」を、続けて失って諦めたら「作り直す」ボタンを出す
  (controller.canRestart。docs/ui.md#webpuppet-の寿命。T0938)。

  オーバーレイ (ギズモ・選択枠・メッシュ編集など、T0606 以降):
  `overlay` スニペットに描く。ViewportController を受け取り、`controller.transform`
  (キャンバス座標 → CSS ピクセル) や `canvasToScreen` / `hitTest` を使って配置する。
  オーバーレイの層は pointer-events: none なので、入力を受ける要素には pointer-events: auto を付ける。
-->
<script lang="ts">
import { Button, currentTheme, Notice, showToast } from "@ugoe/ui";
import { type Snippet, untrack } from "svelte";
import { t } from "../i18n";
import TipButton from "../ui/TipButton.svelte";
import type { ViewportController, ViewportModel } from "./controller.svelte";
import { saveViewportScreenshot, screenshotErrorText } from "./saveScreenshot";

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
/** canvas を置く要素 (canvas は controller が作り、GPU を失ったら作り直す)。 */
let surface = $state<HTMLDivElement>();
let panning: { id: number; x: number; y: number } | null = null;
/** スクリーンショットを保存している間 (ボタンを押せない)。 */
let capturing = $state(false);
/** 右上の HUD (バックエンドの名前・スクリーンショット・全体を表示) の幅。左上の道具の並びがこれと重ならないよう、
 * `--viewport-hud-width` で上に重ねるもの (ToolOverlay の .toolbar・提案のプレビューの帯) に渡す (T0907)。 */
let hudWidth = $state(0);
/** 直近のスクリーンショットの失敗 (閉じるまで残す。保存した場所は Toast で出す)。 */
let shotError = $state<string | null>(null);

/** 今の表示を PNG で保存する (オーバーレイは写らない)。 */
async function screenshot() {
  if (capturing) return;
  capturing = true;
  try {
    const path = await saveViewportScreenshot(controller);
    shotError = null;
    if (path !== null) showToast(t("viewport.screenshot.saved", { path }), { tone: "success" });
  } catch (e) {
    shotError = t("viewport.screenshot.failed", { error: screenshotErrorText(e) });
  } finally {
    capturing = false;
  }
}

$effect(() => {
  const m = model;
  if (m) untrack(() => controller.setModel(m));
});

// canvas は controller が surface の中に作る (GPU を失ったときの作り直しも)。
$effect(() => {
  const target = surface;
  if (!target) return;
  untrack(() => controller.attach(target));
  return () => controller.detach();
});

// devicePixelRatio だけの変化 (ResizeObserver は知らせない) は controller (@ugoe/player の PuppetSurface) が聞く。
$effect(() => {
  const observer = new ResizeObserver(([entry]) => {
    if (!entry) return;
    const { width, height } = entry.contentRect;
    controller.resize(width, height);
  });
  observer.observe(container);
  return () => observer.disconnect();
});

// テーマ (設定・OS) が変わったら背景を読み直す。
$effect(() => {
  currentTheme();
  untrack(applyBackground);
});

/** ビューポートの地 (--ugoe-color-sunken) を描画の背景色にする (canvas が不透明にしか合成できない環境でも同じ色になる)。 */
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
  style:--viewport-hud-width="{hudWidth}px"
  role="presentation"
  onwheel={onWheel}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
>
  <div class="surface" bind:this={surface}></div>
  <div class="overlay">
    {#if overlay}{@render overlay(controller)}{/if}
  </div>
  {#if hint}<p class="hint muted">{hint}</p>{/if}
  <div class="hud" bind:clientWidth={hudWidth}>
    {#if controller.status !== "ready"}
      <span class={controller.status === "error" ? "error" : "muted"}>
        {controller.message ?? t("viewport.loading")}
      </span>
      {#if controller.canRestart}
        <Button small onclick={() => void controller.restart()}>{t("viewport.restart")}</Button>
      {/if}
    {:else}
      <span class="muted">{controller.backend}</span>
    {/if}
    <TipButton
      small
      onclick={() => void screenshot()}
      disabled={controller.status !== "ready" || capturing}
      tooltip={t("viewport.screenshot.tooltip")}>{t("viewport.screenshot.label")}</TipButton
    >
    <Button small onclick={() => controller.fit()}>{t("viewport.fit.label")}</Button>
  </div>
  {#if shotError}
    <div class="shot-notice">
      <Notice tone="danger" ondismiss={() => (shotError = null)}>{shotError}</Notice>
    </div>
  {/if}
</div>

<style>
.viewport {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--ugoe-color-sunken);
  touch-action: none;
}

.surface {
  position: absolute;
  inset: 0;
}

.surface :global(canvas) {
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
  overflow-wrap: anywhere;
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
