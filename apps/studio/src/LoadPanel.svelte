<!--
  刻みの速さと負荷 (docs/studio.md#負荷の表示)。操作ウィンドウだけに出す (出力ウィンドウには何も重ねない)。
  指揮役の 1 刻みの時間・実際の刻みの速さと揺れ・出力ごとの描画の回数と時間と捨てたフレームを並べ、遅れていれば注意を出す。
  使う GPU (自動・省電力・高性能) を選び、受け手ごとに描いている API と GPU の名前を出す (T0860、docs/studio.md#使う-gpu)。
-->
<script lang="ts">
import { formatNumber } from "@ugoe/i18n";
import { Button, Notice, SegmentedControl } from "@ugoe/ui";
import type { GpuPreference, GpuView, RendererLoad, StatusMessage } from "./lib/host";
import { t } from "./lib/i18n";
import { type LoadWarning, loadWarnings, rendererGpu } from "./lib/load";
import { receiverName } from "./lib/receivers";
import { TICK_RATES, type TickRate } from "./lib/settings";

interface Props {
  /** 選んでいる刻みの速さ。 */
  tickRate: TickRate;
  /** 指揮役からの最新の値と負荷 (モデルを開くまでは `null`)。 */
  status: StatusMessage | null;
  /** 使う GPU の様子 (取れるまでは `null`)。 */
  gpu: GpuView | null;
  disabled?: boolean;
  onTickRate: (rate: TickRate) => void;
  onGpuPreference: (preference: GpuPreference) => void;
  /** Windows の「グラフィックの設定」を開く。 */
  onGraphicsSettings: () => void;
}

let {
  tickRate,
  status,
  gpu,
  disabled = false,
  onTickRate,
  onGpuPreference,
  onGraphicsSettings,
}: Props = $props();

const GPU_PREFERENCES: readonly GpuPreference[] = ["auto", "lowPower", "highPerformance"];

function gpuLabel(preference: GpuPreference): string {
  switch (preference) {
    case "auto":
      return t("load.gpuAuto");
    case "lowPower":
      return t("load.gpuLowPower");
    case "highPerformance":
      return t("load.gpuHighPerformance");
  }
}

const gpuOptions = $derived(GPU_PREFERENCES.map((p) => ({ value: p, label: gpuLabel(p) })));

const rateOptions = $derived(
  TICK_RATES.map((rate) => ({
    value: String(rate),
    label: t("load.rateValue", { value: rate }),
  })),
);
const warnings = $derived(status === null ? [] : loadWarnings(status));

function ms(value: number): string {
  return t("load.ms", { value: formatNumber(value, { maximumFractionDigits: 2 }) });
}

function perSecond(value: number): string {
  return t("load.perSecond", { value: formatNumber(value, { maximumFractionDigits: 1 }) });
}

function warningText(w: LoadWarning): string {
  switch (w.kind) {
    case "rateShortfall":
      return t("load.warnings.rateShortfall", {
        measured: perSecond(w.measured),
        target: perSecond(w.target),
      });
    case "intervalSpike":
      return t("load.warnings.intervalSpike", { max: ms(w.maxMs), period: ms(w.periodMs) });
    case "tickHeavy":
      return t("load.warnings.tickHeavy", { tick: ms(w.tickMs), period: ms(w.periodMs) });
    case "renderHeavy":
      return t("load.warnings.renderHeavy", {
        name: receiverName(w.label),
        render: ms(w.renderMs),
        period: ms(w.periodMs),
      });
    case "renderSlow":
      return t("load.warnings.renderSlow", {
        name: receiverName(w.label),
        fps: perSecond(w.fps),
        target: perSecond(w.target),
      });
    case "dropped":
      return t("load.warnings.dropped", { name: receiverName(w.label), count: w.dropped });
    case "stale":
      return t("load.warnings.stale", { name: receiverName(w.label) });
  }
}

function changeRate(value: string) {
  const rate = TICK_RATES.find((r) => String(r) === value);
  if (rate !== undefined) onTickRate(rate);
}

function changeGpu(value: string) {
  const preference = GPU_PREFERENCES.find((p) => p === value);
  if (preference !== undefined) onGpuPreference(preference);
}

/** 受け手が描いている API と GPU の名前 (報告に無ければ `null`)。 */
function gpuText(r: RendererLoad): string | null {
  const used = rendererGpu(r);
  if (used === null) return null;
  return t("load.rendererGpu", {
    backend: used.backend,
    adapter: used.adapter ?? t("load.adapterHidden"),
  });
}
</script>

<div class="load">
  <SegmentedControl
    label={t("load.tickRate")}
    value={String(tickRate)}
    options={rateOptions}
    {disabled}
    onchange={changeRate}
  />
  <p class="load__hint">{t("load.tickRateHint")}</p>

  <SegmentedControl
    label={t("load.gpu")}
    value={gpu?.preference ?? "auto"}
    options={gpuOptions}
    disabled={disabled || gpu === null}
    onchange={changeGpu}
  />
  <p class="load__hint">{t("load.gpuHint")}</p>
  {#if gpu !== null && gpu.preference !== gpu.launched}
    <Notice tone="info">
      {t("load.gpuRestart", { preference: gpuLabel(gpu.preference), launched: gpuLabel(gpu.launched) })}
    </Notice>
  {/if}
  {#if gpu?.graphicsSettings}
    <p class="load__hint">{t("load.gpuWindowsHint")}</p>
    <div>
      <Button small {disabled} onclick={onGraphicsSettings}>{t("load.graphicsSettings")}</Button>
    </div>
  {/if}

  {#if status === null}
    <p class="load__hint">{t("load.empty")}</p>
  {:else}
    <dl class="load__facts">
      <dt>{t("load.tick")}</dt>
      <dd>{ms(status.tickMs)}</dd>
      <dt>{t("load.measured")}</dt>
      <dd>
        {status.measuredRate === null
          ? t("load.measuring")
          : t("load.measuredValue", {
              rate: perSecond(status.measuredRate),
              max: ms(status.maxIntervalMs ?? 0),
            })}
      </dd>
      {#each status.renderers as r (r.label)}
        <dt>{receiverName(r.label)}</dt>
        <dd>
          {r.stale
            ? t("load.stale")
            : t("load.rendererValue", {
                fps: perSecond(r.fps),
                render: ms(r.renderMs),
                max: ms(r.renderMaxMs),
                count: r.dropped,
              })}
          {#if gpuText(r) !== null}
            <span class="load__gpu">{gpuText(r)}</span>
          {/if}
        </dd>
      {/each}
    </dl>
    {#each warnings as w (w.kind + ("label" in w ? w.label : ""))}
      <Notice tone="warning">{warningText(w)}</Notice>
    {/each}
  {/if}
</div>

<style>
  .load {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .load__facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--ugoe-space-2) var(--ugoe-space-4);
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .load__facts dt {
    color: var(--ugoe-color-text-muted);
  }

  .load__facts dd {
    margin: 0;
  }

  .load__gpu {
    display: block;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
    overflow-wrap: anywhere;
  }

  .load__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
