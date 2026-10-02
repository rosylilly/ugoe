// 負荷の表示のための判断 (docs/studio.md#負荷の表示)。操作ウィンドウだけに出す (出力ウィンドウには何も重ねない)。
import type { RendererLoad, StatusMessage } from "./host";

/** 実際の刻みの速さが設定のこの割合を下回ったら「刻みが遅れている」。 */
export const RATE_SHORTFALL = 0.95;
/** 刻みの間隔の最大が、設定の間隔のこの倍を超えたら「刻みが揺れている」(OS のスリープが粗いなど)。 */
export const INTERVAL_SPIKE = 2;
/** 1 刻みの時間・1 回の描画の時間が、刻みの間隔のこの割合を超えたら「重い」。 */
export const BUDGET_SHARE = 0.5;

/** 負荷の注意。文言は呼び手が翻訳する (`load.warnings.<kind>`)。 */
export type LoadWarning =
  | { kind: "rateShortfall"; measured: number; target: number }
  | { kind: "intervalSpike"; maxMs: number; periodMs: number }
  | { kind: "tickHeavy"; tickMs: number; periodMs: number }
  | { kind: "renderHeavy"; label: string; renderMs: number; periodMs: number }
  | { kind: "renderSlow"; label: string; fps: number; target: number }
  | { kind: "dropped"; label: string; dropped: number }
  | { kind: "stale"; label: string };

/** 刻みの間隔 (ミリ秒)。 */
export function periodMs(tickRate: number): number {
  return 1000 / tickRate;
}

function rendererWarnings(load: RendererLoad, tickRate: number): LoadWarning[] {
  // 隠れているなど、報告が途絶えている出力の数値は古いので、ほかの注意は出さない。
  if (load.stale) return [{ kind: "stale", label: load.label }];
  const out: LoadWarning[] = [];
  const period = periodMs(tickRate);
  if (load.renderMs > period * BUDGET_SHARE)
    out.push({ kind: "renderHeavy", label: load.label, renderMs: load.renderMs, periodMs: period });
  if (load.fps < tickRate * RATE_SHORTFALL)
    out.push({ kind: "renderSlow", label: load.label, fps: load.fps, target: tickRate });
  if (load.dropped > 0) out.push({ kind: "dropped", label: load.label, dropped: load.dropped });
  return out;
}

/** 今の負荷から出す注意 (無ければ空)。 */
export function loadWarnings(status: StatusMessage): LoadWarning[] {
  const out: LoadWarning[] = [];
  const period = periodMs(status.tickRate);
  if (status.measuredRate !== null && status.measuredRate < status.tickRate * RATE_SHORTFALL)
    out.push({ kind: "rateShortfall", measured: status.measuredRate, target: status.tickRate });
  if (status.maxIntervalMs !== null && status.maxIntervalMs > period * INTERVAL_SPIKE)
    out.push({ kind: "intervalSpike", maxMs: status.maxIntervalMs, periodMs: period });
  if (status.tickMs > period * BUDGET_SHARE)
    out.push({ kind: "tickHeavy", tickMs: status.tickMs, periodMs: period });
  for (const load of status.renderers) out.push(...rendererWarnings(load, status.tickRate));
  return out;
}
