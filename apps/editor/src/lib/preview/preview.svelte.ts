// プレビューモードの状態と WebPuppet の操作 (docs/editor.md#プレビューモード)。Undo の対象ではなく、コマンドも送らない。
//
// - ViewportController の FrameDriver として毎フレーム `update(dt)` を回す (物理・目パチ・呼吸・リップシンク・ポーズ、
//   モーション・表情。適用順は docs/spec/motion.md#フレーム内の適用順)。
// - スライダーの値 (`values`) は編集用の値 (ViewportController.parameterValues) と別に持つ。入るときに編集用の値を
//   写して始め、出るときは ViewportController がモデルを読み直して編集中の姿に戻す。
// - モーション・表情・物理はドキュメントのもの (EditorDocument) を JSON で渡す。バンドルが作れず JSON で表示している
//   ときも物理が動くよう、WebPuppet に物理が無ければ `set_physics_json` で渡す。
// - フェイストラッキング (T0636) はビューアと同じ部品 (@ugoe/face-tracking) を使う。PreviewMode が TrackingTarget になり、
//   値は `update(dt)` の直前に `set_tracking` で渡す (ビューアの Player と同じ)。プレビューを出るとトラッキングも止まる。
// - 着せ替え (T0649) は DressUp (dressUp.svelte.ts)。モジュールはプレビューの間だけ付ける。
// - モジュールのプロジェクトでは、本体だけのモデル (ipc.getPreviewBaseBundle) を描き、今の編集を書き出したモジュール
//   (ipc.getPreviewModule) を付けて、同じスロットのほかのモジュールと差し替えて比べる (T0651)。今の編集は編集のたびに
//   取り直し、本体だけのモデルは本体の版 (ModuleStatus.base_revision) が描いているものと違うときだけ取る (T0653)。
//   本体だけのモデルのバイト列はバックエンドのセッションが覚えておく (ここでは覚えない。T0654)。
//   書き出せない (検査に通らない) 間は、編集中の (合成した) モデルを描き、自分のスロットにはほかのモジュールを付けない。
// - ポーズ (T0660) は、編集中のモデルのポーズのグループごとに表示する要素を選べる (`show_pose`。ポーズの段が有効な間だけ
//   見える)。入るときは編集中のビューポートで見ていた要素 (ViewportController.posePreview) から始め、読み直すと渡し直す。
//   着せ替えで付けたモジュールのグループ (T0661) は WebPuppet の合成したポーズ (`pose_json`) から読み、付け外しと
//   読み込みのたびに読み直す (`modulePoseGroups`)。選んだ要素は WebPuppet の ID で `modulePoseShown` に持つ。
import {
  FaceTracker,
  setPuppetTracking,
  type TrackingFrame,
  type TrackingTarget,
} from "@ugoe/face-tracking";
import type { EditorDocument } from "../document.svelte";
import { type Expression, ipc, type Model, type Motion, type PoseGroup } from "../ipc";
import { showEntry } from "../panels/poses/poses";
import type { FrameDriver, PuppetStage, ViewportController } from "../viewport/controller.svelte";
import type { WebPuppet } from "../viewport/wasm";
import { editingName } from "./dressUp";
import { DressUp } from "./dressUp.svelte";
import { describeLoadWarnings, parseLoadWarnings } from "./loadWarnings";
import {
  defaultStages,
  type ModulePoseGroup,
  modulePoseGroups,
  previewPart,
  pushEvents,
} from "./preview";
import { saveTrackingRecording } from "./trackingHost";

/** 表示の値を書き換える差の下限 (毎フレームの細かな変化で画面を作り直さないため)。 */
const EPSILON = 1e-6;

