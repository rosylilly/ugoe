// 出力ウィンドウの描画の負荷を約 1 秒ごとにまとめる (docs/studio.md#負荷の表示)。まとめた値は操作ウィンドウへ送る。
import type { FollowRenderStats } from "@ugoe/player";
import type { RenderStats } from "./host";

/** まとめる長さ (ミリ秒)。 */
export const REPORT_INTERVAL_MS = 1000;

export class RenderStatsWindow {
  #start: number | null = null;
  /** `requestAnimationFrame` の回数 (描く機会)。 */
  #frames = 0;
  /** 描いた回数。 */
  #count = 0;
  #total = 0;
  #max = 0;

  /**
   * `requestAnimationFrame` が 1 回来た (受け手は変わったときだけ描くので、描かなかった回も来る)。
   * 約 1 秒たまったら、まとめた値を返す (それ以外は `null`)。描く回数と時間は描いた回だけを数える。
   */
  record({ at, drawn, renderMs, dropped }: FollowRenderStats): RenderStats | null {
    if (this.#start === null) {
      this.#start = at;
      return null;
    }
    this.#frames += 1;
    if (drawn) {
      this.#count += 1;
      this.#total += renderMs;
      this.#max = Math.max(this.#max, renderMs);
    }
    const elapsed = at - this.#start;
    if (elapsed < REPORT_INTERVAL_MS) return null;
    const stats: RenderStats = {
      fps: (this.#count * 1000) / elapsed,
      refreshFps: (this.#frames * 1000) / elapsed,
      renderMs: this.#count === 0 ? 0 : this.#total / this.#count,
      renderMaxMs: this.#max,
      dropped,
    };
    this.#start = at;
    this.#frames = 0;
    this.#count = 0;
    this.#total = 0;
    this.#max = 0;
    return stats;
  }
}
