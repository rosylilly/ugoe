<!--
  入力 (docs/studio.md#操作ウィンドウ のパネル、T0803): カメラのフェイストラッキング (@ugoe/face-tracking のパネル) と、
  マイクの口パク (機器・感度・ノイズの閾値・音量のメーター)。どちらもこのウィンドウの WebView で取り、値だけを指揮役に送る。
  選んだ機器と調整はプロファイルに持つ (`onsettings`)。
-->
<script lang="ts">
import { type FaceTracker, TrackingPanel } from "@ugoe/face-tracking";
import { formatNumber } from "@ugoe/i18n";
import { Button, Notice, Select, Slider, Toggle } from "@ugoe/ui";
import { onMount } from "svelte";
import { t } from "./lib/i18n";
import type { InputSettings } from "./lib/inputs";
import type { Microphone } from "./lib/microphone.svelte";

interface Props {
  tracker: FaceTracker;
  microphone: Microphone;
  settings: InputSettings;
  /** トラッキングが書き込むパラメータ。 */
  trackingParameters: readonly string[];
  /** パラメータの今の値 (ID → 値。指揮役から間引いて届く)。 */
  values: Readonly<Record<string, number>>;
  trackingPaused: boolean;
  lipSyncPaused: boolean;
  disabled?: boolean;
  onsettings: (settings: InputSettings) => void;
  ontrackingpause: (paused: boolean) => void;
  onlipsyncpause: (paused: boolean) => void;
}

let {
  tracker,
  microphone,
  settings,
  trackingParameters,
  values,
  trackingPaused,
  lipSyncPaused,
  disabled = false,
  onsettings,
  ontrackingpause,
  onlipsyncpause,
}: Props = $props();

/** 既定の機器の値 (Select の値は文字列なので、空の ID とぶつからない値)。 */
const DEFAULT_DEVICE = "\u0000default";

const microphoneOptions = $derived([
  { value: DEFAULT_DEVICE, label: t("microphone.defaultDevice") },
  ...microphone.devices.map((d) => ({ value: d.id, label: d.label })),
]);

function change(patch: Partial<InputSettings>) {
  const current = $state.snapshot(settings) as InputSettings;
  const next = { ...current, ...patch };
  if (JSON.stringify(next) !== JSON.stringify(current)) onsettings(next);
}

// トラッキングのパネルで変えた調整と、使ったカメラを覚える。
$effect(() => {
  const patch: Partial<InputSettings> = {
    smoothing: tracker.smoothing,
    swapSides: tracker.swapSides,
    trackBody: tracker.trackBody,
    stopEyeBlink: tracker.stopEyeBlink,
  };
  if (tracker.running && tracker.source?.kind === "camera" && tracker.cameraId !== "") {
    patch.camera = { id: tracker.cameraId, label: tracker.source.label };
  }
  change(patch);
});

function chooseMicrophone(value: string) {
  microphone.deviceId = value === DEFAULT_DEVICE ? "" : value;
  change({
    microphone:
      value === DEFAULT_DEVICE ? null : { id: value, label: microphone.deviceLabel || value },
  });
  if (microphone.running) void microphone.start();
}

async function toggleMicrophone() {
  if (microphone.running) {
    microphone.stop();
    return;
  }
  await microphone.start();
  if (microphone.running && microphone.deviceId !== "") {
    change({ microphone: { id: microphone.deviceId, label: microphone.deviceLabel } });
  }
}

onMount(() => {
  void microphone.refreshDevices().then(() => microphone.choose(settings.microphone));
});

/** メーターの表示 (百分率)。 */
function percent(value: number): string {
  return formatNumber(value, { style: "percent", maximumFractionDigits: 0 });
}
</script>

<div class="inputs">
  <section class="inputs__group" aria-labelledby="inputs-camera">
    <div class="inputs__heading">
      <h4 id="inputs-camera">{t("inputs.camera")}</h4>
      <Button
        small
        pressed={trackingPaused}
        {disabled}
        onclick={() => ontrackingpause(!trackingPaused)}
        >{trackingPaused ? t("inputs.resumeTracking") : t("inputs.pauseTracking")}</Button
      >
    </div>
    {#if trackingPaused}
      <Notice tone="warning">{t("inputs.trackingPaused")}</Notice>
    {/if}
    <TrackingPanel {tracker} parameterIds={trackingParameters} {values} />
  </section>

  <section class="inputs__group" aria-labelledby="inputs-microphone">
    <div class="inputs__heading">
      <h4 id="inputs-microphone">{t("inputs.microphone")}</h4>
      <Button
        small
        pressed={lipSyncPaused}
        {disabled}
        onclick={() => onlipsyncpause(!lipSyncPaused)}
        >{lipSyncPaused ? t("inputs.resumeLipSync") : t("inputs.pauseLipSync")}</Button
      >
    </div>
    <Select
      label={t("microphone.device")}
      value={microphone.deviceId === "" ? DEFAULT_DEVICE : microphone.deviceId}
      options={microphoneOptions}
      {disabled}
      onchange={chooseMicrophone}
    />
    <div class="inputs__row">
      <Button
        variant={microphone.running ? "secondary" : "primary"}
        disabled={disabled || microphone.status === "loading"}
        onclick={toggleMicrophone}
        >{microphone.running ? t("microphone.stop") : t("microphone.start")}</Button
      >
      <span class="inputs__hint" aria-live="polite">
        {microphone.status === "loading"
          ? t("microphone.starting")
          : microphone.running
            ? lipSyncPaused
              ? t("inputs.lipSyncPaused")
              : t("microphone.running")
            : t("microphone.stopped")}
      </span>
    </div>
    {#if microphone.error !== null}
      <Notice tone="danger">{microphone.error}</Notice>
    {/if}
    <div class="inputs__meter">
      <label for="inputs-loudness">{t("microphone.loudness")}</label>
      <meter
        id="inputs-loudness"
        min="0"
        max="1"
        low={settings.noiseGate}
        value={microphone.loudness}>{percent(microphone.loudness)}</meter
      >
      <label for="inputs-mouth">{t("microphone.mouth")}</label>
      <meter id="inputs-mouth" min="0" max="1" value={microphone.level}>{percent(microphone.level)}</meter>
    </div>
    <Slider
      label={t("microphone.sensitivity")}
      value={settings.sensitivity}
      min={0}
      max={1}
      step={0.01}
      defaultValue={0.5}
      {disabled}
      oncommit={(v) => change({ sensitivity: v })}
    />
    <Slider
      label={t("microphone.noiseGate")}
      value={settings.noiseGate}
      min={0}
      max={1}
      step={0.01}
      defaultValue={0.3}
      {disabled}
      oncommit={(v) => change({ noiseGate: v })}
    />
    <p class="inputs__hint">{t("microphone.hint")}</p>
  </section>
</div>

<style>
  .inputs {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-5);
  }

  .inputs__group {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .inputs__heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--ugoe-space-2);
  }

  .inputs__heading h4 {
    margin: 0;
    font-size: var(--ugoe-font-size-md);
    font-weight: var(--ugoe-font-weight-strong);
  }

  .inputs__row {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-3);
  }

  .inputs__meter {
    display: grid;
    grid-template-columns: max-content 1fr;
    align-items: center;
    gap: var(--ugoe-space-2) var(--ugoe-space-3);
    font-size: var(--ugoe-font-size-sm);
    color: var(--ugoe-color-text-muted);
  }

  .inputs__meter meter {
    width: 100%;
  }

  .inputs__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
