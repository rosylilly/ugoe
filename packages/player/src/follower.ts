// 受け手のモード (スタジオの出力ウィンドウ。ADR 0028 の 3.、docs/studio.md#フレームの配信-内部の形)。
//
// 指揮役 (スタジオのバックエンド) が刻みごとに配るフレーム `{seq, revision, dt, values, lipSync}` を、`seq` の順に 1 つずつ
// `WebPuppet.follow(values, dt, level, strength)` で適用する (値を基準値として書き、物理 → リップシンク → ポーズの順に進める。
// docs/spec/motion.md#受け手として進める)。描くのは最後のフレームだけでよい。
// フレームのバイト列の形は apps/studio/src-tauri/src/stream.rs の `Frame::encode` と同じ (両方のテストが同じバイト列を使う)。

/** フレームのバイト列の先頭の種類の値。 */
export const FRAME_KIND = 1;
/** フラグ: このフレームを適用する前に物理をリセットする。 */
export const FLAG_RESET = 1;
/** フラグ: リップシンクを当てる (音量と強さが入っている)。 */
export const FLAG_LIP_SYNC = 2;
/** フレームの頭の大きさ (バイト)。 */
export const FRAME_HEADER = 32;
/** 適用を待つフレームの上限 (60 刻みで 10 秒)。超えたら古いものから捨てる (捨てた後は `seq` が飛んだものとして扱う)。 */
export const MAX_PENDING_FRAMES = 600;

/** 指揮役の 1 刻み分のフレーム。 */
export interface FollowFrame {
  seq: number;
  /** どのシーンの値か。 */
  revision: number;
  /** この刻みで進める時間 (秒)。 */
  dt: number;
  /** 適用する前に物理をリセットするか。 */
  reset: boolean;
  /** 指揮役の物理の段に入る直前のパラメータの値 (シーンのパラメータの ID の順。モデルの定義順)。 */
  values: Float32Array;
  /** 指揮役のリップシンクの音量と強さ。当てなければ `null`。 */
  lipSync: { level: number; strength: number } | null;
}

/** シーンの通知 (JSON)。受け手はこのシーンのモデルを読んでから `revision` のフレームを適用する。 */
export interface SceneMessage {
  kind: "scene";
  revision: number;
  modelName: string;
  parameterIds: string[];
}

/** 受け手が使う `WebPuppet` の口 (wasm.ts の `WebPuppet` の一部)。 */
export interface FollowerPuppet {
  follow(
    values: Float32Array,
    dt: number,
    lipSyncLevel?: number | null,
    lipSyncStrength?: number | null,
  ): void;
  reset_physics(): void;
}

/** フレームのバイト列を読む (リトルエンディアン)。形が違えば例外を投げる。 */
export function decodeFrame(buffer: ArrayBuffer): FollowFrame {
  if (buffer.byteLength < FRAME_HEADER)
    throw new Error(`the frame is too short (${buffer.byteLength} bytes)`);
  const view = new DataView(buffer);
  const kind = view.getUint8(0);
  if (kind !== FRAME_KIND) throw new Error(`not a frame (kind ${kind})`);
  const count = view.getUint32(28, true);
  const flags = view.getUint8(1);
  if (buffer.byteLength - FRAME_HEADER !== count * 4)
    throw new Error(
      `the frame has ${count} values but ${buffer.byteLength - FRAME_HEADER} bytes of values`,
    );
  // 値は 4 の倍数の位置から始まるので、写さずに重ねられる (Float32Array はプラットフォームのバイト順だが、
  // WebView が動く環境はすべてリトルエンディアン)。
  return {
    seq: Number(view.getBigUint64(8, true)),
    revision: view.getUint32(4, true),
    dt: view.getFloat32(16, true),
    reset: (flags & FLAG_RESET) !== 0,
    lipSync:
      (flags & FLAG_LIP_SYNC) !== 0
        ? { level: view.getFloat32(20, true), strength: view.getFloat32(24, true) }
        : null,
    values: new Float32Array(buffer, FRAME_HEADER, count),
  };
}

/**
 * フレームを 1 つ適用する。受け手で `follow` を呼ぶのはここだけにする (T0134 で引数が増えたときに合わせる場所を 1 つにするため)。
 * `gap` なら (`seq` が飛んだ) 物理をリセットしてから適用する。
 */