export class PreviewMode implements FrameDriver, TrackingTarget {
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
  /** 直前の読み込みの警告 (テクスチャを白で代用したなど。読み込むたびに置き換える。T0125)。 */
  warnings = $state<string[]>([]);
  /** フェイストラッキング (入力・キャリブレーション・平滑化・記録)。 */
  readonly tracker: FaceTracker;
  /** トラッキングが書き込むパラメータ (モデルの定義順。`avatar.tracking` の上書きを反映する)。 */
  trackingParameterIds = $state<string[]>([]);
  /** トラッキングの値を渡している (`setTracking` に `null` 以外を渡した後)。 */
  tracking = $state(false);
  /**
   * モデルの自動の目パチを許すか (トラッキング中は止められる。ビューアと同じ)。目パチの段 (`stages.eye_blink`)
   * とどちらも有効なときだけ動く。
   */
  autoEyeBlink = $state(true);
  /** 着せ替え (`.ugoemod` の付け外し)。 */
  readonly dressUp: DressUp;
  /**
   * ポーズで表示する要素のパーツ (編集中のモデルの ID。グループに 1 つまで)。挙げていないグループは最初の要素
   * (ポーズの初期状態)。
   */
  poseShown = $state.raw<readonly string[]>([]);
  /** 着せ替えで付けたモジュールのポーズのグループ (WebPuppet の ID。T0661)。 */
  modulePoses = $state.raw<readonly ModulePoseGroup[]>([]);
  /** モジュールのグループで表示する要素のパーツ (WebPuppet の ID。グループに 1 つまで)。 */
  modulePoseShown = $state.raw<readonly string[]>([]);

  #doc: EditorDocument;
  #viewport: ViewportController;
  /** 次の `update` の前に渡す値。 */
  #pendingTracking: TrackingFrame | null = null;
  /** 最後に受け取った値 (モデルを読み直したときに渡し直す)。 */
  #lastTracking: TrackingFrame | null = null;
  /**
   * モジュールのプロジェクトで、本体だけのモデルに今の編集を付けて描いているか (自分のスロットの行も出し、
   * ほかのモジュールと差し替えられる)。
   */
  comparing = $state(false);
  /** 今の編集を取り直した EditorDocument.revision。 */
  #ownRevision = -1;
  /** 描いている本体だけのモデルの、本体の版 (比べていなければ `null`)。 */
  #baseRevision: number | null = null;

