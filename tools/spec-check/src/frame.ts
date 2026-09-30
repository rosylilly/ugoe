// 1 フレームの順 (spec/motion.md#order-of-operations-in-a-frame): モーション・表情・効果・物理・
// ポーズ・追従 (トラッキング) を決まった順に行い、評価に渡すパラメータの値と上書きとオフセットを作る。

import {
  type EvaluationResult,
  emptyOffsets,
  evaluate,
  evaluateDeformers,
  type Offsets,
  type ParameterValues,
  type PartOverrides,
  parameterValues,
} from "./evaluate.ts";
import {
  Breath,
  combineRequests,
  ExpressionManager,
  EyeBlink,
  MotionManager,
  PoseState,
  type Priority,
  writeParameter,
} from "./motion.ts";
import { Physics } from "./physics.ts";
import { applyTracking, type TrackingInput } from "./tracking.ts";
import type * as M from "./types.ts";

/** ホストが切れる段 (spec/motion.md#order-of-operations-in-a-frame の Consequences)。 */
export type Stage = "physics" | "eye_blink" | "breath" | "lip_sync" | "pose";

export interface FrameResult {
  /** 評価に渡したパラメータの値 (手順 10 で覚える値)。 */
  values: ParameterValues;
  overrides: PartOverrides;
  offsets: Offsets;
  events: string[];
  evaluation: EvaluationResult;
}

export class Runtime {
  readonly model: M.Model;
  readonly values: ParameterValues;
  readonly motions = new MotionManager();
  readonly expressions = new ExpressionManager();
  /** 効果の設定 (はじめはモデルのもの。ホストが差し替えられる)。 */
  effects: M.Effects;
  eyeBlink: EyeBlink;
  breath: Breath;
  readonly pose: PoseState;
  readonly physics: Physics | null;
  readonly enabled: Record<Stage, boolean> = {
    physics: true,
    eye_blink: true,
    breath: true,
    lip_sync: true,
    pose: true,
  };
  /** リップシンクの音量 (無ければ null)。 */
  lipSyncLevel: number | null = null;
  /** 毎フレーム使う追従の入力 (無ければ null)。 */
  tracking: TrackingInput | null = null;
  private base: ParameterValues | null = null;
  private left: ParameterValues | null = null;

  constructor(model: M.Model, physics: M.PhysicsSettings | null = null) {
    this.model = model;
    this.values = parameterValues(model);
    this.effects = model.effects;
    this.eyeBlink = new EyeBlink(model.effects.eye_blink);
    this.breath = new Breath(model.effects.breath.parameters);
    this.pose = new PoseState(model.pose);
    this.physics = physics ? new Physics(model, physics) : null;
  }

  /** ホストがパラメータを書く (範囲に切り詰める。spec/evaluation.md#parameter-values)。 */
  set(id: string, value: number): void {
    if (!Number.isFinite(value)) return;
    writeParameter(this.model, this.values, id, () => value);
  }

  /**
   * 効果の設定を差し替える (spec/motion.md#replacing-the-effect-settings)。目パチは新しい seed で
   * 始め直し (開いた相の始めから、最初の間隔を引く)、呼吸は各波の位相を 0 から始め直す。
   */
  setEffects(effects: M.Effects): void {
    this.effects = effects;
    this.eyeBlink = new EyeBlink(effects.eye_blink);
    this.breath = new Breath(effects.breath.parameters);
  }

  /** ポーズのエントリを表示する (spec/motion.md#pose)。`part` はエントリの `part`。 */
  showPose(part: string): void {
    this.pose.show(part);
  }

  startMotion(motion: M.Motion, priority: Priority): boolean {
    return this.motions.start(motion, priority);
  }

  setStage(stage: Stage, on: boolean): void {
    if (stage === "physics" && this.enabled.physics !== on) {
      // 切ればオフセットを消し、入れ直せば静止の姿勢から始める (リセット)
      this.physics?.reset();
    }
    this.enabled[stage] = on;
  }

  /** 1 フレーム進める。 */
  frame(dt: number): FrameResult {
    const model = this.model;
    const values = this.values;

    // 1. 基準の値に戻す
    if (this.base === null || this.left === null) {
      this.base = new Map(values);
    } else {
      for (const [id, v] of values) {
        if (!Object.is(v, this.left.get(id))) this.base.set(id, v);
      }
      for (const [id, v] of this.base) values.set(id, v);
    }

    // 2. モーション
    const motion = this.motions.frame(model, values, dt);

    // 3. 追従
    if (this.tracking) applyTracking(model, values, this.tracking);

    // 4. 基準の値を保存する
    this.base = new Map(values);

    // 5〜7. 表情・目パチ・呼吸
    this.expressions.frame(model, values, dt);
    if (this.enabled.eye_blink) {
      this.eyeBlink.advance(dt);
      this.eyeBlink.apply(model, values, motion.strength.EyeBlink);
    }
    if (this.enabled.breath) {
      this.breath.advance(dt);
      this.breath.apply(model, values);
    }

    // 8. 物理 (動的な物理なら評価の 1 段目)
    let offsets = emptyOffsets();
    if (this.physics && this.enabled.physics) {
      const deformers = this.physics.usesDynamic ? evaluateDeformers(model, values) : null;
      this.physics.update(dt, values, deformers);
      offsets = this.physics.offsets;
    }

    // 9. リップシンク
    if (this.enabled.lip_sync && this.lipSyncLevel !== null) {
      const level = Math.min(Math.max(this.lipSyncLevel, 0), 1);
      const s = motion.strength.LipSync;
      for (const id of this.effects.lip_sync.parameters) {
        writeParameter(model, values, id, (v) => v + (level - v) * s);
      }
    }

    // 10. このフレームが残した値
    this.left = new Map(values);

    // 11. パーツの不透明度の上書き (spec/motion.md#part-opacity)
    const overrides: PartOverrides = new Map();
    const poseActive = this.enabled.pose && model.pose.groups.length > 0;
    const poseParts = poseActive ? this.pose.entryParts() : new Set<string>();
    const poseRequests = new Map<string, number>();
    for (const [part, requests] of motion.requests) {
      const o = combineRequests(requests);
      if (poseParts.has(part)) poseRequests.set(part, o.value * o.weight);
      else overrides.set(part, o);
    }
    if (poseActive) this.pose.frame(poseRequests, dt, overrides);

    // 12. 評価 (動的な物理なら 2 段目)
    const evaluation = evaluate(model, values, { offsets, partOverrides: overrides });
    return { values: new Map(values), overrides, offsets, events: motion.events, evaluation };
  }
}
