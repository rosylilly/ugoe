// トラッキングの状態の文 (T0958)。顔が見つからない・体が見つからない・止まったなどの状態を、fps・再生の位置と分けて
// パネルの `role="status"` の要素に置く。顔が一瞬だけ外れたような短い出入りで読み上げが続かないよう、状態が変わってから
// 一定の時間続いたときだけ文を変える (docs/ui.md#状態の変化の知らせ)。
import type { TrackerSource, TrackerStatus } from "./tracker.svelte";

/** 読み上げで知らせるトラッキングの状態。 */
export type TrackingCondition =
  | "idle"
  | "loading"
  | "stopped"
  | "noFace"
  | "noBody"
  | "tracking"
  | "replaying";

/** 状態を決める FaceTracker の値。 */
export interface TrackingConditionInput {
  status: TrackerStatus;
  source: Pick<TrackerSource, "kind"> | null;
  faceFound: boolean;
  trackBody: boolean;
  bodyFound: boolean;
}

/** FaceTracker の値から今の状態を決める (揺れは抑えない。抑えるのは `SettledValue`)。 */
export function trackingCondition(input: TrackingConditionInput): TrackingCondition {
  switch (input.status) {
    case "idle":
      return "idle";
    case "loading":
      return "loading";
    case "error":
      return "stopped";
    case "running":
      if (!input.faceFound) return "noFace";
      // 記録は体の値を持たないことがあるので、体が見つからないとは言わない。
      if (input.source?.kind === "replay") return "replaying";
      if (input.trackBody && !input.bodyFound) return "noBody";
      return "tracking";
  }
}

/** 状態が変わってから文を変えるまでの時間 (ミリ秒)。 */
export const CONDITION_SETTLE_MS = 1000;

/**
 * 揺れを抑えた値。入った値が `delayMs` ミリ秒以上続いたときだけ、見せる値をその値にする。途中で別の値に変わったら
 * 待ち直し、見せている値に戻ったら待つのをやめる。時刻は呼ぶ側が渡す (テストで時間を進めるため)。
 */
export class SettledValue<T> {
  #value: T;
  #pending: T;
  #since = 0;

  /**
   * @param initial 最初に見せる値
   * @param delayMs 値が続いてから見せるまでの時間 (ミリ秒)
   * @param immediate 待たずに見せる値 (止めた後の空の文など。読み上げの邪魔にならないもの)
   */
  constructor(
    initial: T,
    readonly delayMs: number,
    readonly immediate: (value: T) => boolean = () => false,
  ) {
    this.#value = initial;
    this.#pending = initial;
  }

  /** 見せている値。 */
  get value(): T {
    return this.#value;
  }

  /** 時刻 `now` (ミリ秒) の値 `next` を入れ、見せる値を返す。 */
  update(next: T, now: number): T {
    if (this.immediate(next)) {
      this.#value = next;
      this.#pending = next;
      return next;
    }
    if (!Object.is(next, this.#pending)) {
      this.#pending = next;
      this.#since = now;
    }
    if (!Object.is(this.#pending, this.#value) && now - this.#since >= this.delayMs) {
      this.#value = this.#pending;
    }
    return this.#value;
  }

  /** 見せる値が変わりうる時刻 (待っている値が無ければ `null`)。この時刻に `update` を呼び直す。 */
  get settlesAt(): number | null {
    return Object.is(this.#pending, this.#value) ? null : this.#since + this.delayMs;
  }
}
