<!--
  刻みの速さ・描く解像度と負荷 (docs/studio.md#負荷の表示)。操作ウィンドウだけに出す (出力ウィンドウには何も重ねない)。
  描く解像度 (T0858) は出力ウィンドウとブラウザソースの描画バッファの倍率で、選ぶと受け手の次のフレームから効く。
  指揮役の 1 刻みの時間・実際の刻みの速さと揺れ・出力ごとの描画の回数と時間と捨てたフレームを並べ、遅れていれば注意を出す。
  使う GPU (自動・省電力・高性能) を選び、受け手ごとに描いている API と GPU の名前を出す (T0860、docs/studio.md#使う-gpu)。
  出力ウィンドウとブラウザソースの行には、描いた回数と並べて描く機会 (`requestAnimationFrame`) の回数を出す (T0862)。
  読み上げの区切り (T0862): 受け手ごとの行 (`<dd>`) の値は、見える区切りの文字 (`load.separator`。「、」・", ") でつないだ 1 つの文にする。
  「・」は読み上げで黙って飛ばされることがあるので使わない。2 行目 (API と GPU の名前) の前は見た目では改行なので、
  区切りの文字を `.ugoe-visually-hidden` で読み上げにだけ置く (`aria-label` は `<dd>` の名前にならないので使わない)。
  Windows では GPU の優先度 (通常・低い) を選び、当てられなかった理由を出す (T0861、docs/studio.md#gpu-の優先度)。
-->
<script lang="ts">
import { formatNumber } from "@ugoe/i18n";
import { Button, Notice, SegmentedControl } from "@ugoe/ui";
import { describeError } from "./lib/errors";
import type { GpuPreference, GpuPriority, GpuView, RendererLoad, StatusMessage } from "./lib/host";
import { t } from "./lib/i18n";
import { type LoadWarning, loadWarnings, rendererGpu } from "./lib/load";
import { receiverName } from "./lib/receivers";
import { RENDER_SCALES, type RenderScale, TICK_RATES, type TickRate } from "./lib/settings";

interface Props {
  /** 選んでいる刻みの速さ。 */
  tickRate: TickRate;
  /** 選んでいる描く解像度の倍率。 */
  renderScale: RenderScale;
  /** 指揮役からの最新の値と負荷 (モデルを開くまでは `null`)。 */
  status: StatusMessage | null;
  /** 使う GPU の様子 (取れるまでは `null`)。 */
  gpu: GpuView | null;
  disabled?: boolean;
  onTickRate: (rate: TickRate) => void;
  onRenderScale: (scale: RenderScale) => void;
  onGpuPreference: (preference: GpuPreference) => void;
  /** GPU の優先度を選ぶ (Windows だけ)。 */
  onGpuPriority: (priority: GpuPriority) => void;
  /** Windows の「グラフィックの設定」を開く。 */
  onGraphicsSettings: () => void;
}

let {
  tickRate,
  renderScale,
  status,
  gpu,
  disabled = false,
  onTickRate,
  onRenderScale,
  onGpuPreference,
  onGpuPriority,
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

const GPU_PRIORITIES: readonly GpuPriority[] = ["normal", "low"];

const priorityOptions = $derived(
  GPU_PRIORITIES.map((p) => ({
    value: p,
    label: p === "normal" ? t("load.priorityNormal") : t("load.priorityLow"),
  })),
);

const rateOptions = $derived(
  TICK_RATES.map((rate) => ({
    value: String(rate),
    label: t("load.rateValue", { value: rate }),
  })),
);
const scaleOptions = $derived(
  RENDER_SCALES.map((scale) => ({
    value: String(scale),
    label: t("load.renderScaleValue", { value: scale * 100 }),
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

function changeScale(value: string) {
  const scale = RENDER_SCALES.find((s) => String(s) === value);
  if (scale !== undefined) onRenderScale(scale);
}

function changeGpu(value: string) {
  const preference = GPU_PREFERENCES.find((p) => p === value);
  if (preference !== undefined) onGpuPreference(preference);
}

function changePriority(value: string) {
  const priority = GPU_PRIORITIES.find((p) => p === value);
  if (priority !== undefined) onGpuPriority(priority);
}

/**
 * 受け手の行の 1 行目の値 (描画の回数と描く機会の回数・1 回の時間・捨てたフレーム)。描く機会の回数 (`refreshFps`) は
 * 出力ウィンドウとブラウザソースだけが報告する (仮想カメラは描く機会ごとに描くので出さない)。
 */
function rendererValues(r: RendererLoad): string[] {
  if (r.stale) return [t("load.stale")];
  return [
    r.refreshFps === null
      ? t("load.drawn", { fps: perSecond(r.fps) })
      : t("load.drawnRefresh", { fps: perSecond(r.fps), refresh: perSecond(r.refreshFps) }),
    t("load.renderTime", { render: ms(r.renderMs), max: ms(r.renderMaxMs) }),
    t("load.dropped", { count: r.dropped }),
  ];
}

/** 受け手が描いている API と GPU の名前 (報告に無ければ空)。 */
function gpuValues(r: RendererLoad): string[] {
  const used = rendererGpu(r);
  if (used === null) return [];
  return [used.backend, used.adapter ?? t("load.adapterHidden")];
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
    label={t("load.renderScale")}
    value={String(renderScale)}
    options={scaleOptions}
    {disabled}
    onchange={changeScale}
  />
  <p class="load__hint">{t("load.renderScaleHint")}</p>

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

  {#if gpu?.prioritySupported}
    <SegmentedControl
      label={t("load.priority")}
      value={gpu.priority}
      options={priorityOptions}
      {disabled}
      onchange={changePriority}
    />
    <p class="load__hint">{t("load.priorityHint")}</p>
    {#each gpu.priorityErrors as e (e.code + JSON.stringify(e.args))}
      <Notice tone="warning">{describeError(e)}</Notice>
    {/each}
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
          <!-- 2 行目は見た目では改行で分かれるので、読み上げのためだけに区切りを置く (間に空白を入れない) -->
          {rendererValues(r).join(t("load.separator"))}{#if gpuValues(r).length > 0}<span class="load__gpu"
              ><span class="ugoe-visually-hidden">{t("load.separator")}</span>{gpuValues(r).join(t("load.separator"))}</span
            >{/if}
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
    /* 読み上げにだけ置く区切り (`position: absolute`) を、この行の中に留める */
    position: relative;
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
