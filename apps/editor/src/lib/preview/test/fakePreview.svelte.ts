// プレビューの部品 (PreviewPanel・DifferenceOverlay) のテスト (docs/testing.md#エディタの部品のテスト) で使う
// PreviewMode の偽物。部品が読む状態だけを持ち、操作は `calls` に記録する (WebPuppet には触らない)。
// トラッカー (@ugoe/face-tracking) と着せ替え (DressUp) は本物を使う。@ugoe/face-tracking の読み込みは重いので、
// ビューポートのツールの道具 (harness.svelte.ts) とは別のファイルに置く。
import { FaceTracker, type TrackingFrame, type TrackingTarget } from "@ugoe/face-tracking";
import type { PoseGroup } from "../../ipc";
import type { PuppetStage, ViewportController } from "../../viewport/controller.svelte";
import { DressUp } from "../dressUp.svelte";
import { defaultStages, type ModulePoseGroup } from "../preview";
import type { PreviewMode } from "../preview.svelte";

export class FakePreview implements TrackingTarget {
  active = $state(true);
  values = $state<Record<string, number>>({});
  stages = $state<Record<PuppetStage, boolean>>(defaultStages());
  lipSyncLevel = $state(0.5);
  motion = $state<string | null>(null);
  motionPlaying = $state(false);
  expression = $state<string | null>(null);
  events = $state<string[]>([]);
  error = $state<string | null>(null);
  warnings = $state<string[]>([]);
  trackingParameterIds = $state<string[]>([]);
  comparing = $state(false);
  poseShown = $state.raw<readonly string[]>([]);
  modulePoses = $state.raw<readonly ModulePoseGroup[]>([]);
  modulePoseShown = $state.raw<readonly string[]>([]);
  readonly tracker: FaceTracker;
  readonly dressUp: DressUp;
  /** 部品が呼んだ操作 (名前)。 */
  readonly calls: string[] = [];

  /** 着せ替えは `viewport` の WebPuppet に付け外しする (テストでは付けない)。 */
  constructor(viewport: ViewportController) {
    // 動かさないので、繰り返しの仕組みは何もしない。
    this.tracker = new FaceTracker(this, { schedule: () => () => {} });
    this.dressUp = new DressUp({
      puppet: () => viewport.puppet,
      active: () => this.active,
      changed: () => this.calls.push("changed"),
    });
  }

  setTracking(_frame: TrackingFrame | null): void {
    this.calls.push("setTracking");
  }
  setAutoEyeBlink(_enabled: boolean): void {
    this.calls.push("setAutoEyeBlink");
  }
  setParameter(_id: string, _value: number): void {
    this.calls.push("setParameter");
  }
  resetAllParameters(): void {
    this.calls.push("resetAllParameters");
  }
  setStage(_stage: PuppetStage, _enabled: boolean): void {
    this.calls.push("setStage");
  }
  setLipSyncLevel(_level: number): void {
    this.calls.push("setLipSyncLevel");
  }
  resetPhysics(): void {
    this.calls.push("resetPhysics");
  }
  showPose(_group: PoseGroup, part: string): void {
    this.calls.push(`showPose:${part}`);
  }
  showModulePose(_group: PoseGroup, part: string): void {
    this.calls.push(`showModulePose:${part}`);
  }
  playMotion(): void {
    this.calls.push("playMotion");
  }
  stopMotions(): void {
    this.calls.push("stopMotions");
  }
  setExpression(_name: string | null): void {
    this.calls.push("setExpression");
  }

  get asPreview(): PreviewMode {
    return this as unknown as PreviewMode;
  }
}
