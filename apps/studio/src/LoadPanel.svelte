<!--
  刻みの速さと負荷 (docs/studio.md#負荷の表示)。操作ウィンドウだけに出す (出力ウィンドウには何も重ねない)。
  指揮役の 1 刻みの時間・実際の刻みの速さと揺れ・出力ごとの描画の回数と時間と捨てたフレームを並べ、遅れていれば注意を出す。
-->
<script lang="ts">
import { formatNumber } from "@ugoe/i18n";
import { Notice, SegmentedControl } from "@ugoe/ui";
import type { StatusMessage } from "./lib/host";
import { t } from "./lib/i18n";
import { type LoadWarning, loadWarnings } from "./lib/load";
import { TICK_RATES, type TickRate } from "./lib/settings";

interface Props {
  /** 選んでいる刻みの速さ。 */
  tickRate: TickRate;
  /** 指揮役からの最新の値と負荷 (モデルを開くまでは `null`)。 */
  status: StatusMessage | null;
  disabled?: boolean;
  onTickRate: (rate: TickRate) => void;
}

let { tickRate, status, disabled = false, onTickRate }: Props = $props();

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

function receiverName(label: string): string {
  if (label === "output") return t("load.outputWindow");
  if (label === "virtualCamera") return t("load.virtualCamera");
  // ブラウザソースのページ (つないだ順に `browserSource`・`browserSource:2`…。api/source.rs の `LABEL`)。
  if (label === "browserSource") return t("load.browserSource");
  const page = /^browserSource:(\d+)$/.exec(label);
  if (page !== null) return t("load.browserSourceN", { n: Number(page[1]) });
  return label;
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

  .load__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
