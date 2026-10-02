<!--
  出力ウィンドウ (docs/studio.md#出力ウィンドウ)。モデルだけを、背景を透過して描く。UI の要素を一切重ねない。
  @ugoe/player の受け手のモードで、指揮役のフレームを順に適用して物理を回す (ADR 0028 の 3.)。
  枠が無いので、Alt を押しながらドラッグするとウィンドウを動かせる。
-->
<script lang="ts">
import { decodeFrame, Player, type SceneMessage } from "@ugoe/player";
import { onMount } from "svelte";
import { studioHost } from "./lib/host";
import { loadWasm } from "./lib/wasm/load";

let container: HTMLDivElement;

const player = new Player({
  loadWasm,
  // 着せ替えの組を戻す経路は使わない (モデルは指揮役のシーンから取る)。
  readFile: () => Promise.reject(new Error("the output window reads models from the scene only")),
});
player.startFollowing();
// 開発中やヘッドレスのブラウザで状態を見るため。
window.__UGOE_STUDIO_OUTPUT__ = player;

/** 最後に届いたシーン (読んでいる間に次のシーンが来たら、古い方の読み終わりは捨てる)。 */
let latest = 0;

async function loadScene(scene: SceneMessage) {
  latest = scene.revision;
  const host = await studioHost();
  let bytes: Uint8Array;
  try {
    bytes = await host.sceneModel(scene.revision);
  } catch {
    // もう次のシーンになっている。次のシーンの通知を待つ。
    return;
  }
  if (scene.revision !== latest) return;
  await player.load({ name: scene.modelName, bytes });
  if (scene.revision !== latest) return;
  player.fit();
  player.followScene(scene.revision);
}

function onPointerDown(event: PointerEvent) {
  if (!event.altKey || event.button !== 0) return;
  event.preventDefault();
  void studioHost().then((host) => host.startDragging());
}

onMount(() => {
  const observer = new ResizeObserver(() => {
    player.resize(container.clientWidth, container.clientHeight);
    if (player.status === "ready") player.fit();
  });
  observer.observe(container);
  player.resize(container.clientWidth, container.clientHeight);
  void (async () => {
    await player.attach(container);
    player.setBackground("#000000", true);
    const host = await studioHost();
    await host.subscribeFrames((message) => {
      if (message instanceof ArrayBuffer) player.pushFrame(decodeFrame(message));
      else if (message.kind === "scene") void loadScene(message);
    });
  })();
  return () => {
    observer.disconnect();
    player.detach();
  };
});
</script>

<div class="output" bind:this={container} onpointerdown={onPointerDown} role="presentation"></div>

<style>
  .output {
    position: fixed;
    inset: 0;
  }

  .output :global(canvas) {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
