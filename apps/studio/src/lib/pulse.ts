// 入力の処理を回す刻み (T0803、docs/studio.md#隠れても止まらない)。
//
// 操作ウィンドウを最小化すると、WebView (Linux の WebKitGTK) は `requestAnimationFrame` を止める。カメラとマイクの処理は
// バックエンドの刻みのスレッドが送るメッセージ (`subscribe_pulse`) で回す (Channel のメッセージは隠れていても届く)。
// 刻みを受け取れないとき (ヘッドレスのブラウザの口が持たないなど) は `requestAnimationFrame` で回す。
//
// 処理 (MediaPipe の検出など) が刻みの間隔より重いと、届いた刻みが WebView に溜まり続け、ほかのメッセージや `invoke` の応答が
// 何秒も待たされる (T0812)。前の処理が終わってから前の処理にかかった時間が経つまでは、刻みを飛ばして間引く (飛ばすのは安い)。
import { animationFrames, type TrackerSchedule } from "@ugoe/face-tracking";

type Step = (now: number) => void;

export class Pulse {
  readonly #steps = new Set<Step>();
  readonly #clock: () => number;
  #connected = false;
  /** 前の処理が終わった時刻。 */
  #lastEnd = -Infinity;
  /** 前の処理にかかった時間 (ミリ秒)。 */
  #lastCost = 0;

  /** @param clock 今の時刻 (ミリ秒)。テストで置き換える。 */
  constructor(clock: () => number = () => performance.now()) {
    this.#clock = clock;
  }

  /** バックエンドの刻みにつないだ (つなぐまでは `requestAnimationFrame` で回す)。 */
  connect(): void {
    this.#connected = true;
  }

  get connected(): boolean {
    return this.#connected;
  }

  /**
   * 刻みが届いた: 登録した処理をすべて呼ぶ。前の処理が終わってから前の処理にかかった時間が経っていなければ飛ばす。
   * 1 つの処理が投げても、ほかの処理と次の刻みは止めない (投げたものは console に出す)。
   */
  beat(now: number): void {
    if (now - this.#lastEnd < this.#lastCost) return;
    for (const step of this.#steps) {
      try {
        step(now);
      } catch (e) {
        console.error(e);
      }
    }
    this.#lastEnd = this.#clock();
    this.#lastCost = this.#lastEnd - now;
  }

  /** `step` を刻みごとに呼ぶ。止める関数を返す (FaceTracker の `schedule` の形)。 */
  readonly schedule: TrackerSchedule = (step) => {
    if (!this.#connected) return animationFrames(step);
    this.#steps.add(step);
    return () => {
      this.#steps.delete(step);
    };
  };
}
