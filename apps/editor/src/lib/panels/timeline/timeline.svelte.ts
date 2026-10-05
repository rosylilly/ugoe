// タイムラインの表示の状態 (選択中のモーション・再生位置・再生・キーの選択) と、パラメータへの書き込み。
// Undo の対象ではない。モーションの編集は EditorDocument.apply (Command) で行う (docs/editor.md#タイムライン)。
//
// 再生・シークは、モーションの各カーブをその時刻で評価した値 (wasm の MotionSampler。フェードなし) を
// スライダーと同じ ViewportController.setParameter で書き込む。MotionSampler はカーブごとの区間の索引を持つので、
// モーション (選び直し・編集の後の新しいオブジェクト) が変わったときだけ set_motion_json で渡し直し、再生中は sample(t) だけを呼ぶ。WebPuppet のモーションの再生は使わないので、
// 編集のたびにモデルを読み直しても再生位置は保たれ、同じ時刻は常に同じ姿になる。
import type { EditorDocument } from "../../document.svelte";
import type { CurveTarget, Expression, Motion } from "../../ipc";
import type { ViewportController } from "../../viewport/controller.svelte";
import { loadUgoeWasm, type MotionSampler } from "../../viewport/wasm";
import { applyExpression } from "./expressions";
import { advance, roundTime, targetKey } from "./timeline";

/** 選んだキー: 行 (targetKey) と時刻。 */
export interface KeySelection {
  track: string;
  t: number;
}

export class TimelineState {
  /** 選択中のモーションの名前。 */
  motionName = $state<string | null>(null);
  /** 再生位置 (秒)。 */
  time = $state(0);
  playing = $state(false);
  /** 再生を最後で折り返す (モーションの `looping` とは別の、表示の設定)。 */
  repeat = $state(true);
  /** 選択中の行 (targetKey)。カーブのグラフに出す。 */
  selectedTrack = $state<string | null>(null);
  selectedKey = $state<KeySelection | null>(null);
  /** プレビュー中の表情の名前。 */
  previewExpression = $state<string | null>(null);

  #doc: EditorDocument;
  #viewport: ViewportController;
  /** wasm の MotionSampler のコンストラクタ (wasm がまだ読めていなければ null)。 */
  #Sampler: (new (json: string) => MotionSampler) | null = null;
  #sampler: MotionSampler | null = null;
  /**
   * `#sampler` が今持っているモーション。ドキュメントは編集のたびにモーションの配列を読み直す (`$state.raw`) ので、
   * オブジェクトが同じなら中身も同じ。違えば索引を作り直す。
   */
  #sampled: Motion | null = null;
  #frame = 0;
  #last = 0;
  /**
   * モーションを選んでから、シーク・再生・キーの操作をしたか。したときだけ、編集の後に姿を書き込み直す
   * (モーションを選んだり開いたりしただけでは、スライダーを動かさない)。
   */
  engaged = false;
  /** 表情のプレビューの前の値 (戻すため)。 */
  #previewBase: Record<string, number> | null = null;

  constructor(doc: EditorDocument, viewport: ViewportController) {
    this.#doc = doc;
    this.#viewport = viewport;
    loadUgoeWasm().then((wasm) => {
      if (wasm) this.#Sampler = wasm.MotionSampler;
    });
  }

  /** 選択中のモーション (ドキュメントから消えていれば null)。 */
  get motion(): Motion | null {
    const name = this.motionName;
    return this.#doc.motions.find((m) => m.name === name) ?? null;
  }

  select(name: string | null): void {
    this.stop();
    this.motionName = name;
    this.selectedKey = null;
    this.time = 0;
    this.engaged = false;
  }

  /**
   * 時刻 `t` での各パラメータのカーブの値 (ID → 値)。wasm がまだ読めていなければ空。
   * パラメータ以外 (パーツの不透明度・エフェクトの強さ) のカーブは含めない。
   */
  sample(t: number, motion: Motion | null = this.motion): Record<string, number> {
    const out: Record<string, number> = {};
    if (!motion) return out;
    const sampler = this.#samplerFor(motion);
    if (!sampler) return out;
    try {
      const values = JSON.parse(sampler.sample(t)) as {
        target: CurveTarget;
        value: number;
      }[];
      for (const { target, value } of values) {
        if (target.kind === "parameter") out[target.id] = value;
      }
    } catch (e) {
      console.warn("ugoe: cannot sample the motion:", e);
    }
    return out;
  }

