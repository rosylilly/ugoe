<script lang="ts">
// フェイストラッキングのパネル (T0703、ビューアとエディタで共有。T0636): 入力の選択、映像とランドマーク、
// 調整、記録、値の表示。文言は @ugoe/i18n の名前空間 `face-tracking` (i18n.ts)。
import { formatNumber } from "@ugoe/i18n";
import { onDestroy, onMount } from "svelte";
import { t } from "./i18n";
import type { FaceTracker } from "./tracker.svelte";

interface Props {
  tracker: FaceTracker;
  /** トラッキングが書き込むパラメータ (`WebPuppet.tracking_parameter_ids`)。 */
  parameterIds: readonly string[];
  /** パラメータの今の値 (ID → 値)。 */
  values: Readonly<Record<string, number>>;
  /**
   * 開いているか (最初の状態)。`bind:open` にしているのは、単方向の属性だと値の表示が変わるたびに
   * 属性が書き戻され、利用者が開閉した状態が戻ってしまうため。
   */
  open?: boolean;
}

let { tracker, parameterIds, values, open = $bindable(true) }: Props = $props();

/** 入力の種類の名前のキー。 */
const SOURCE_KIND = {
  camera: "source.camera",
  video: "source.video",
  image: "source.image",
  replay: "source.replay",
} as const;

/** 記録の再生の位置 (秒。小数第 1 位まで)。 */
function seconds(value: number): string {
  return formatNumber(value, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
    useGrouping: false,
  });
}

/** 値の表示 (小数第 3 位まで)。 */
function formatValue(value: number): string {
  const rounded = Math.round(value * 1000) / 1000;
  return (Object.is(rounded, -0) ? 0 : rounded).toString();
}

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

let statusText = $derived.by(() => {
  switch (tracker.status) {
    case "loading":
      return t("status.loading");
    case "running": {
      if (tracker.source?.kind === "replay") {
        return t("status.replayPosition", {
          time: seconds(tracker.replayTime),
          duration: seconds(tracker.replayDuration),
        });
      }
      if (!tracker.faceFound) return t("status.noFace");
      const fps = t("status.fps", { fps: tracker.fps });
      if (tracker.trackBody && !tracker.bodyFound) return `${fps} · ${t("status.noBody")}`;
      return fps;
    }
    default:
      return "";
  }
});

let blendShapes = $derived(Object.entries(tracker.output?.blendShapes ?? {}));
/** 体の値の表示 (無い値は「—」)。 */
let bodyRows = $derived.by(() => {
  const body = tracker.output?.body;
  if (!body) return [];
  const deg = (v: number | null) => (v === null ? "—" : `${v.toFixed(1)}°`);
  const num = (v: number | null) => (v === null ? "—" : v.toFixed(2));
  return [
    ["yaw", deg(body.yaw)],
    ["pitch", deg(body.pitch)],
    ["roll", deg(body.roll)],
    ["shoulderLeft", num(body.shoulderLeft)],
    ["shoulderRight", num(body.shoulderRight)],
  ] as const;
});
let showMedia = $derived(tracker.running && tracker.source?.kind !== "replay");
</script>

