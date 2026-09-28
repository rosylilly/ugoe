<script lang="ts">
// フェイストラッキングのパネル (T0703): 入力の選択、映像とランドマーク、調整、記録、値の表示。
import { onDestroy, onMount } from "svelte";
import { formatValue } from "./parameters";
import type { Player } from "./player.svelte";
import type { FaceTracker } from "./tracking/tracker.svelte";

let { player, tracker }: { player: Player; tracker: FaceTracker } = $props();

let canvas = $state<HTMLCanvasElement | null>(null);
let fileInput = $state<HTMLInputElement | null>(null);

$effect(() => {
  tracker.attachCanvas(canvas);
});

onMount(() => {
  void tracker.refreshCameras();
});

onDestroy(() => {
  tracker.stop();
});

function onFile(e: Event & { currentTarget: HTMLInputElement }) {
  const file = e.currentTarget.files?.[0];
  e.currentTarget.value = "";
  if (file) void tracker.startFile(file);
}

const SOURCE_KIND = { camera: "カメラ", video: "動画", image: "画像", replay: "記録の再生" };

let statusText = $derived.by(() => {
  switch (tracker.status) {
    case "loading":
      return "準備中…";
    case "running":
      if (tracker.source?.kind === "replay") {
        return `${tracker.replayTime.toFixed(1)} / ${tracker.replayDuration.toFixed(1)} 秒`;
      }
      return tracker.faceFound ? `${tracker.fps} fps` : "顔が見つかりません";
    default:
      return "";
  }
});

let blendShapes = $derived(Object.entries(tracker.output?.blendShapes ?? {}));
let showMedia = $derived(tracker.running && tracker.source?.kind !== "replay");
</script>