  /**
   * `motion` を持った MotionSampler。持っているモーションと違えば渡し直す (読めなければ null。前のモーションの索引では評価しない)。
   */
  #samplerFor(motion: Motion): MotionSampler | null {
    if (this.#sampled === motion) return this.#sampler;
    const Sampler = this.#Sampler;
    if (!Sampler) return null;
    this.#sampled = null;
    try {
      const json = JSON.stringify(motion);
      if (this.#sampler) this.#sampler.set_motion_json(json);
      else this.#sampler = new Sampler(json);
      this.#sampled = motion;
      return this.#sampler;
    } catch (e) {
      console.warn("ugoe: cannot sample the motion:", e);
      return null;
    }
  }

  /** 再生位置を `t` にし、その時刻の姿をスライダーに書き込む。 */
  seek(t: number): void {
    const duration = this.motion?.duration ?? 0;
    this.time = roundTime(Math.min(Math.max(t, 0), duration));
    this.engaged = true;
    this.applyPose();
  }

  /** 今の再生位置の姿 (カーブのあるパラメータだけ) をスライダーに書き込む。 */
  applyPose(): void {
    for (const [id, value] of Object.entries(this.sample(this.time))) {
      this.#viewport.setParameter(id, value);
    }
  }

  play(): void {
    const motion = this.motion;
    if (!motion || this.playing) return;
    this.endPreview();
    if (this.time >= motion.duration) this.time = 0;
    this.playing = true;
    this.engaged = true;
    this.#last = performance.now();
    const step = (now: number) => {
      const current = this.motion;
      if (!this.playing || !current) return;
      const dt = (now - this.#last) / 1000;
      this.#last = now;
      const next = advance(this.time, dt, current.duration, this.repeat || current.looping);
      this.time = next.time;
      this.applyPose();
      if (next.ended) {
        this.playing = false;
        this.#frame = 0;
        return;
      }
      this.#frame = requestAnimationFrame(step);
    };
    this.#frame = requestAnimationFrame(step);
  }

  stop(): void {
    this.playing = false;
    cancelAnimationFrame(this.#frame);
    this.#frame = 0;
  }

  toggle(): void {
    if (this.playing) this.stop();
    else this.play();
  }

  selectKey(target: CurveTarget, t: number): void {
    const track = targetKey(target);
    this.engaged = true;
    this.selectedTrack = track;
    this.selectedKey = { track, t };
  }

  // --- 表情のプレビュー ---

  /** 表情を重み 1 で今の値に重ねて見せる。同じ表情なら戻す。 */
  togglePreview(expression: Expression): void {
    if (this.previewExpression === expression.name) {
      this.endPreview();
      return;
    }
    this.endPreview();
    this.stop();
    const base = { ...this.#viewport.parameterValues };
    this.#previewBase = base;
    this.previewExpression = expression.name;
    for (const [id, value] of Object.entries(applyExpression(expression, base))) {
      this.#viewport.setParameter(id, value);
    }
  }

  /** 表情の内容が変わったらプレビューを作り直す。 */
  refreshPreview(expression: Expression | null): void {
    const base = this.#previewBase;
    if (!base || !expression || expression.name !== this.previewExpression) return;
    for (const [id, value] of Object.entries(base)) this.#viewport.setParameter(id, value);
    for (const [id, value] of Object.entries(applyExpression(expression, base))) {
      this.#viewport.setParameter(id, value);
    }
  }

  /** プレビューをやめ、重ねる前の値に戻す。 */
  endPreview(): void {
    const base = this.#previewBase;
    this.#previewBase = null;
    this.previewExpression = null;
    if (!base) return;
    for (const [id, value] of Object.entries(base)) this.#viewport.setParameter(id, value);
  }

  /** プレビュー中なら、重ねる前の値 (表情を作るときの基準)。 */
  get previewBase(): Record<string, number> | null {
    return this.#previewBase;
  }

  dispose(): void {
    this.stop();
    this.#sampler?.free();
    this.#sampler = null;
    this.#sampled = null;
  }
}
