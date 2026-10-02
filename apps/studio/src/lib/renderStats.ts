// 出力ウィンドウの描画の負荷を約 1 秒ごとにまとめる (docs/studio.md#負荷の表示)。まとめた値は操作ウィンドウへ送る。
import type { RenderStats } from "./host";

/** まとめる長さ (ミリ秒)。 */
export const REPORT_INTERVAL_MS = 1000;

export class RenderStatsWindow {
  #start: number | null = null;
  #count = 0;
  #total = 0;
  #max = 0;

  /**
   * 1 回描いた。`at` は描いた時刻、`renderMs` はその CPU の時間 (ミリ秒)、`dropped` は捨てたフレームの合計。
   * 約 1 秒たまったら、まとめた値を返す (それ以外は `null`)。
   */
  record(at: number, renderMs: number, dropped: number): RenderStats | null {
    if (this.#start === null) {
      this.#start = at;
      return null;
    }
    this.#count += 1;
    this.#total += renderMs;
    this.#max = Math.max(this.#max, renderMs);
    const elapsed = at - this.#start;
    if (elapsed < REPORT_INTERVAL_MS) return null;
    const stats: RenderStats = {
      fps: (this.#count * 1000) / elapsed,
      renderMs: this.#total / this.#count,
      renderMaxMs: this.#max,
      dropped,
    };
    this.#start = at;
    this.#count = 0;
    this.#total = 0;
    this.#max = 0;
    return stats;
  }
}
