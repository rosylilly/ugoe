<!--
  ビューポート: ugoe-wasm でモデルを canvas に描き、その上にオーバーレイを重ねる。

  操作: ホイールでカーソル位置を中心にズーム、中ボタン (または Alt + 左ボタン) のドラッグか Alt + 矢印キーでパン
  (矢印キーはフォーカスがどこにも無いかビューポートの中にあるときだけ。tools/keys.ts の panOf)。右下の表示の道具
  (縮小・拡大・全体を表示) はビューの中央を中心に拡大し、キャンバス全体を表示する (ビューアと同じ並び。docs/editor.md#ビューポート)。
  右上の「Screenshot」で今の表示を PNG で保存する (saveScreenshot.ts。WebPuppet.read_pixels で
  取るので、オーバーレイは写らない。docs/editor.md#スクリーンショット)。左ボタンはツール (オーバーレイ) のために空けておく。
  GPU を失って戻るのを待つ間は右上に「GPU の復元を待っています」を、続けて失って諦めた・作れなかったら「作り直す」ボタンを出す
  (controller.canRestart。docs/ui.md#webpuppet-の寿命。T0938・T0941)。描画の方式 (WebGPU・WebGL2) は開発者向けの情報なので、
  設定の「一般」に出す (T0675)。

  ビューポートの上の図の色は、ここで CSS の変数 (`--viewport-*`) に決める (テーマに依らない。docs/ui.md#ビューポートの上の図)。

  オーバーレイ (ギズモ・選択枠・メッシュ編集など、T0606 以降):
  `overlay` スニペットに描く。ViewportController を受け取り、`controller.transform`
  (キャンバス座標 → CSS ピクセル) や `canvasToScreen` / `hitTest` を使って配置する。
  オーバーレイの層は pointer-events: none なので、入力を受ける要素には pointer-events: auto を付ける。
-->
<script lang="ts">
import { Button, currentTheme, Icon, IconButton, Notice, showToast, Toolbar } from "@ugoe/ui";
import { type Snippet, untrack } from "svelte";
import { t } from "../i18n";
import TipButton from "../ui/TipButton.svelte";
import type { ViewportController, ViewportModel } from "./controller.svelte";
import { saveViewportScreenshot, screenshotErrorText } from "./saveScreenshot";
import { acceptsNudgeKey, panOf } from "./tools/keys";

interface Props {
  controller: ViewportController;
  /** 表示するモデル。変わるたびに読み直す。 */
  model: ViewportModel | null;
  overlay?: Snippet<[ViewportController]>;
  /** 中央に出す空の状態 (空のプロジェクトでの次の操作の文とボタン。docs/ui.md#デザイン の「空の状態」)。 */
  empty?: Snippet;
}

let { controller, model, overlay, empty }: Props = $props();

let container: HTMLDivElement;
/** canvas を置く要素 (canvas は controller が作り、GPU を失ったら作り直す)。 */
let surface = $state<HTMLDivElement>();
let panning: { id: number; x: number; y: number } | null = null;
/** スクリーンショットを保存している間 (ボタンを押せない)。 */
let capturing = $state(false);
/** 右上の HUD (スクリーンショット・GPU の知らせ) の幅。左上の道具の並びがこれと重ならないよう、
 * `--viewport-hud-width` で上に重ねるもの (ToolOverlay の .strip・提案のプレビューの帯) に渡す (T0907)。 */
let hudWidth = $state(0);
/** 右下の表示の道具の幅。左下に重ねる帯 (グルー・違いの知らせ) に `--viewport-view-tools-width` で渡す。 */
let viewToolsWidth = $state(0);
/** 表示の道具の 1 回の拡大率 (ビューアと同じ)。 */
const ZOOM_STEP = 1.25;
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

/** ビューの中央を中心に `factor` 倍する (表示の道具から。ホイールの代わり)。 */
function zoomBy(factor: number) {
  controller.zoomAt(factor, container.clientWidth / 2, container.clientHeight / 2);
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

/** Alt + 矢印キーでパンする (ドラッグの代わり。docs/editor.md#ビューポートの操作と代わりの手段)。 */
function onKeyDown(e: KeyboardEvent) {
  const delta = acceptsNudgeKey(e.target) ? panOf(e) : null;
  if (!delta) return;
  e.preventDefault();
  controller.panBy(delta[0], delta[1]);
}
</script>

<svelte:window onkeydown={onKeyDown} />

<div
  class="viewport"
  data-viewport
  bind:this={container}
  style:--viewport-hud-width="{hudWidth}px"
  style:--viewport-view-tools-width="{viewToolsWidth}px"
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
  {#if empty}<div class="empty">{@render empty()}</div>{/if}
  <div class="hud" bind:clientWidth={hudWidth}>
    {#if controller.status !== "ready"}
      <span class={controller.status === "error" ? "error" : "muted"}>
        {controller.message ?? t("viewport.loading")}
      </span>
      {#if controller.canRestart}
        <Button small onclick={() => void controller.restart()}>{t("viewport.restart")}</Button>
      {/if}
    {/if}
    <TipButton
      small
      onclick={() => void screenshot()}
      disabled={controller.status !== "ready" || capturing}
      tooltip={t("viewport.screenshot.tooltip")}>{t("viewport.screenshot.label")}</TipButton
    >
  </div>
  {#if controller.status === "ready" && !empty}
    <!-- 表示の道具の上のホイール・押下はビューポートのズーム・パンにしない -->
    <div
      class="view-tools"
      role="presentation"
      bind:clientWidth={viewToolsWidth}
      onpointerdown={(e) => e.stopPropagation()}
      onwheel={(e) => e.stopPropagation()}
    >
      <Toolbar label={t("viewport.view.aria")}>
        <IconButton small label={t("viewport.zoomOut.label")} onclick={() => zoomBy(1 / ZOOM_STEP)}>
          <Icon name="minus" />
        </IconButton>
        <IconButton small label={t("viewport.zoomIn.label")} onclick={() => zoomBy(ZOOM_STEP)}>
          <Icon name="plus" />
        </IconButton>
        <IconButton small label={t("viewport.fit.label")} onclick={() => controller.fit()}>
          <Icon name="fit" />
        </IconButton>
      </Toolbar>
    </div>
  {/if}
  {#if shotError}
    <div class="shot-notice">
      <Notice tone="danger" ondismiss={() => (shotError = null)}>{shotError}</Notice>
    </div>
  {/if}
</div>

<style>
.viewport {
  /*
   * ビューポートの上の図の色 (docs/ui.md#ビューポートの上の図)。テーマに依らない値で、ライトとダークの地
   * (--ugoe-color-sunken の #e4e4e7・#111113) のどちらにも 3:1 以上の中ほどの明るさの色を線に使う。
   * 点は白の中と線の色の縁で描き、明るい地でも暗い地でもモデルの上でも見えるようにする。
   */
  --viewport-edit: #8b5cf6; /* 選択の輪郭・メッシュの辺と点・フォーム編集 */
  --viewport-warp: #0284c7; /* ワープの格子と制御点 */
  --viewport-rotation: #db2777; /* 回転のギズモ */
  --viewport-base: #71717a; /* モジュールのプロジェクトの本体の要素 */
  --viewport-point: #ffffff; /* 点の中 */
  --viewport-picked: #f59e0b; /* 選んだ点の中 */
  --viewport-picked-edge: #92400e; /* 選んだ点の縁 */
  --viewport-glue-a: #15803d;
  --viewport-glue-b: #9333ea;
  --viewport-glue-pair: #d9480f;
  --viewport-collider: #2563eb; /* 物理の衝突体 */
  --viewport-chain: #d9480f; /* 物理の粒子の鎖 */
  --viewport-chain-selected: #dc2626;
  --viewport-overflow: #dc2626; /* はみ出しの印 */
  --viewport-highlight: #d9480f; /* プレビューの違いの強調 */
  --viewport-light: #facc15; /* 光源 (黄色は明るい地に弱いので、暗い縁取り --viewport-halo を付けて描く) */
  --viewport-halo: rgb(0 0 0 / 0.6);
  --viewport-ink: #1f2937; /* 色の付いた取っ手の縁 (光源の取っ手) */
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

/* 右上の HUD の下から下端までの間の中央。低い画面では中で送る */
.empty {
  position: absolute;
  inset: calc(var(--ugoe-target-min) + var(--ugoe-space-5)) var(--ugoe-space-4) var(--ugoe-space-4);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: safe center;
  gap: var(--ugoe-space-3);
  overflow-y: auto;
  text-align: center;
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
  max-width: 50%;
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  align-items: center;
  gap: var(--ugoe-space-2);
}

/* GPU の知らせはモデルの上に重なることがあるので、テーマの地に載せる */
.hud > span {
  padding: 0 var(--ugoe-space-1);
  border-radius: var(--ugoe-radius-sm);
  background: var(--ugoe-color-surface);
}

/* ビューアの表示の道具と同じ右下の位置 */
.view-tools {
  position: absolute;
  right: 8px;
  bottom: 8px;
  padding: var(--ugoe-space-1);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
  background: var(--ugoe-color-surface);
  box-shadow: var(--ugoe-shadow-overlay);
  cursor: default;
}
</style>