export function applyFrame(puppet: FollowerPuppet, frame: FollowFrame, gap = false): void {
  if (frame.reset || gap) puppet.reset_physics();
  puppet.follow(frame.values, frame.dt, frame.lipSync?.level, frame.lipSync?.strength);
}

/**
 * 届いたフレームを、今のシーンの順に並べて渡す入れ物。
 *
 * - 前のシーンのフレームは捨てる。まだ読んでいないシーンのフレームは、そのシーンのモデルを読むまで待たせる
 *   (新しいシーンの最初のフレームから適用するので、どの受け手も同じ形になる)。
 * - 同じシーンの中で `seq` が飛んだら (接続が切れていた・溢れて捨てた)、物理をリセットしてから続ける。
 * - 同じモデルのまま次のシーンに移ったとき (`continueScene`。見せ方や着せ替えだけが変わった) は、前のシーンのフレームも
 *   捨てずに順に適用し、`seq` の続きも保つ (受け手ごとに物理の刻みが抜けないように)。
 */
export class FrameQueue {
  #pending: FollowFrame[] = [];
  #revision: number | null = null;
  /** 適用するフレームの `revision` の下限 (モデルを読んだシーン。`continueScene` では変わらない)。 */
  #from: number | null = null;
  #lastSeq: number | null = null;
  /** 溢れて捨てたので、次に適用するフレームは `seq` が飛んだものとして扱う。 */
  #dropped = false;

  /** 今のシーン (モデルを読み終えたもの)。 */
  get revision(): number | null {
    return this.#revision;
  }

  /** 待っているフレームの数。 */
  get pending(): number {
    return this.#pending.length;
  }

  /** シーン `revision` のモデルを読み終えた。以後はそのシーンのフレームを最初から適用する。 */
  setScene(revision: number): void {
    this.#revision = revision;
    this.#from = revision;
    this.#lastSeq = null;
    this.#dropped = false;
  }

  /**
   * 同じモデルのまま、シーン `revision` に移る。前のシーンのフレームも捨てずに順に適用し、`seq` の続きも保つ。
   * まだシーンが無ければ `setScene` と同じ。
   */
  continueScene(revision: number): void {
    if (this.#revision === null || this.#from === null) {
      this.setScene(revision);
      return;
    }
    this.#revision = Math.max(this.#revision, revision);
  }

  push(frame: FollowFrame): void {
    this.#pending.push(frame);
    if (this.#pending.length > MAX_PENDING_FRAMES) {
      this.#pending.splice(0, this.#pending.length - MAX_PENDING_FRAMES);
      this.#dropped = true;
    }
  }

  /**
   * 今のシーンで適用できるフレームを順に取り出す (`gap` は物理をリセットしてから適用するか)。
   * まだ読んでいないシーンのフレームに当たったら、そこで止める (残りは待たせる)。
   */
  take(): { frame: FollowFrame; gap: boolean }[] {
    const out: { frame: FollowFrame; gap: boolean }[] = [];
    const revision = this.#revision;
    const from = this.#from ?? revision;
    if (revision === null || from === null) return out;
    let i = 0;
    for (; i < this.#pending.length; i++) {
      const frame = this.#pending[i] as FollowFrame;
      if (frame.revision < from) continue;
      if (frame.revision > revision) break;
      const last = this.#lastSeq;
      const gap = this.#dropped || (last !== null && frame.seq !== last + 1);
      this.#dropped = false;
      this.#lastSeq = frame.seq;
      out.push({ frame, gap });
    }
    this.#pending.splice(0, i);
    return out;
  }
}

/** 受け手のモードで 1 回描いたときの負荷 (`Player.startFollowing` の引数に渡す。スタジオの負荷の表示、docs/studio.md#負荷の表示)。 */
export interface FollowRenderStats {
  /** 描いた時刻 (`requestAnimationFrame` の時刻。ミリ秒)。 */
  at: number;
  /** 前に描いてからのフレームの適用と、この描画にかかった CPU の時間 (ミリ秒)。 */
  renderMs: number;
  /** 溢れて捨てたフレームの数 (受け手を作ってからの合計)。 */
  dropped: number;
}
