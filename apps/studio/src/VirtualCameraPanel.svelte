<!--
  仮想カメラ (docs/studio.md#仮想カメラ、T0807・T0809)。開始と停止・解像度・フレームレート・透過を埋める色を選び、
  止まった (始められない・途中で失敗した)・遅れた・受け取るアプリがいないことを知らせる。始められない理由に合わせて、
  v4l2loopback の入れ方 (Linux)・OBS の仮想カメラを使う手順 (Windows 10)・入れ直し (登録が無い) を案内する。
-->
<script lang="ts">
import { formatNumber } from "@ugoe/i18n";
import {
  ColorPicker,
  hexToRgb,
  Notice,
  rgbToHex,
  SegmentedControl,
  Select,
  Toggle,
} from "@ugoe/ui";
import type { StudioHost } from "./lib/host";
import { t } from "./lib/i18n";
import {
  CAMERA_FRAME_RATES,
  CAMERA_RESOLUTIONS,
  type CameraGuide,
  type CameraNotice,
  cameraNotices,
  isCameraActive,
  loadCameraSettings,
  saveCameraSettings,
  type VirtualCameraSettings,
  type VirtualCameraStatus,
} from "./lib/virtualCamera";

interface Props {
  host: StudioHost | null;
  /** バックエンドからの最新の状態 (取れるまでは `null`)。 */
  status: VirtualCameraStatus | null;
}

let { host, status }: Props = $props();

let settings = $state<VirtualCameraSettings>(loadCameraSettings());
let error = $state<string | null>(null);

const active = $derived(status !== null && isCameraActive(status));
const notices = $derived(status === null ? [] : cameraNotices(status));
const resolutionOptions = $derived(
  CAMERA_RESOLUTIONS.map(([width, height]) => ({
    value: `${width}x${height}`,
    label: t("virtualCamera.resolutionValue", { width, height }),
  })),
);
const rateOptions = $derived(
  CAMERA_FRAME_RATES.map((fps) => ({
    value: String(fps),
    label: t("virtualCamera.fpsValue", { value: fps }),
  })),
);
const stateText = $derived.by(() => {
  if (status === null) return t("virtualCamera.state.unknown");
  switch (status.state) {
    case "stopped":
      return t("virtualCamera.state.stopped");
    case "starting":
      return t("virtualCamera.state.starting");
    case "running":
      return t("virtualCamera.state.running", {
        device: status.device,
        width: status.settings.width,
        height: status.settings.height,
        fps: formatNumber(status.settings.fps),
      });
    case "failed":
      return t("virtualCamera.state.failed");
  }
});

async function run(action: (host: StudioHost) => Promise<void>) {
  if (host === null) return;
  error = null;
  try {
    await action(host);
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  }
}

function update(patch: Partial<VirtualCameraSettings>) {
  settings = { ...settings, ...patch };
  saveCameraSettings(settings);
  // 解像度とフレームレートは、動いていれば始め直して効かせる (OS のカメラの形が変わるため)。
  if (active && ("width" in patch || "fps" in patch))
    void run((h) => h.startVirtualCamera(settings));
}

function toggle(on: boolean) {
  void run((h) => (on ? h.startVirtualCamera(settings) : h.stopVirtualCamera()));
}

function changeResolution(value: string) {
  const found = CAMERA_RESOLUTIONS.find(([w, h]) => `${w}x${h}` === value);
  if (found !== undefined) update({ width: found[0], height: found[1] });
}

function changeRate(value: string) {
  const fps = CAMERA_FRAME_RATES.find((r) => String(r) === value);
  if (fps !== undefined) update({ fps });
}

function changeBackground(hex: string) {
  const { r, g, b } = hexToRgb(hex);
  update({ background: [r, g, b] });
  if (active) void run((h) => h.setVirtualCameraBackground([r, g, b]));
}

function errorText(code: string): string {
  switch (code) {
    case "unsupported":
    case "windowsTooOld":
    case "macosTooOld":
    case "needsApproval":
    case "notInApplications":
    case "extensionFailed":
    case "notRegistered":
    case "noDevice":
    case "busy":
    case "invalidConfig":
    case "gpu":
    case "model":
      return t(`virtualCamera.errors.${code}`);
    default:
      return t("virtualCamera.errors.os");
  }
}

function noticeTitle(notice: CameraNotice): string {
  switch (notice.kind) {
    case "unavailable":
      return t("virtualCamera.notices.unavailable");
    case "failed":
      return t("virtualCamera.notices.failed");
    case "noConsumer":
      return t("virtualCamera.notices.noConsumer");
    case "late":
      return t("virtualCamera.notices.late");
  }
}

function guideOf(notice: CameraNotice): CameraGuide | null {
  return notice.kind === "unavailable" || notice.kind === "failed" ? notice.guide : null;
}
</script>

<div class="camera">
  <Toggle
    label={t("virtualCamera.enable")}
    checked={active}
    disabled={host === null || status === null || status.state === "starting"}
    onchange={toggle}
  />
  <p class="camera__state" role="status">{stateText}</p>
  <Select
    label={t("virtualCamera.resolution")}
    value={`${settings.width}x${settings.height}`}
    options={resolutionOptions}
    disabled={host === null}
    onchange={changeResolution}
  />
  <SegmentedControl
    label={t("virtualCamera.fps")}
    value={String(settings.fps)}
    options={rateOptions}
    disabled={host === null}
    onchange={changeRate}
  />
  <ColorPicker
    label={t("virtualCamera.background")}
    value={rgbToHex({ r: settings.background[0], g: settings.background[1], b: settings.background[2] })}
    disabled={host === null}
    onchange={changeBackground}
  />
  <p class="camera__hint">{t("virtualCamera.backgroundHint")}</p>

  {#if error !== null}
    <Notice tone="danger">{error}</Notice>
  {/if}

  {#each notices as notice (notice.kind)}
    <Notice tone={notice.tone} title={noticeTitle(notice)}>
      {#if notice.kind === "unavailable" || notice.kind === "failed"}
        <p class="camera__text">{errorText(notice.code)}</p>
        {#if notice.kind === "failed" && notice.detail !== ""}
          <p class="camera__detail">{notice.detail}</p>
        {/if}
      {:else if notice.kind === "noConsumer"}
        <p class="camera__text">{t("virtualCamera.notices.noConsumerHint")}</p>
      {:else}
        <p class="camera__text">{t("virtualCamera.notices.lateHint")}</p>
      {/if}
      {#if guideOf(notice) === "linux"}
        <p class="camera__text">{t("virtualCamera.guide.linux.intro")}</p>
        <pre class="camera__code">{t("virtualCamera.guide.linux.install")}

{t("virtualCamera.guide.linux.load")}</pre>
        <p class="camera__text">{t("virtualCamera.guide.linux.after")}</p>
      {:else if guideOf(notice) === "windows"}
        <p class="camera__text">{t("virtualCamera.guide.windows")}</p>
      {:else if guideOf(notice) === "reinstall"}
        <p class="camera__text">{t("virtualCamera.guide.reinstall")}</p>
      {/if}
    </Notice>
  {/each}
</div>

<style>
  .camera {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .camera__state {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .camera__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }

  .camera__text,
  .camera__detail {
    margin: 0 0 var(--ugoe-space-2);
  }

  .camera__detail {
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
    overflow-wrap: anywhere;
  }

  .camera__code {
    margin: 0 0 var(--ugoe-space-2);
    padding: var(--ugoe-space-2);
    background: var(--ugoe-color-bg);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-sm);
    font-family: var(--ugoe-font-mono);
    font-size: var(--ugoe-font-size-sm);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