<details class="tracking" bind:open>
  <summary>
    <h2>{t("title")}</h2>
    <span class="status" class:warn={tracker.running && !tracker.faceFound}>{statusText}</span>
  </summary>

  <div class="row">
    <select
      bind:value={tracker.cameraId}
      disabled={tracker.running}
      title={t("camera.select.tooltip")}
      onfocus={() => void tracker.refreshCameras()}
    >
      {#each tracker.cameras as camera (camera.id)}
        <option value={camera.id}>{camera.label}</option>
      {:else}
        <option value="">{t("camera.default")}</option>
      {/each}
    </select>
    {#if tracker.running || tracker.status === "loading"}
      <button type="button" onclick={() => tracker.stop()}>{t("camera.stop.label")}</button>
    {:else}
      <button type="button" onclick={() => void tracker.startCamera()}>{t("camera.start.label")}</button>
    {/if}
    <button
      type="button"
      onclick={() => fileInput?.click()}
      title={t("file.tooltip")}>{t("file.label")}</button
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
    <p class="source">{t(SOURCE_KIND[tracker.source.kind])}: {tracker.source.label}</p>
  {/if}

  <div class="preview" class:hidden={!showMedia}>
    <canvas bind:this={canvas} class:mirror={tracker.mirrorView}></canvas>
  </div>

  <div class="options">
    <label title={t("options.mirrorView.tooltip")}>
      <input type="checkbox" bind:checked={tracker.mirrorView} /> {t("options.mirrorView.label")}
    </label>
    <label title={t("options.landmarks.tooltip")}>
      <input type="checkbox" bind:checked={tracker.showLandmarks} /> {t("options.landmarks.label")}
    </label>
    <label title={t("options.swapSides.tooltip")}>
      <input type="checkbox" bind:checked={tracker.swapSides} /> {t("options.swapSides.label")}
    </label>
    <label title={t("options.stopEyeBlink.tooltip")}>
      <input
        type="checkbox"
        checked={tracker.stopEyeBlink}
        onchange={(e) => tracker.setStopEyeBlink(e.currentTarget.checked)}
      /> {t("options.stopEyeBlink.label")}
    </label>
    <label title={t("options.trackBody.tooltip")}>
      <input
        type="checkbox"
        checked={tracker.trackBody}
        onchange={(e) => tracker.setTrackBody(e.currentTarget.checked)}
      /> {t("options.trackBody.label")}
    </label>
  </div>
  {#if tracker.trackBody && tracker.bodyError}
    <p class="error">{tracker.bodyError}</p>
  {/if}

  <label class="slider" title={t("smoothing.tooltip")}>
    {t("smoothing.label")}
    <input type="range" min="0" max="1" step="0.05" bind:value={tracker.smoothing} />
    <span class="value">{tracker.smoothing.toFixed(2)}</span>
  </label>

  <div class="row">
    <button
      type="button"
      onclick={() => tracker.calibrate()}
      disabled={!tracker.running || !tracker.faceFound}
      title={t("calibration.calibrate.tooltip")}>{t("calibration.calibrate.label")}</button
    >
    <button
      type="button"
      onclick={() => tracker.clearCalibration()}
      disabled={!tracker.calibrated}
      title={t("calibration.clear.tooltip")}>{t("calibration.clear.label")}</button
    >
  </div>

  <div class="row">
    {#if tracker.recording}
      <button type="button" class="rec" onclick={() => tracker.stopRecording()}
        >{t("recording.stop.label", { frames: tracker.recordedCount })}</button
      >
    {:else}
      <button
        type="button"
        onclick={() => tracker.startRecording()}
        disabled={!tracker.running || tracker.source?.kind === "replay"}
        title={t("recording.record.tooltip")}>{t("recording.record.label")}</button
      >
      <button
        type="button"
        onclick={() => void tracker.saveRecording()}
        disabled={tracker.recordedCount === 0}>{t("recording.save.label", { frames: tracker.recordedCount })}</button
      >
    {/if}
  </div>

  {#if tracker.output}
    {@const head = tracker.output.head}
    <details>
      <summary>{t("values.headAndBlendShapes")} <span class="count">{blendShapes.length}</span></summary>
      <dl class="values">
        <dt>yaw</dt>
        <dd>{head.yaw.toFixed(1)}°</dd>
        <dt>pitch</dt>
        <dd>{head.pitch.toFixed(1)}°</dd>
        <dt>roll</dt>
        <dd>{head.roll.toFixed(1)}°</dd>
        {#if bodyRows.length > 0}
          <dt class="group" title={t("values.body.tooltip")}>{t("values.body.label")}</dt>
          <dd></dd>
          {#each bodyRows as [name, text] (name)}
            <dt title={name}>{name}</dt>
            <dd><span class="num">{text}</span></dd>
          {/each}
        {/if}
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
  {#if parameterIds.length > 0}
    <details>
      <summary>{t("values.parameters")} <span class="count">{parameterIds.length}</span></summary>
      <dl class="values">
        {#each parameterIds as id (id)}
          <dt title={id}>{id}</dt>
          <dd><span class="num">{formatValue(values[id] ?? 0)}</span></dd>
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
    color: var(--warning, #b7791f);
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
    color: var(--danger, #c53030);
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
    border-color: var(--danger, #c53030);
    color: var(--danger, #c53030);
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
  .values dt.group {
    font-weight: 600;
    margin-top: 0.2rem;
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
