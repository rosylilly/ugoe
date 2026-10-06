<script lang="ts">
// フェイストラッキングのパネル (T0703、ビューアとエディタで共有。T0636): 入力の選択、映像とランドマーク、
// 調整、記録、値の表示。文言は @ugoe/i18n の名前空間 `face-tracking` (i18n.ts)。部品は @ugoe/ui (T0907)。
import { formatNumber } from "@ugoe/i18n";
import { Button, type ButtonBaseProps, Section, Select, Slider, Toggle, Tooltip } from "@ugoe/ui";
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
let cameraOptions = $derived(
  tracker.cameras.length > 0
    ? tracker.cameras.map((c) => ({ value: c.id, label: c.label }))
    : [{ value: "", label: t("camera.default") }],
);
</script>

<div class="tracking">
  <Section title={t("title")} level={2} bind:open>
    {#snippet actions()}
      <span class="status" class:warn={tracker.running && !tracker.faceFound}>{statusText}</span>
    {/snippet}

    <div class="row">
      <!-- 一覧を開く前に機器を数え直す (つないだばかりのカメラを出す) -->
      <div class="camera" onfocusin={() => void tracker.refreshCameras()}>
        <Select
          aria-label={t("camera.select.tooltip")}
          value={tracker.cameraId}
          options={cameraOptions}
          disabled={tracker.running}
          onchange={(id) => (tracker.cameraId = id)}
        />
      </div>
      {#if tracker.running || tracker.status === "loading"}
        <Button small onclick={() => tracker.stop()}>{t("camera.stop.label")}</Button>
      {:else}
        <Button small onclick={() => void tracker.startCamera()}>{t("camera.start.label")}</Button>
      {/if}
      {@render tip(t("file.label"), t("file.tooltip"), () => fileInput?.click())}
      <input
        bind:this={fileInput}
        class="hidden"
        type="file"
        accept="video/*,image/*,.json,application/json"
        onchange={onFile}
      />
    </div>

    {#if tracker.error}
      <p class="error ugoe-selectable">{tracker.error}</p>
    {/if}
    {#if tracker.source}
      <p class="source">{t(SOURCE_KIND[tracker.source.kind])}: {tracker.source.label}</p>
    {/if}

    <div class="preview" class:hidden={!showMedia}>
      <canvas bind:this={canvas} class:mirror={tracker.mirrorView}></canvas>
    </div>

    <!-- すぐに効く入・切なので Toggle (docs/ui.md の使い分け) -->
    <div class="options">
      <Toggle
        label={t("options.mirrorView.label")}
        tooltip={t("options.mirrorView.tooltip")}
        bind:checked={tracker.mirrorView}
      />
      <Toggle
        label={t("options.landmarks.label")}
        tooltip={t("options.landmarks.tooltip")}
        bind:checked={tracker.showLandmarks}
      />
      <Toggle
        label={t("options.swapSides.label")}
        tooltip={t("options.swapSides.tooltip")}
        bind:checked={tracker.swapSides}
      />
      <Toggle
        label={t("options.stopEyeBlink.label")}
        tooltip={t("options.stopEyeBlink.tooltip")}
        checked={tracker.stopEyeBlink}
        onchange={(on) => tracker.setStopEyeBlink(on)}
      />
      <Toggle
        label={t("options.trackBody.label")}
        tooltip={t("options.trackBody.tooltip")}
        checked={tracker.trackBody}
        onchange={(on) => tracker.setTrackBody(on)}
      />
    </div>
    {#if tracker.trackBody && tracker.bodyError}
      <p class="error ugoe-selectable">{tracker.bodyError}</p>
    {/if}

    <div class="smoothing">
      <Slider
        label={t("smoothing.label")}
        min={0}
        max={1}
        step={0.05}
        decimals={2}
        value={tracker.smoothing}
        tooltip={t("smoothing.tooltip")}
        onchange={(v) => (tracker.smoothing = v)}
      />
    </div>

    <div class="row">
      {@render tip(
        t("calibration.calibrate.label"),
        t("calibration.calibrate.tooltip"),
        () => tracker.calibrate(),
        !tracker.running || !tracker.faceFound,
      )}
      {@render tip(
        t("calibration.clear.label"),
        t("calibration.clear.tooltip"),
        () => tracker.clearCalibration(),
        !tracker.calibrated,
      )}
    </div>

    <div class="row">
      {#if tracker.recording}
        <Button small variant="danger" onclick={() => tracker.stopRecording()}
          >{t("recording.stop.label", { frames: tracker.recordedCount })}</Button
        >
      {:else}
        {@render tip(
          t("recording.record.label"),
          t("recording.record.tooltip"),
          () => tracker.startRecording(),
          !tracker.running || tracker.source?.kind === "replay",
        )}
        <Button small onclick={() => void tracker.saveRecording()} disabled={tracker.recordedCount === 0}
          >{t("recording.save.label", { frames: tracker.recordedCount })}</Button
        >
      {/if}
    </div>

    {#if tracker.output}
      {@const head = tracker.output.head}
      <Section title={t("values.headAndBlendShapes")} level={3} open={false}>
        {#snippet actions()}<span class="count">{blendShapes.length}</span>{/snippet}
        <dl class="values">
          <dt>yaw</dt>
          <dd>{head.yaw.toFixed(1)}°</dd>
          <dt>pitch</dt>
          <dd>{head.pitch.toFixed(1)}°</dd>
          <dt>roll</dt>
          <dd>{head.roll.toFixed(1)}°</dd>
          {#if bodyRows.length > 0}
            <dt class="group">{t("values.body.label")}</dt>
            <dd class="note">{t("values.body.tooltip")}</dd>
            {#each bodyRows as [name, text] (name)}
              <dt>{name}</dt>
              <dd><span class="num">{text}</span></dd>
            {/each}
          {/if}
          {#each blendShapes as [name, score] (name)}
            <dt>{name}</dt>
            <dd>
              <span class="bar" style:width="{Math.round(score * 100)}%"></span>
              <span class="num">{score.toFixed(2)}</span>
            </dd>
          {/each}
        </dl>
      </Section>
    {/if}
    {#if parameterIds.length > 0}
      <Section title={t("values.parameters")} level={3} open={false}>
        {#snippet actions()}<span class="count">{parameterIds.length}</span>{/snippet}
        <dl class="values">
          {#each parameterIds as id (id)}
            <dt>{id}</dt>
            <dd><span class="num">{formatValue(values[id] ?? 0)}</span></dd>
          {/each}
        </dl>
      </Section>
    {/if}
  </Section>
</div>

<!-- 説明付きの小さなボタン (`title` 属性の代わりに Tooltip。無効のときは説明が出ない) -->
{#snippet tip(label: string, tooltip: string, onclick: () => void, disabled = false)}
  <Tooltip text={tooltip} {disabled}>
    {#snippet trigger(attrs)}
      <Button {...attrs as ButtonBaseProps} small {disabled} onclick={(e: MouseEvent) => {
          (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
          onclick();
        }}>{label}</Button
      >
    {/snippet}
  </Tooltip>
{/snippet}

<style>
  .status {
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
    font-variant-numeric: tabular-nums;
  }
  .status.warn {
    color: var(--ugoe-color-warning);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-2);
    align-items: center;
    margin-top: var(--ugoe-space-2);
  }
  .camera {
    flex: 1 1 10rem;
    min-width: 0;
  }
  .camera :global(.ugoe-select),
  .camera :global(.ugoe-select__trigger) {
    width: 100%;
  }
  .hidden {
    display: none;
  }
  .error {
    color: var(--ugoe-color-danger);
    margin: var(--ugoe-space-2) 0 0;
  }
  .source {
    color: var(--ugoe-color-text-muted);
    margin: var(--ugoe-space-2) 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .preview {
    margin-top: var(--ugoe-space-2);
    background: #000;
    border-radius: var(--ugoe-radius-md);
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
    grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
    gap: 0 var(--ugoe-space-3);
    margin-top: var(--ugoe-space-2);
  }
  .smoothing {
    margin-top: var(--ugoe-space-2);
  }
  .note {
    margin: var(--ugoe-space-1) 0 0;
    font-size: var(--ugoe-font-size-sm);
    color: var(--ugoe-color-text-muted);
  }
  .num,
  .count {
    font-variant-numeric: tabular-nums;
    color: var(--ugoe-color-text-muted);
  }
  .values {
    display: grid;
    grid-template-columns: minmax(0, 9.5rem) 1fr;
    gap: var(--ugoe-space-1) var(--ugoe-space-2);
    margin: var(--ugoe-space-1) 0 0;
    font-size: var(--ugoe-font-size-sm);
  }
  .values dt.group {
    font-weight: var(--ugoe-font-weight-strong);
    margin-top: var(--ugoe-space-1);
  }
  .values dd.note {
    margin-top: var(--ugoe-space-1);
    text-align: left;
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
    background: var(--ugoe-color-accent);
    opacity: 0.3;
    border-radius: var(--ugoe-radius-sm);
  }
  .num {
    position: relative;
  }
</style>
