// 受け手のモード (スタジオの出力ウィンドウ。ADR 0028 の 3.、docs/studio.md#フレームの配信-内部の形)。
//
// 指揮役 (スタジオのバックエンド) が刻みごとに配るフレーム `{seq, revision, dt, values, lipSync, partOpacity*}` を、`seq` の順に
// 1 つずつ `WebPuppet.follow(values, dt, level, strength, parts, opacities)` で適用する (値を基準値として書き、物理 → リップシンク →
// パーツの不透明度 (モーションのカーブとポーズ) の順に進める。docs/spec/motion.md#受け手として進める)。描くのは最後のフレームだけでよい。
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
  /**
   * 指揮役のモーションのパーツの不透明度のカーブがあるパーツの番号 (このフレームのシーンの `partIds` の順。T0135)。
   * 受け手は `FrameQueue.take` が渡す `PartRemap` で自分のモデルの番号に引き直して当てる (T0853)。
   */
  partOpacityParts: Uint32Array;
  /** `partOpacityParts` の各パーツのカーブの値と重み (交互。`partOpacityParts` の 2 倍の数)。 */
  partOpacityValues: Float32Array;
}

/** シーンの通知 (JSON)。受け手はこのシーンのモデルを読んでから `revision` のフレームを適用する。 */
export interface SceneMessage {
  kind: "scene";
  revision: number;
  modelName: string;
  parameterIds: string[];
  /** フレームのパーツの番号の並び (指揮役の合成したモデルの `parts` の ID の順。モジュールのパーツは `<モジュール>/<パーツ>`。T0853)。 */
  partIds: string[];
}

/** 受け手が使う `WebPuppet` の口 (wasm.ts の `WebPuppet` の一部)。 */
export interface FollowerPuppet {
  follow(
    values: Float32Array,
    dt: number,
    lipSyncLevel?: number | null,
    lipSyncStrength?: number | null,
    partOpacityParts?: Uint32Array | null,
    partOpacityValues?: Float32Array | null,
  ): void;
  reset_physics(): void;
}

/**
 * フレームのバイト列を読む (リトルエンディアン)。形が違えば例外を投げる。
 * 例外の文は開発者向け (指揮役と受け手の食い違い。画面に出さない) なので英語のまま (`i18n-ignore`)。
 */
export function decodeFrame(buffer: ArrayBuffer): FollowFrame {
  if (buffer.byteLength < FRAME_HEADER)
    // i18n-ignore: 開発者向け
    throw new Error(`the frame is too short (${buffer.byteLength} bytes)`);
  const view = new DataView(buffer);
  const kind = view.getUint8(0);
  // i18n-ignore: 開発者向け
  if (kind !== FRAME_KIND) throw new Error(`not a frame (kind ${kind})`);
  const count = view.getUint32(28, true);
  const flags = view.getUint8(1);
  // 値の後に、パーツの不透明度の数・パーツの番号・値と重みが続く (T0135)。
  const partsAt = FRAME_HEADER + count * 4;
  const parts = partsAt + 4 <= buffer.byteLength ? view.getUint32(partsAt, true) : 0;
  const expected = (count + 1 + parts * 3) * 4;
  if (buffer.byteLength - FRAME_HEADER !== expected)
    throw new Error(
      // i18n-ignore: 開発者向け
      `the frame has ${expected} bytes after the header but ${buffer.byteLength - FRAME_HEADER} bytes`,
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
    partOpacityParts: new Uint32Array(buffer, partsAt + 4, parts),
    partOpacityValues: new Float32Array(buffer, partsAt + 4 + parts * 4, parts * 2),
  };
}

/**
 * シーン 1 つのパーツの並び (`partIds`) の番号から、受け手のモデルの番号への引き直し (T0853)。
 * 表はシーンを受けたとき (`FrameQueue.setScene`・`continueScene`) に作り、フレームごとには作らない。
 */
