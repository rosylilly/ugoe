<!--
  出力ウィンドウ (docs/studio.md#出力ウィンドウ) とブラウザソースのページ (docs/studio.md#ブラウザソース) の受け手。
  モデルだけを描く。UI の要素を一切重ねない。どちらも同じこの部品で描き、配信の受け取り方 (`feed`) だけが違う
  (出力ウィンドウは Tauri の Channel、ブラウザソースは WebSocket。lib/feed.ts)。
  @ugoe/player の受け手のモードで、指揮役のフレームを順に適用して物理を回す (ADR 0028 の 3.)。
  シーンの通知を受けたら、版の番号を比べてモデルの読み直し・モジュールの付け直し・見せ方の変更を選ぶ
  (lib/outputLayout.ts の `planScene`、docs/studio.md#シーンの切り替えと受け手)。
  背景はシーンに従って要素の CSS で描き (透過・単色・クロマキー・画像)、モデルの canvas は常に透過にする。
  回転は描画の要素 (舞台) の CSS で回す (ビューに回転が無いため)。
  出力ウィンドウは枠が無いので、左ボタンでドラッグするとウィンドウを動かせる (`feed.startDragging` があるとき。T0813)。
  描画の負荷は約 1 秒ごとにバックエンドへ送り、操作ウィンドウに出す (ここには何も重ねない。docs/studio.md#負荷の表示)。
-->
<script lang="ts">
import { decodeFrame, Player } from "@ugoe/player";
import { onMount } from "svelte";
import type { OutputFeed } from "./lib/feed";
import type { StudioScene } from "./lib/host";
import {
  backgroundCss,
  type FollowedScene,
  planScene,
  stageLayout,
  zoomFactor,
} from "./lib/outputLayout";
import { sceneLighting } from "./lib/profile";
import { RenderStatsWindow } from "./lib/renderStats";
import { loadWasm } from "./lib/wasm/load";

interface Props {
  /** 配信を受け取る口 (出力ウィンドウは Tauri の StudioHost、ブラウザソースは lib/feed.ts の `sourceFeed`)。 */
  feed: () => Promise<OutputFeed>;
}

let { feed }: Props = $props();

let output: HTMLDivElement;
let stage: HTMLDivElement;

const player = new Player({
  loadWasm,
  // 着せ替えの組を戻す経路は使わない (モデルとモジュールは指揮役のシーンから取る)。
  readFile: () => Promise.reject(new Error("the output window reads models from the scene only")),
});
const renderStats = new RenderStatsWindow();
player.startFollowing(({ at, renderMs, dropped }) => {
  const stats = renderStats.record(at, renderMs, dropped);
  if (stats !== null) void feed().then((f) => f.reportRenderStats(stats));
});
// 開発中やヘッドレスのブラウザで状態を見るため。
window.__UGOE_STUDIO_OUTPUT__ = player;

/** 最後に届いたシーンの `revision` (読んでいる間に次のシーンが来たら、古い方の続きは捨てる)。 */
let latest = 0;
/** 見せ方を当てたシーン。 */
let shown = $state<StudioScene | null>(null);
/** 受け手が今持っているモデルとモジュールの版。 */
let followed: FollowedScene | null = null;
/** 背景の画像 (パスと blob URL)。 */
let image = $state<{ path: string; url: string } | null>(null);
/** シーンの通知を 1 つずつ順に処理する。 */
let queue: Promise<void> = Promise.resolve();

const visible = $derived(shown !== null && shown.modelRevision !== 0);
const background = $derived(
  shown === null
    ? "transparent"
    : backgroundCss(
        shown.background,
        shown.background.kind === "image" && image?.path === shown.background.path
          ? image.url
          : null,
      ),
);

function onScene(scene: StudioScene) {
  latest = scene.revision;
  queue = queue
    .then(() => applyScene(scene))
    .catch((e: unknown) => {
      player.error = e instanceof Error ? e.message : String(e);
    });
}

async function applyScene(scene: StudioScene): Promise<void> {
  if (scene.revision !== latest) return;
  const host = await feed();
  const plan = planScene(followed, scene);
  if (plan === "empty") {
    followed = { modelRevision: 0, modulesRevision: scene.modulesRevision };
    await showLook(scene);
    return;
  }
  const modules =
    plan === "reload" || plan === "modules"
      ? await Promise.all(
          scene.modules.map(async (name, i) => ({
            name: `${name}.ugoemod`,
            bytes: await host.sceneModule(scene.revision, i),
          })),
        ).catch(() => null)
      : [];
  // 取れなければ、もう次のシーンになっている。次のシーンの通知を待つ。
  if (modules === null || scene.revision !== latest) return;
  if (plan === "reload") {
    let bytes: Uint8Array;
    try {
      bytes = await host.sceneModel(scene.revision);
    } catch {
      return;
    }
    if (scene.revision !== latest) return;
    for (const m of player.modules) player.unloadModule(m.name);
    await player.load(
      { name: scene.modelName, bytes },
      modules.map((file) => ({ file, attach: true })),
    );
    // 読んだモデルには前のシーンのフレームを当てない (値の数が違う)。見せ方を待たずに、このシーンのフレームへ移る。
    followed = { modelRevision: scene.modelRevision, modulesRevision: scene.modulesRevision };
    player.followScene(scene.revision);
    await showLook(scene);
    return;
  }
  if (plan === "modules") {
    for (const m of player.modules) player.unloadModule(m.name);
    for (const file of modules) player.loadModule(file, true);
    followed = { modelRevision: scene.modelRevision, modulesRevision: scene.modulesRevision };
  }
  await showLook(scene);
  player.continueScene(scene.revision);
}

/** 見せ方 (ライティング・配置・背景) を当てる。 */
async function showLook(scene: StudioScene): Promise<void> {
  if (scene.background.kind === "image" && image?.path !== scene.background.path) {
    const path = scene.background.path;
    try {
      const bytes = await (await feed()).sceneBackground(scene.revision);
      if (image !== null) URL.revokeObjectURL(image.url);
      image = { path, url: URL.createObjectURL(new Blob([bytes as BlobPart])) };
    } catch {
      // もう次のシーンになっている (次の通知で取り直す)。
    }
  }
  if (scene.revision !== latest) return;
  const lighting = sceneLighting(scene);
  player.updateLighting((s) => Object.assign(s, lighting), { preview: true });
  shown = scene;
  layout();
}

/** 舞台の大きさと位置・回転を出力の大きさとシーンの配置に合わせ、ビューを合わせ直す。 */
function layout() {
  if (output === undefined) return;
  const width = Math.max(1, output.clientWidth);
  const height = Math.max(1, output.clientHeight);
  const transform = shown?.transform ?? { x: 0, y: 0, scale: 1, rotation: 0 };
  const l = stageLayout(width, height, transform);
  stage.style.width = `${l.width}px`;
  stage.style.height = `${l.height}px`;
  stage.style.left = `${l.left}px`;
  stage.style.top = `${l.top}px`;
  stage.style.transformOrigin = `calc(50% + ${l.offsetX}px) calc(50% + ${l.offsetY}px)`;
  stage.style.transform = l.rotation === 0 ? "" : `rotate(${l.rotation}deg)`;
  player.resize(l.width, l.height);
  if (player.status !== "ready") return;
  player.fit();
  const canvas = player.canvasSize;
  if (canvas !== null) {
    player.zoomBy(zoomFactor([width, height], [l.width, l.height], canvas, transform.scale));
  }
  player.panBy(l.offsetX, l.offsetY);
}

/** 解決済みの配信の口。押した瞬間に `startDragging` を呼ぶために、先に受け取っておく。 */
let resolvedFeed: OutputFeed | null = null;

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0) return;
  const start = resolvedFeed?.startDragging;
  if (start === undefined) return;
  event.preventDefault();
  void start.call(resolvedFeed);
}

onMount(() => {
  const observer = new ResizeObserver(layout);
  observer.observe(output);
  layout();
  void (async () => {
    await player.attach(stage);
    player.setBackground("#000000", true);
    const host = await feed();
    resolvedFeed = host;
    await host.subscribeFrames((message) => {
      if (message instanceof ArrayBuffer) player.pushFrame(decodeFrame(message));
      else if (message.kind === "scene") onScene(message);
    });
  })();
  return () => {
    observer.disconnect();
    player.detach();
    if (image !== null) URL.revokeObjectURL(image.url);
  };
});
</script>

<div
  class="output"
  bind:this={output}
  style:background
  onpointerdown={onPointerDown}
  role="presentation"
  data-background={shown?.background.kind ?? "transparent"}
>
  <div class="stage" class:stage--hidden={!visible} bind:this={stage}></div>
</div>

<style>
  .output {
    position: fixed;
    inset: 0;
    overflow: hidden;
  }

  .stage {
    position: absolute;
  }

  .stage--hidden {
    visibility: hidden;
  }

  .stage :global(canvas) {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
