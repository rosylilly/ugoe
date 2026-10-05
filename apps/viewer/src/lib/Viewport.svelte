<script lang="ts">
// モデルを描く領域。ホイールでズーム、ドラッグでパン、ダブルクリックでフィット。
// ライティングを使うモデルでは光源の印を重ね、印をドラッグすると光源が動く (LightMarkers.svelte、T0708)。
// GPU を失って戻るのを待つ間は「GPU の復元を待っています」を、続けて失って諦めたら「作り直す」ボタンを出す
// (docs/ui.md#webpuppet-の寿命。T0938)。
// 拡大・縮小・全体の表示の道具は、領域の右下に重ねる (ツールバーを狭い幅で 2 行に収めるため。T0725)。
// モデルが無いときは、呼び手が渡す空の状態 (`empty`。開く・サンプルのボタン) を真ん中に出す。

import { formatNumber } from "@ugoe/i18n";
import type { Player } from "@ugoe/player";
import { Button, Icon, IconButton, Toolbar } from "@ugoe/ui";
import { onMount, type Snippet } from "svelte";
import { t } from "./i18n";
import LightMarkers from "./LightMarkers.svelte";

let { player, empty }: { player: Player; empty?: Snippet } = $props();

let container: HTMLDivElement;
let dragging: { id: number; x: number; y: number } | null = $state(null);

onMount(() => {
  const observer = new ResizeObserver(() => {
    player.resize(container.clientWidth, container.clientHeight);
  });
  observer.observe(container);
  player.resize(container.clientWidth, container.clientHeight);
  void player.attach(container);
  return () => {
    observer.disconnect();
    player.detach();
  };
});

function local(e: MouseEvent): [number, number] {
  const rect = container.getBoundingClientRect();
  return [e.clientX - rect.left, e.clientY - rect.top];
}

function onWheel(e: WheelEvent) {
  e.preventDefault();
  // トラックパッドの細かいスクロールでも滑らかになるよう、量に比例させる
  const pixels = e.deltaMode === WheelEvent.DOM_DELTA_LINE ? e.deltaY * 16 : e.deltaY;
  player.zoomBy(Math.exp(-pixels * 0.0015), ...local(e));
}

function onPointerDown(e: PointerEvent) {
  if (e.button !== 0 && e.button !== 1) return;
  container.setPointerCapture(e.pointerId);
  dragging = { id: e.pointerId, x: e.clientX, y: e.clientY };
}

function onPointerMove(e: PointerEvent) {
  if (dragging?.id !== e.pointerId) return;
  player.panBy(e.clientX - dragging.x, e.clientY - dragging.y);
  dragging = { id: e.pointerId, x: e.clientX, y: e.clientY };
}

function onPointerUp(e: PointerEvent) {
  if (dragging?.id === e.pointerId) dragging = null;
}
</script>

<!--
  領域の全体は操作 (ドラッグ・ホイール) を受けるだけの要素にし、モデルの絵の名前 (role="img") は中の別の要素に付ける
  (光源の印・空の状態のボタンなど、操作できる部品を role="img" の中に入れない。axe の nested-interactive。T0725)。
-->
<div
  class="viewport"
  class:transparent={player.transparent}
  class:dragging={dragging !== null}
  bind:this={container}
  onwheel={onWheel}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
  ondblclick={() => player.fit()}
  role="presentation"