export class PartRemap {
  readonly revision: number;
  readonly partIds: readonly string[];
  /** シーンの番号ごとのモデルの番号 (モデルに無い ID は -1)。並びがモデルと同じなら `null` (引き直さない)。 */
  #table: Int32Array | null = null;
  #parts = new Uint32Array(0);
  #values = new Float32Array(0);
  /** 最後に `remap` で引き直したパーツの番号。 */
  parts: Uint32Array = new Uint32Array(0);
  /** 最後に `remap` で引き直した値と重み (交互)。 */
  values: Float32Array = new Float32Array(0);

  constructor(revision: number, partIds: readonly string[], model: readonly string[]) {
    this.revision = revision;
    this.partIds = partIds;
    this.rebuild(model);
  }

  /** 引き直しが要らない (シーンの並びがモデルと同じ)。 */
  get identity(): boolean {
    return this.#table === null;
  }

  /** モデルのパーツの並び `model` への表を作り直す。 */
  rebuild(model: readonly string[]): void {
    if (sameIds(this.partIds, model)) {
      this.#table = null;
      return;
    }
    const index = new Map(model.map((id, i) => [id, i]));
    this.#table = Int32Array.from(this.partIds, (id) => index.get(id) ?? -1);
  }

  /**
   * フレームのパーツの番号と値を引き直し (モデルに無いパーツは捨てる)、`parts`・`values` に置く (次に呼ぶまで有効。
   * 中身の入れ物は使い回し、引き直しの要らない表では何もしない)。
   */
  remap(parts: Uint32Array, values: Float32Array): void {
    const table = this.#table;
    if (table === null) {
      this.parts = parts;
      this.values = values;
      return;
    }
    if (this.#parts.length < parts.length) {
      this.#parts = new Uint32Array(parts.length);
      this.#values = new Float32Array(parts.length * 2);
    }
    let n = 0;
    for (let i = 0; i < parts.length; i++) {
      const to = table[parts[i] as number] ?? -1;
      if (to < 0) continue;
      this.#parts[n] = to;
      this.#values[n * 2] = values[i * 2] as number;
      this.#values[n * 2 + 1] = values[i * 2 + 1] as number;
      n++;
    }
    this.parts = this.#parts.subarray(0, n);
    this.values = this.#values.subarray(0, n * 2);
  }
}

function sameIds(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((id, i) => id === b[i]);
}

/**
 * フレームを 1 つ適用する。受け手で `follow` を呼ぶのはここだけにする (T0134・T0135 で引数が増えたときに合わせる場所を 1 つにするため)。
 * `gap` なら (`seq` が飛んだ) 物理をリセットしてから適用する。`parts` があれば、パーツの番号をそれで引き直す (T0853)。
 */
export function applyFrame(
  puppet: FollowerPuppet,
  frame: FollowFrame,
  gap = false,
  parts: PartRemap | null = null,
): void {
  if (frame.reset || gap) puppet.reset_physics();
  parts?.remap(frame.partOpacityParts, frame.partOpacityValues);
  puppet.follow(
    frame.values,
    frame.dt,
    frame.lipSync?.level,
    frame.lipSync?.strength,
    parts?.parts ?? frame.partOpacityParts,
    parts?.values ?? frame.partOpacityValues,
  );
}

/**
 * 届いたフレームを、今のシーンの順に並べて渡す入れ物。
 *
 * - 前のシーンのフレームは捨てる。まだ読んでいないシーンのフレームは、そのシーンのモデルを読むまで待たせる
 *   (新しいシーンの最初のフレームから適用するので、どの受け手も同じ形になる)。
 * - 同じシーンの中で `seq` が飛んだら (接続が切れていた・溢れて捨てた)、物理をリセットしてから続ける。
 * - 同じモデルのまま次のシーンに移ったとき (`continueScene`。見せ方や着せ替えだけが変わった) は、前のシーンのフレームも
 *   捨てずに順に適用し、`seq` の続きも保つ (受け手ごとに物理の刻みが抜けないように)。
 * - フレームのパーツの番号は、そのフレームのシーン (受けていなければその前の最も新しいシーン) の `partIds` で ID に直し、
 *   今のシーンの `partIds` (受け手が付けたモデルの並び) の番号に引き直す (着せ替えの直後の前のシーンのフレーム。T0853)。
 */