  constructor(doc: EditorDocument, viewport: ViewportController) {
    this.#doc = doc;
    this.#viewport = viewport;
    this.tracker = new FaceTracker(this, { saveJson: saveTrackingRecording });
    this.dressUp = new DressUp({
      puppet: () => this.#viewport.puppet,
      active: () => this.active,
      changed: (puppet) => {
        this.trackingParameterIds = puppet.tracking_parameter_ids();
        this.#readPose(puppet);
        this.#sync(puppet);
      },
      ownSlot: () => (this.comparing ? null : (this.#doc.module?.slot.id ?? null)),
    });
    // ヘッドレスのブラウザでの確認のため (記録した値の列の再生など)。
    window.__UGOE_TRACKER__ = this.tracker;
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
    this.poseShown = this.#viewport.posePreview ?? [];
    this.modulePoses = [];
    this.modulePoseShown = [];
    this.active = true;
    this.#viewport.setDriver(this);
    void this.dressUp.refresh();
    if (this.#doc.module) void this.#compareOwn();
  }

  /** プレビューモードを出て、編集中の姿に戻す。 */
  exit(): void {
    if (!this.active) return;
    this.tracker.stop();
    // 編集の画面には本体だけを出す (付けたい組は覚えておき、次に入ったときに付け直す)。
    this.dressUp.takeOff();
    this.dressUp.highlight = null;
    this.active = false;
    // 本体だけのモデルは忘れ、編集中のモデルに戻る (setDriver(null) が読み直す)。
    this.comparing = false;
    this.#ownRevision = -1;
    this.#baseRevision = null;
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
    this.#applyEyeBlink();
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

  // --- ポーズ ---

  /** グループ `group` で `part` (編集中のモデルのパーツ) を表示する。切り替えはポーズのフェードの時間で行う。 */
  showPose(group: PoseGroup, part: string): void {
    this.poseShown = showEntry(this.poseShown, group, part);
    this.#run((puppet) => this.#applyPose(puppet));
  }

  /** モジュールのグループ `group` で `part` (WebPuppet の ID) を表示する (T0661)。 */
  showModulePose(group: PoseGroup, part: string): void {
    this.modulePoseShown = showEntry(this.modulePoseShown, group, part);
    this.#run((puppet) => puppet.show_pose(part));
  }

  /** 今の編集を別の名前で付けて比べているなら、モジュールの名前と付けた名前 (`previewPart` に渡す)。 */
  #ownModule(): { name: string; editing: string } | null {
    const module = this.#doc.module;
    return this.comparing && module
      ? { name: module.name, editing: editingName(module.name) }
      : null;
  }

  #applyPose(puppet: WebPuppet): void {
    const own = this.#ownModule();
    for (const part of this.poseShown) puppet.show_pose(previewPart(part, own));
    // 外したモジュールのパーツは show_pose が false を返すだけ (付け直すと選んでいた要素に戻る)。
    for (const part of this.modulePoseShown) puppet.show_pose(part);
  }

  /** WebPuppet の合成したポーズから、付けたモジュールのグループを読み直す。 */
  #readPose(puppet: WebPuppet): void {
    try {
      const json = this.#doc.model?.json;
      const own = json ? (JSON.parse(json) as Model).pose.groups : [];
      this.modulePoses = modulePoseGroups(puppet.pose_json(), own, this.#ownModule());
    } catch (e) {
      console.warn("ugoe:", e);
      this.modulePoses = [];
    }
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

  // --- TrackingTarget (FaceTracker が呼ぶ) ---

  /**
   * フェイストラッキングの値を設定する (次のフレームの `update` の前に Rust の対応付けに渡す)。`null` で止め、
   * トラッキングが書き込んでいたパラメータを既定値に戻す (最後の値が基準値として残るため)。
   */
  setTracking(frame: TrackingFrame | null): void {
    if (frame !== null) {
      this.#lastTracking = frame;
      this.#pendingTracking = frame;
      this.tracking = true;
      return;
    }
    if (this.#lastTracking === null) return;
    this.#lastTracking = null;
    this.#pendingTracking = null;
    this.tracking = false;
    const puppet = this.#puppet();
    if (!puppet) return;
    puppet.clear_tracking();
    const ids = new Set(this.trackingParameterIds);
    for (const def of this.#doc.parameters) {
      if (ids.has(def.id)) puppet.set_parameter(def.id, def.default);
    }
    this.#sync(puppet);
  }

  setAutoEyeBlink(enabled: boolean): void {
    this.autoEyeBlink = enabled;
    this.#applyEyeBlink();
  }

  // --- FrameDriver (ViewportController が呼ぶ) ---

  loaded(puppet: WebPuppet): void {
    const physics = this.#doc.physics;
    let physicsReplaced = false;
    // 本体だけのモデルには、本体の物理がバンドルに入っている (ドキュメントの物理はモジュールのものを含む)。
    if (physics && !puppet.has_physics() && !this.comparing) {
      try {
        puppet.set_physics_json(JSON.stringify(physics));
        physicsReplaced = true;
      } catch (e) {
        this.error = `Physics: ${e}`;
      }
    }
    this.warnings = this.#loadWarnings(puppet, physicsReplaced);
    // 物理を渡した後に付ける (付けるときの本体の物理が合成の元になる)。
    this.dressUp.loaded(puppet);
    for (const [id, value] of Object.entries(this.values)) puppet.set_parameter(id, value);
    // 読み直すと再生の状態は初めからになるので、表情だけ掛け直す (モーションは止まる)。
    const expression = this.expression === null ? null : this.#expression(this.expression);
    if (expression) puppet.set_expression_json(JSON.stringify(expression));
    else this.expression = null;
    this.motionPlaying = false;
    this.#applyLipSync(puppet);
    this.#applyEyeBlink(puppet);
    try {
      this.#readPose(puppet);
      this.#applyPose(puppet);
    } catch (e) {
      console.warn("ugoe:", e);
    }
    this.trackingParameterIds = puppet.tracking_parameter_ids();
    // 読み直すとトラッキングの入力も消えるので、最後の値を渡し直す。
    this.#pendingTracking = this.#lastTracking;
    this.#sync(puppet);
    // 編集で読み直したなら、今の編集のモジュールを取り直す。
    if (this.comparing && this.#doc.revision !== this.#ownRevision) void this.#compareOwn();
  }

  /** 直前の読み込みの警告 (`load_warnings_json`) を知らせの文にする。 */
  #loadWarnings(puppet: WebPuppet, physicsReplaced: boolean): string[] {
    try {
      return describeLoadWarnings(parseLoadWarnings(puppet.load_warnings_json()), physicsReplaced);
    } catch (e) {
      console.warn("ugoe:", e);
      return [];
    }
  }

  beforeUpdate(puppet: WebPuppet): void {
    const frame = this.#pendingTracking;
    if (frame === null) return;
    this.#pendingTracking = null;
    setPuppetTracking(puppet, frame);
  }

  afterUpdate(puppet: WebPuppet): void {
    this.#sync(puppet);
    const playing = puppet.is_motion_playing();
    if (playing !== this.motionPlaying) this.motionPlaying = playing;
    const fired = puppet.motion_events();
    if (fired.length > 0) this.events = pushEvents(this.events, fired);
  }

  // --- 内部 ---

  /**
   * モジュールのプロジェクトで、本体だけのモデルと今の編集のモジュールを取り、本体に今の編集を付けて描く。
   * 本体だけのモデルは、描いているものと本体の版が違うときだけ取る (作り直すかはバックエンドが決める)。
   * 書き出せなければ理由を出し、編集中の (合成した) モデルに戻す。取る間に編集されたら、その読み込みで取り直す。
   */
  async #compareOwn(): Promise<void> {
    const module = this.#doc.module;
    if (!module) return;
    const revision = this.#doc.revision;
    this.#ownRevision = revision;
    const baseRevision = module.base_revision;
    const drawn = this.comparing && this.#baseRevision === baseRevision;
    let base: Uint8Array | null;
    let own: Uint8Array;
    try {
      [base, own] = await Promise.all([
        drawn ? null : ipc.getPreviewBaseBundle(),
        ipc.getPreviewModule(editingName(module.name)),
      ]);
      // 取る間に比べるのをやめていたら、描き直すので本体も取る。
      if (base === null && !this.comparing) base = await ipc.getPreviewBaseBundle();
    } catch (e) {
      if (!this.active || revision !== this.#doc.revision) return;
      this.dressUp.ownProblem = `Your edits can't be put on the base to compare with other modules: ${e}`;
      this.#stopComparing();
      return;
    }
    if (!this.active || revision !== this.#doc.revision) return;
    this.dressUp.ownProblem = null;
    const comparing = this.comparing;
    this.comparing = true;
    // 描いている本体だけのモデルに付けるなら、ここで読み込み直す。初めてなら、本体を読み込んだ後の loaded で読み込む。
    this.dressUp.setOwn(own, editingName(module.name), comparing, module.name);
    if (base !== null) {
      this.#baseRevision = baseRevision;
      this.#viewport.setPreviewModel({ json: this.#doc.model?.json ?? "", bundle: base });
    }
  }

  /** 今の編集を付けて比べるのをやめ、編集中の (合成した) モデルを描く。 */
  #stopComparing(): void {
    const comparing = this.comparing;
    this.comparing = false;
    this.#baseRevision = null;
    this.dressUp.setOwn(null, "", false);
    if (comparing) this.#viewport.setPreviewModel(null);
  }

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

  /** 目パチの段は、段の設定とトラッキングの指定のどちらも有効なときだけ動かす。 */
  #applyEyeBlink(puppet: WebPuppet | null = this.#puppet()): void {
    if (!puppet) return;
    try {
      puppet.set_enabled("eye_blink", this.stages.eye_blink && this.autoEyeBlink);
    } catch (e) {
      console.warn("ugoe:", e);
    }
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