>
  <div class="picture" role="img" aria-label={player.modelName ?? t("viewport.aria")}></div>
  <!--
    知らせ (空の状態・準備中・失敗・GPU の復元待ち) は、テーマの地の札に載せる (領域の地はモデルの背景の色で、
    テーマに従わないので、テーマの文字の色を直に載せると読めないことがある。T0725)。
    ボタンを押すのを、領域のドラッグ (ポインタの捕捉) に取られないようにする。
  -->
  {#if player.status === "empty"}
    <div class="hint">
      <div class="card" role="presentation" onpointerdown={(e) => e.stopPropagation()}>
        {#if empty}{@render empty()}{:else}<p>{t("viewport.empty")}</p>{/if}
      </div>
    </div>
  {:else if player.status === "starting"}
    <div class="hint"><p class="card">{t("viewport.starting")}</p></div>
  {:else if player.status === "unavailable" && player.canRestart}
    <div class="hint">
      <div class="card" role="presentation" onpointerdown={(e) => e.stopPropagation()}>
        <p class="error ugoe-selectable">{player.error}</p>
        <Button onclick={() => void player.restart()}>{t("viewport.restart")}</Button>
      </div>
    </div>
  {:else if player.status === "unavailable"}
    <div class="hint"><p class="card error ugoe-selectable">{player.error}</p></div>
  {:else if player.waitingForGpu}
    <div class="hint"><p class="card" role="status">{t("viewport.gpuWaiting")}</p></div>
  {/if}
  {#if player.status === "ready" && player.usesLighting && player.lighting.enabled && player.lightMarkers}
    <LightMarkers {player} />
  {/if}
  {#if player.status === "ready"}
    <!-- ダブルクリック (全体の表示) とドラッグを、道具の上では起こさない -->
    <div
      class="view-tools"
      role="presentation"
      onpointerdown={(e) => e.stopPropagation()}
      ondblclick={(e) => e.stopPropagation()}
      onwheel={(e) => e.stopPropagation()}
    >
      <Toolbar label={t("toolbar.view.aria")}>
        <IconButton label={t("toolbar.zoomOut.aria")} onclick={() => player.zoomBy(1 / 1.25)}>
          <Icon name="minus" />
        </IconButton>
        <span class="zoom">{formatNumber(player.zoom, { style: "percent", maximumFractionDigits: 0 })}</span>
        <IconButton label={t("toolbar.zoomIn.aria")} onclick={() => player.zoomBy(1.25)}>
          <Icon name="plus" />
        </IconButton>
        <IconButton label={t("toolbar.fit.aria")} shortcut="F" onclick={() => player.fit()}>
          <Icon name="fit" />
        </IconButton>
      </Toolbar>
    </div>
  {/if}
</div>

<style>
  .viewport {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    cursor: grab;
    touch-action: none;
  }
  .viewport.dragging {
    cursor: grabbing;
  }
  /* 背景を透明にしたときは市松模様を見せる (描画の中身なので、色はトークンの外で決める。docs/ui.md#デザイントークン) */
  .viewport.transparent {
    background-color: #fff;
    background-image:
      linear-gradient(45deg, #ccc 25%, transparent 25%),
      linear-gradient(-45deg, #ccc 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #ccc 75%),
      linear-gradient(-45deg, transparent 75%, #ccc 75%);
    background-size: 16px 16px;
    background-position:
      0 0,
      0 8px,
      8px -8px,
      -8px 0;
  }
  .viewport :global(.viewport-canvas) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
  }
  .hint {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: var(--ugoe-space-6);
    pointer-events: none;
  }
  .card {
    display: grid;
    justify-items: center;
    gap: var(--ugoe-space-3);
    max-width: 32rem;
    margin: 0;
    padding: var(--ugoe-space-4) var(--ugoe-space-5);
    text-align: center;
    color: var(--ugoe-color-text-muted);
    background: var(--ugoe-color-surface);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-lg);
    pointer-events: auto;
    cursor: default;
  }
  .card :global(p) {
    margin: 0;
  }
  .error {
    color: var(--ugoe-color-danger);
  }
  .picture {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .view-tools {
    position: absolute;
    right: var(--ugoe-space-3);
    bottom: var(--ugoe-space-3);
    padding: var(--ugoe-space-1);
    background: var(--ugoe-color-surface);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-md);
    box-shadow: var(--ugoe-shadow-overlay);
    cursor: default;
  }
  .zoom {
    min-width: 5ch;
    text-align: center;
    font-size: var(--ugoe-font-size-sm);
    font-variant-numeric: tabular-nums;
  }
</style>