export class FrameQueue {
  #pending: FollowFrame[] = [];
  #revision: number | null = null;
  /** 適用するフレームの `revision` の下限 (モデルを読んだシーン。`continueScene` では変わらない)。 */
  #from: number | null = null;
  #lastSeq: number | null = null;
  /** 溢れて捨てたので、次に適用するフレームは `seq` が飛んだものとして扱う。 */
  #dropped = false;
  /** 受けたシーンのパーツの並び (`revision` の順。並びが前と同じシーンは足さない)。 */
  #parts: PartRemap[] = [];

  /** 今のシーン (モデルを読み終えたもの)。 */
  get revision(): number | null {
    return this.#revision;
  }

  /** 待っているフレームの数。 */
  get pending(): number {
    return this.#pending.length;
  }

  /**
   * シーン `revision` (パーツの並びは `partIds`) のモデルを読み終えた。以後はそのシーンのフレームを最初から適用する。
   */
  setScene(revision: number, partIds: readonly string[] = []): void {
    this.#parts = [new PartRemap(revision, partIds, partIds)];
    this.#revision = revision;
    this.#from = revision;
    this.#lastSeq = null;
    this.#dropped = false;
  }

  /**
   * 同じモデルのまま、シーン `revision` に移る。前のシーンのフレームも捨てずに順に適用し、`seq` の続きも保つ。
   * まだシーンが無ければ `setScene` と同じ。モジュールを付け直したなら、付け直した後に呼ぶ。
   */
  continueScene(revision: number, partIds: readonly string[] = []): void {
    if (this.#revision === null || this.#from === null) {
      this.setScene(revision, partIds);
      return;
    }
    if (revision > this.#revision) {
      for (const map of this.#parts) map.rebuild(partIds);
      const last = this.#parts.at(-1);
      if (last === undefined || !sameIds(last.partIds, partIds))
        this.#parts.push(new PartRemap(revision, partIds, partIds));
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
   * 今のシーンで適用できるフレームを順に取り出す (`gap` は物理をリセットしてから適用するか、`parts` はパーツの番号の
   * 引き直し。要らなければ `null`。`applyFrame` に渡す)。まだ読んでいないシーンのフレームに当たったら、そこで止める
   * (残りは待たせる)。
   */
  take(): { frame: FollowFrame; gap: boolean; parts: PartRemap | null }[] {
    const out: { frame: FollowFrame; gap: boolean; parts: PartRemap | null }[] = [];
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
      out.push({ frame, gap, parts: this.#partsFor(frame.revision) });
    }
    this.#pending.splice(0, i);
    return out;
  }

  /**
   * シーン `revision` (受けていなければその前の最も新しいシーン) のパーツの引き直し。フレームは順に適用するので、
   * それより前のシーンの並びは捨てる。
   */
  #partsFor(revision: number): PartRemap | null {
    let at = 0;
    for (let i = this.#parts.length - 1; i > 0; i--) {
      if ((this.#parts[i] as PartRemap).revision <= revision) {
        at = i;
        break;
      }
    }
    if (at > 0) this.#parts.splice(0, at);
    const map = this.#parts[0];
    return map === undefined || map.identity ? null : map;
  }
}

/**
 * 受け手のモードの `requestAnimationFrame` 1 回分の負荷 (`Player.startFollowing` の引数に渡す。スタジオの負荷の表示、
 * docs/studio.md#負荷の表示)。受け手は変わったときだけ描くので、描かなかった回も渡す (`drawn` が偽)。
 */
export interface FollowRenderStats {
  /** `requestAnimationFrame` の時刻 (ミリ秒)。 */
  at: number;
  /** この回に描いたか (前に描いてから、新しいフレームを適用したか見せ方が変わった)。 */
  drawn: boolean;
  /** 描いたなら、前に描いてからのフレームの適用と、この描画にかかった CPU の時間 (ミリ秒)。描かなければ 0。 */
  renderMs: number;
  /** 溢れて捨てたフレームの数 (受け手を作ってからの合計)。 */
  dropped: number;
}
