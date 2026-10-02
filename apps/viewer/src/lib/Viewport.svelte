<script lang="ts">
// モデルを描く領域。ホイールでズーム、ドラッグでパン、ダブルクリックでフィット。
// ライティングを使うモデルでは光源の印を重ね、印をドラッグすると光源が動く (LightMarkers.svelte、T0708)。

import type { Player } from "@ugoe/player";
import { onMount } from "svelte";
import { t } from "./i18n";
import LightMarkers from "./LightMarkers.svelte";

let { player }: { player: Player } = $props();

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
  role="img"
  aria-label={player.modelName ?? t("viewport.aria")}
>
  {#if player.status === "empty"}
    <p class="hint">{t("viewport.empty")}</p>
  {:else if player.status === "starting"}
    <p class="hint">{t("viewport.starting")}</p>
  {:else if player.status === "unavailable"}
    <p class="hint error">{player.error}</p>
  {/if}
  {#if player.status === "ready" && player.usesLighting && player.lighting.enabled && player.lightMarkers}
    <LightMarkers {player} />
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
    margin: 0;
    padding: var(--ugoe-space-6);
    text-align: center;
    color: var(--ugoe-color-text-muted);
    pointer-events: none;
  }
  .hint.error {
    color: var(--ugoe-color-danger);
  }
</style>
