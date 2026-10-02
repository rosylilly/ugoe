// 入力の処理を回す刻み (T0803、docs/studio.md#隠れても止まらない)。
//
// 操作ウィンドウを最小化すると、WebView (Linux の WebKitGTK) は `requestAnimationFrame` を止める。カメラとマイクの処理は
// バックエンドの刻みのスレッドが送るメッセージ (`subscribe_pulse`) で回す (Channel のメッセージは隠れていても届く)。
// 刻みを受け取れないとき (ヘッドレスのブラウザの口が持たないなど) は `requestAnimationFrame` で回す。
import { animationFrames, type TrackerSchedule } from "@ugoe/face-tracking";

type Step = (now: number) => void;

export class Pulse {
  readonly #steps = new Set<Step>();
  #connected = false;

  /** バックエンドの刻みにつないだ (つなぐまでは `requestAnimationFrame` で回す)。 */
  connect(): void {
    this.#connected = true;
  }

  get connected(): boolean {
    return this.#connected;
  }

  /** 刻みが届いた: 登録した処理をすべて呼ぶ。 */
  beat(now: number): void {
    for (const step of this.#steps) step(now);
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