<details class="panel tracking" open>
  <summary>
    <h2>フェイストラッキング</h2>
    <span class="status" class:warn={tracker.running && !tracker.faceFound}>{statusText}</span>
  </summary>

  <div class="row">
    <select
      bind:value={tracker.cameraId}
      disabled={tracker.running}
      title="カメラ"
      onfocus={() => void tracker.refreshCameras()}
    >
      {#each tracker.cameras as camera (camera.id)}
        <option value={camera.id}>{camera.label}</option>
      {:else}
        <option value="">カメラ (既定)</option>
      {/each}
    </select>
    {#if tracker.running || tracker.status === "loading"}
      <button type="button" onclick={() => tracker.stop()}>停止</button>
    {:else}
      <button type="button" onclick={() => void tracker.startCamera()}>開始</button>
    {/if}
    <button
      type="button"
      onclick={() => fileInput?.click()}
      title="動画・画像、または記録した値 (JSON) で確かめる">ファイル…</button
    >
    <input
      bind:this={fileInput}
      class="hidden"
      type="file"
      accept="video/*,image/*,.json,application/json"
      onchange={onFile}
    />
  </div>

  {#if tracker.error}
    <p class="error">{tracker.error}</p>
  {/if}
  {#if tracker.source}
    <p class="source">{SOURCE_KIND[tracker.source.kind]}: {tracker.source.label}</p>
  {/if}

  <div class="preview" class:hidden={!showMedia}>
    <canvas bind:this={canvas} class:mirror={tracker.mirrorView}></canvas>
  </div>

  <div class="options">
    <label title="映像を鏡のように表示する (表示だけで、値は変わらない)">
      <input type="checkbox" bind:checked={tracker.mirrorView} /> 鏡像で表示
    </label>
    <label title="ランドマーク (顔の輪郭・目・眉・唇) を重ねて描く">
      <input type="checkbox" bind:checked={tracker.showLandmarks} /> ランドマーク
    </label>
    <label
      title="左右 (Left / Right) と、ヨー・ロールの符号を入れ替える。モデルを鏡に映った自分のように動かすとき、または入力の映像がすでに左右反転しているとき (反転済みの仮想カメラなど) に使う"
    >
      <input type="checkbox" bind:checked={tracker.swapSides} /> 左右を入れ替える
    </label>
    <label title="トラッキング中はモデルの自動の目パチを止める (目の開閉はトラッキングの値だけになる)">
      <input
        type="checkbox"
        checked={tracker.stopEyeBlink}
        onchange={(e) => tracker.setStopEyeBlink(e.currentTarget.checked)}
      /> 目パチを止める
    </label>
  </div>

  <label class="slider" title="揺れを抑える強さ (One Euro フィルタ)。強いほど滑らかで、動きが遅れる">
    平滑化
    <input type="range" min="0" max="1" step="0.05" bind:value={tracker.smoothing} />
    <span class="value">{tracker.smoothing.toFixed(2)}</span>
  </label>

  <div class="row">
    <button
      type="button"
      onclick={() => tracker.calibrate()}
      disabled={!tracker.running || !tracker.faceFound}
      title="今の顔を中立 (正面・無表情) にする">キャリブレーション</button
    >
    <button
      type="button"
      onclick={() => tracker.clearCalibration()}
      disabled={!tracker.calibrated}
      title="キャリブレーションを取り消す">解除</button
    >
  </div>

  <div class="row">
    {#if tracker.recording}
      <button type="button" class="rec" onclick={() => tracker.stopRecording()}
        >記録を止める ({tracker.recordedCount})</button
      >
    {:else}
      <button
        type="button"
        onclick={() => tracker.startRecording()}
        disabled={!tracker.running || tracker.source?.kind === "replay"}
        title="検出した値を記録する (JSON に保存して、カメラの無い環境で再生できる)">記録</button
      >
      <button
        type="button"
        onclick={() => void tracker.saveRecording()}
        disabled={tracker.recordedCount === 0}>保存 ({tracker.recordedCount})</button
      >
    {/if}
  </div>

  {#if tracker.output}
    {@const head = tracker.output.head}
    <details>
      <summary>頭の向きとブレンドシェイプ <span class="count">{blendShapes.length}</span></summary>
      <dl class="values">
        <dt>yaw</dt>
        <dd>{head.yaw.toFixed(1)}°</dd>
        <dt>pitch</dt>
        <dd>{head.pitch.toFixed(1)}°</dd>
        <dt>roll</dt>
        <dd>{head.roll.toFixed(1)}°</dd>
        {#each blendShapes as [name, score] (name)}
          <dt title={name}>{name}</dt>
          <dd>
            <span class="bar" style:width="{Math.round(score * 100)}%"></span>
            <span class="num">{score.toFixed(2)}</span>
          </dd>
        {/each}
      </dl>
    </details>
  {/if}
  {#if player.trackingParameterIds.length > 0}
    <details>
      <summary>
        変換後のパラメータ <span class="count">{player.trackingParameterIds.length}</span>
      </summary>
      <dl class="values">
        {#each player.trackingParameterIds as id (id)}
          <dt title={id}>{id}</dt>
          <dd><span class="num">{formatValue(player.values[id] ?? 0)}</span></dd>
        {/each}
      </dl>
    </details>
  {/if}
</details>

<style>
  .tracking {
    flex-shrink: 0;
    max-height: 60%;
    overflow-y: auto;
    font-size: 0.8rem;
  }
  .tracking > summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
  }
  h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    display: inline;
  }
  .status {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .status.warn {
    color: var(--warning);
  }
  .row {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    margin-top: 0.4rem;
  }
  select {
    flex: 1;
    min-width: 0;
    font: inherit;
    color: inherit;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.2rem;
  }
  .hidden {
    display: none;
  }
  .error {
    color: var(--danger);
    margin: 0.4rem 0 0;
  }
  .source {
    color: var(--muted);
    margin: 0.4rem 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .preview {
    margin-top: 0.4rem;
    background: #000;
    border-radius: 4px;
    overflow: hidden;
  }
  .preview canvas {
    display: block;
    width: 100%;
    height: auto;
  }
  .preview canvas.mirror {
    transform: scaleX(-1);
  }
  .options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.2rem 0.5rem;
    margin-top: 0.4rem;
  }
  .slider {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.4rem;
  }
  .slider input {
    flex: 1;
  }
  .value,
  .num {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .rec {
    border-color: var(--danger);
    color: var(--danger);
  }
  details details > summary {
    cursor: pointer;
    font-weight: 600;
    margin-top: 0.5rem;
  }
  .count {
    color: var(--muted);
    font-weight: normal;
  }
  .values {
    display: grid;
    grid-template-columns: 9.5rem 1fr;
    gap: 0.1rem 0.4rem;
    margin: 0.3rem 0 0;
  }
  .values dt {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .values dd {
    margin: 0;
    position: relative;
    text-align: right;
  }
  .bar {
    position: absolute;
    left: 0;
    top: 15%;
    height: 70%;
    background: var(--accent);
    opacity: 0.3;
    border-radius: 2px;
  }
  .num {
    position: relative;
  }
</style>
