// プレビューモードの状態と WebPuppet の操作 (docs/editor.md#プレビューモード)。Undo の対象ではなく、コマンドも送らない。
//
// - ViewportController の FrameDriver として毎フレーム `update(dt)` を回す (物理・目パチ・呼吸・リップシンク・ポーズ、
//   モーション・表情。適用順は docs/spec/motion.md#フレーム内の適用順)。
// - スライダーの値 (`values`) は編集用の値 (ViewportController.parameterValues) と別に持つ。入るときに編集用の値を
//   写して始め、出るときは ViewportController がモデルを読み直して編集中の姿に戻す。
// - モーション・表情・物理はドキュメントのもの (EditorDocument) を JSON で渡す。バンドルが作れず JSON で表示している
//   ときも物理が動くよう、WebPuppet に物理が無ければ `set_physics_json` で渡す。
import type { EditorDocument } from "../document.svelte";
import type { Expression, Motion } from "../ipc";
import type { FrameDriver, PuppetStage, ViewportController } from "../viewport/controller.svelte";
import type { WebPuppet } from "../viewport/wasm";
import { defaultStages, pushEvents } from "./preview";

/** 表示の値を書き換える差の下限 (毎フレームの細かな変化で画面を作り直さないため)。 */
const EPSILON = 1e-6;

export class PreviewMode implements FrameDriver {
  /** プレビューモード中か。 */
  active = $state(false);
  /** プレビューのスライダーの値 (ID → 値)。毎フレーム、物理・呼吸などを掛けた後の値に更新する。 */
  values = $state<Record<string, number>>({});
  /** 段の有効・無効 (`set_enabled`)。プレビューを出入りしても保つ。 */
  stages = $state<Record<PuppetStage, boolean>>(defaultStages());
  /** リップシンクの音量 (0〜1)。リップシンクの段が有効な間だけ渡す。 */
  lipSyncLevel = $state(0.5);
  /** 最後に再生したモーションの名前。 */
  motion = $state<string | null>(null);
  /** モーションを再生中か (フェードアウト中も含む)。 */
  motionPlaying = $state(false);
  /** 使用中の表情の名前。 */
  expression = $state<string | null>(null);
  /** 最近発火したモーションのイベント (新しい順)。 */
  events = $state<string[]>([]);
  /** 直近の失敗 (物理の設定がモデルと合わないなど)。 */
  error = $state<string | null>(null);

  #doc: EditorDocument;
  #viewport: ViewportController;

  constructor(doc: EditorDocument, viewport: ViewportController) {
    this.#doc = doc;
    this.#viewport = viewport;
  }

  /** プレビューモードに入る。スライダーは編集中の姿の値から始める。 */
  enter(): void {
    if (this.active) return;
    this.values = { ...this.#viewport.parameterValues };
    this.motion = null;
    this.motionPlaying = false;
    this.expression = null;
    this.events = [];
    this.error = null;
    this.active = true;
    this.#viewport.setDriver(this);
  }

  /** プレビューモードを出て、編集中の姿に戻す。 */
  exit(): void {
    if (!this.active) return;
    this.active = false;
    this.#viewport.setDriver(null);
  }

  toggle(): void {
    if (this.active) this.exit();
    else this.enter();
  }

  // --- パラメータ ---

  setParameter(id: string, value: number): void {
    this.values[id] = value;
    const puppet = this.#puppet();
    if (puppet?.set_parameter(id, value)) this.values[id] = puppet.parameter(id) ?? value;
  }

  resetParameter(id: string): void {
    const def = this.#doc.parameters.find((p) => p.id === id);
    if (def) this.setParameter(id, def.default);
  }

  resetAllParameters(): void {
    const puppet = this.#puppet();
    if (!puppet) return;
    puppet.reset_parameters();
    this.#sync(puppet);
  }

  // --- 段 ---

  setStage(stage: PuppetStage, enabled: boolean): void {
    this.stages[stage] = enabled;
    this.#viewport.applyStages();
    if (stage === "lip_sync") this.#applyLipSync();
  }

  setLipSyncLevel(level: number): void {
    this.lipSyncLevel = Math.min(Math.max(level, 0), 1);
    this.#applyLipSync();
  }

  /** 粒子を今の入力の静止姿勢に置き直す。 */
  resetPhysics(): void {
    this.#run((puppet) => puppet.reset_physics());
  }

  // --- モーション・表情 ---

  /** モーションを最優先 (`force`) で始める。再生中のものはフェードアウトする。 */
  playMotion(motion: Motion): void {
    this.#run((puppet) => {
      puppet.start_motion_json(JSON.stringify(motion), "force");
      this.motion = motion.name;
      this.motionPlaying = puppet.is_motion_playing();
    });
  }

  stopMotions(): void {
    this.#run((puppet) => puppet.stop_motions());
  }

  /** 表情をこれだけにする。`null` で外す (どちらもフェードする)。 */
  setExpression(name: string | null): void {
    this.#run((puppet) => {
      const expression = name === null ? null : this.#expression(name);
      if (expression) puppet.set_expression_json(JSON.stringify(expression));
      else puppet.stop_expressions();
      this.expression = expression ? expression.name : null;
    });
  }

  // --- FrameDriver (ViewportController が呼ぶ) ---

  loaded(puppet: WebPuppet): void {
    const physics = this.#doc.physics;
    if (physics && !puppet.has_physics()) {
      try {
        puppet.set_physics_json(JSON.stringify(physics));
      } catch (e) {
        this.error = `Physics: ${e}`;
      }
    }
    for (const [id, value] of Object.entries(this.values)) puppet.set_parameter(id, value);
    // 読み直すと再生の状態は初めからになるので、表情だけ掛け直す (モーションは止まる)。
    const expression = this.expression === null ? null : this.#expression(this.expression);
    if (expression) puppet.set_expression_json(JSON.stringify(expression));
    else this.expression = null;
    this.motionPlaying = false;
    this.#applyLipSync(puppet);
    this.#sync(puppet);
  }

  afterUpdate(puppet: WebPuppet): void {
    this.#sync(puppet);
    const playing = puppet.is_motion_playing();
    if (playing !== this.motionPlaying) this.motionPlaying = playing;
    const fired = puppet.motion_events();
    if (fired.length > 0) this.events = pushEvents(this.events, fired);
  }

  // --- 内部 ---

  #puppet(): WebPuppet | null {
    const puppet = this.#viewport.puppet;
    return this.active && puppet?.has_model() ? puppet : null;
  }

  #expression(name: string): Expression | null {
    return this.#doc.expressions.find((e) => e.name === name) ?? null;
  }

  #run(action: (puppet: WebPuppet) => void): void {
    const puppet = this.#puppet();
    if (!puppet) return;
    try {
      action(puppet);
      this.error = null;
    } catch (e) {
      this.error = String(e);
    }
  }

  #applyLipSync(puppet: WebPuppet | null = this.#puppet()): void {
    puppet?.set_lip_sync_level(this.stages.lip_sync ? this.lipSyncLevel : undefined);
  }

  /** WebPuppet の値をスライダーの表示へ写す (変わったものだけ)。 */
  #sync(puppet: WebPuppet): void {
    for (const { id } of this.#doc.parameters) {
      const value = puppet.parameter(id);
      if (value === undefined) continue;
      const shown = this.values[id];
      if (shown === undefined || Math.abs(shown - value) > EPSILON) this.values[id] = value;
    }
  }
}
