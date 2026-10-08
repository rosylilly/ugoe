// モデル 1 体の表示と、フレームのループ。WebPuppet (ugoe-wasm) を持ち、UI に状態を公開する。
//
// フレームごとに: ドライバ (drivers.ts) の値を設定 → トラッキングの入力を渡す → `update(dt)` (物理など)
// → スライダーの値を同期 → 描画。
// 描画は WebGPU が無ければ WebGL2 で行う (docs/adr/0003-editor-viewport-in-webview.md)。WebPuppet と canvas の寿命は surface.ts。
// wasm の読み込みとファイルの読み直しはアプリが `PlayerHost` で渡す。
import { setPuppetTracking, type TrackingFrame } from "@ugoe/face-tracking";
import {
  describeRejectedMotion,
  type ExpressionInfo,
  type IdleChoice,
  idleMemory,
  initialIdleMotion,
  loadTrackingKept,
  type MotionEventInfo,
  type MotionInfo,
  type MotionPriority,
  motionOverTrackingActive,
  parseExpressions,
  parseMotionEvents,
  parseMotions,
  saveTrackingKept,
} from "./animation";
import { parseHexColor } from "./color";
import {
  describeDressUpError,
  describeLoadWarnings,
  type ModuleInfo,
  parseAvatarSlots,
  parseModules,
  parseSlots,
  type SlotInfo,
} from "./dressUp";
import {
  type DressUpSet,
  describeRestoreFailure,
  dressUpMemory,
  loadAutoRestore,
  needsRestore,
  planRestore,
  type RememberedModule,
  sameAttached,
  saveAutoRestore,
} from "./dressUpMemory";
import { DriverSet } from "./drivers";
import { applyFrame, type FollowFrame, type FollowRenderStats, FrameQueue } from "./follower";
import { t } from "./i18n";
import {
  defaultLighting,
  type LightingSettings,
  lightingMemory,
  loadLighting,
  type ScreenTransform,
  saveLighting,
  toLightingJson,
} from "./lighting";
import { describeCreateError, describeLoadError, isCreateFailure } from "./loadError";
import { parseLoadWarnings } from "./loadWarnings";
import { detectModelKind, type FileLocation, type ModelFile, type ModuleToOpen } from "./modelFile";
import { type ModelIdentity, type ModelMemory, modelIdentity, sameModel } from "./modelMemory";
import {
  groupParameters,
  type ParameterDef,
  type ParameterSection,
  parseParameterList,
} from "./parameters";
import { carryPoseShown, followPoseShown, type PoseGroupInfo, parsePose } from "./pose";
import { mergeRemembered, type RememberedModel, type RememberedOrder } from "./rememberedModels";
import type { Screenshot } from "./screenshot";
import { PlayerStorage, STORAGE_NAMES } from "./storage";
import { GpuLostRepeatedlyError, type PowerPreference, PuppetSurface } from "./surface";
import type { UgoeWasmModule, WebPuppet } from "./wasm";

/** プレーヤーを使うアプリが渡すもの (wasm の読み込みとファイルの読み直し)。 */
export interface PlayerHost {
  /** ugoe-wasm を初期化して返す (2 回目以降は同じもの)。生成物が無ければ `null`。 */
  loadWasm(): Promise<UgoeWasmModule | null>;
  /** 覚えた場所 (modelFile.ts の `FileLocation`) からファイルを読み直す (着せ替えの組を戻すとき)。 */
  readFile(location: FileLocation): Promise<ModelFile>;
  /** `loadWasm` が `null` を返したときに `error` に出す文。 */
  wasmMissingMessage?: string;
  /**
   * 覚えるもの (光源・着せ替えの組・待機モーションなど) の localStorage のキーの接頭辞 (storage.ts。T0930)。
   * 省くとビューアの `ugoe.viewer` (`DEFAULT_STORAGE_PREFIX`)。スタジオは `ugoe.studio.player`。
   */
  storagePrefix?: string;
  /** 覚える先。省くと `localStorage` (テストで差し替える)。 */
  storage?: Storage;
}

export type PlayerStatus =
  /** wasm と GPU を用意している。 */
  | "starting"
  /** モデルを開いていない。 */
  | "empty"
  | "ready"
  /** wasm の生成物が無い、または描画を始められない。 */
  | "unavailable";

/** 1 フレームで進める時間の上限 (秒)。タブが裏にあった後などに物理が飛ばないように。 */
const MAX_DT = 0.1;
/** スライダーの値を書き換える差の下限。 */
const EPSILON = 1e-6;
/** モーションのイベントを覚えておく数 (新しいものから)。 */
const MOTION_EVENTS = 5;
/** このセッションで読んだモジュールの中身を手元に置く数 (一覧から除いた後でも組を戻せるように)。 */
const SESSION_FILES = 32;

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** WebPuppet を作れなかった理由の文 (`CreateError` は訳した文、ほかは例外の文)。 */
function createFailureText(e: unknown): string {
  return describeCreateError(e) ?? t("errors.renderStart", { message: message(e) });
}

/**
 * `PuppetSurface` が描画を始められなかった・作り直しを諦めたときの文。作り直しの中で作れなかったことがあれば
 * (`GpuLostRepeatedlyError` の `cause`)、その理由の文に作り直しを諦めたことを添える (T0946)。
 */
function surfaceFailureText(e: unknown): string {
  if (!(e instanceof GpuLostRepeatedlyError)) return createFailureText(e);
  if (e.cause === undefined) return t("errors.gpuLostRepeatedly");
  return t("errors.recreateFailed", { reason: createFailureText(e.cause) });
}

export class Player {
  status = $state<PlayerStatus>("starting");
  /** 直近のエラー (読み込みや描画の失敗)。 */
  error = $state<string | null>(null);
  /** 読み込めたが注意が要ること (テクスチャの代用など)。 */
  warnings = $state<string[]>([]);
  modelName = $state<string | null>(null);
  /** 描画に使っている API (`webgpu` / `webgl2`)。 */
  backend = $state<string | null>(null);
  /**
   * 使っている GPU の名前 (`WebPuppet.adapter_name`。T0499)。ブラウザが伏せると空文字 (空のときに何を出すかは画面が決める)。
   * 作るまでは `null`。
   */
  adapterName = $state<string | null>(null);
  /**
   * GPU を失って、戻るのを待っている (WebGL2 の復元を待つ・作り直しを試し直すのを待つ。surface.ts)。待っている間は描かない。
   * 画面には「GPU の復元を待っています」を出す (T0938)。
   */
  waitingForGpu = $state(false);
  /**
   * GPU を続けて失って描画を止めた・GPU やブラウザの都合 (`CreateError`) で作れなかった。`restart()` で作り直せる
   * (画面には「作り直す」ボタンを出す。T0938・T0941)。
   */
  canRestart = $state(false);
  sections = $state<ParameterSection[]>([]);
  /** パラメータの現在値 (ID → 値)。 */
  values = $state<Record<string, number>>({});
  /** 直前のフレームでドライバかトラッキングが値を設定したパラメータ。スライダーでは動かせない。 */
  driven = $state<ReadonlySet<string>>(new Set());
  /** トラッキングが書き込むパラメータ (モデルの定義順。`avatar.tracking` の上書きを反映する)。 */
  trackingParameterIds = $state<string[]>([]);
  /** トラッキングの入力を渡しているか。 */
  tracking = $state(false);
  /** モデルの自動の目パチ。モデルを開き直しても保つ。 */
  autoEyeBlink = $state(true);
  /** フィットに対する倍率。 */
  zoom = $state(1);
  background = $state("#e6e7eb");
  transparent = $state(false);
  /** 開いているモデルにライティングを受けるメッシュがあるか (光源のパネルを出すか)。 */
  usesLighting = $state(false);
  /**
   * 光源の設定 (lighting.ts)。モデルごとに覚え (T0709)、覚えていないモデルを開いたときは、最後に使った設定を保つ。
   * 最後に使った設定も localStorage に覚える (起動し直したときに、覚えていないモデルで使う)。
   */
  lighting = $state<LightingSettings>(defaultLighting());
  /** 開いているモデルの光源の設定を覚えているか (覚えたものの一覧で忘れる)。 */
  lightingRemembered = $state(false);
  /** ビューポートに光源の印を出すか (T0708)。localStorage に覚える。 */
  lightMarkers = $state(true);
  /** 開いているモデルのキャンバスの大きさ `[幅, 高さ]`。モデルが無ければ `null`。 */
  canvasSize = $state<[number, number] | null>(null);
  /** ビューポート (canvas 要素) の大きさ `[幅, 高さ]` (CSS ピクセル。画面の外の印を端に寄せるため、T0714)。 */
  viewportSize = $state<[number, number]>([1, 1]);
  /** キャンバス座標 → canvas 要素の CSS ピクセルの変換 (ビューポートに重ねる印のため)。モデルが無ければ `null`。 */
  view = $state<ScreenTransform | null>(null);
  /** 読み込んだ着せ替えのモジュール (読み込んだ順)。同じモデルを開き直したときは残り、別のモデルを開くと空になる。 */
  modules = $state<ModuleInfo[]>([]);
  /** 開いているモデル (本体) のスロット (公開していないものも含む)。 */
  slots = $state<SlotInfo[]>([]);
  /**
   * 本体のアバタープロファイルがアプリに公開するスロットの ID (`avatar.slots`)。プロファイルが無ければ `null`
   * (すべて見せる)。着せ替えのパネルは公開していないスロットを出さない (dressUp.ts の `visibleSlots`)。
   */
  publishedSlots = $state<string[] | null>(null);
  /** 直近の着せ替えの失敗 (利用者に見せる文。複数あれば改行で区切る)。 */
  dressUpError = $state<string | null>(null);
  /**
   * 開いたモデルに前に付けていた組 (dressUpMemory.ts)。戻すか尋ねている間だけ入る (`restoreDressUp` か
   * `dismissRestore`、または着せ替えを操作すると空になる)。
   */
  restoreOffer = $state<DressUpSet | null>(null);
  /** モデルを開いたとき、前に付けていた組を尋ねずに戻すか。localStorage に覚える。 */
  autoRestore = $state(false);
  /** 開いているモデルに付けていた組を覚えているか (「覚えた組を忘れる」で忘れる)。 */
  dressUpRemembered = $state(false);
  /** 開いているモデル (バンドル) のモーション (T0720)。 */
  motions = $state<MotionInfo[]>([]);
  /** 開いているモデル (バンドル) の表情 (T0720)。 */
  expressions = $state<ExpressionInfo[]>([]);
  /** 最後に始めて、まだ終わっていない (止めていない) モーション。フェードアウト中のものは含めない。 */
  currentMotion = $state<string | null>(null);
  /** 次に始めるモーションの優先度。 */
  motionPriority = $state<MotionPriority>("normal");
  /** 終わったモーションを同じ優先度で初めから始め直す (ループしないモーションを繰り返し見る)。 */
  repeatMotion = $state(false);
  /** モーションを始められなかったときの説明。 */
  motionNotice = $state<string | null>(null);
  /**
   * 直近に発火したモーションのイベント (新しい順、`MOTION_EVENTS` 個まで)。発火したモーションの名前と番号も持つ
   * (クロスフェード中にどちらのモーションのイベントかを見分ける。T0734)。
   */
  motionEvents = $state<MotionEventInfo[]>([]);
  /** 使っている表情 (開始した順。フェードアウト中のものは含めない)。 */
  activeExpressions = $state<string[]>([]);
  /**
   * 待機モーションを自動で流す (T0721)。モデルを開いたときと、ほかのモーションが終わったとき (止めたときも) に
   * `idleMotion` を待機の優先度で始める。localStorage に覚える。
   */
  autoIdle = $state(true);
  /**
   * 自動で流す待機モーション (開いているモデルのモーションの名前)。選ぶとモデルごとに覚え (T0722)、モデルを開くと
   * 覚えた選択、無ければ既定 (`defaultIdleMotion`) になる (animation.ts の `initialIdleMotion`)。
   */
  idleMotion = $state<string | null>(null);
  /** 開いているモデルの待機モーションの選択を覚えているか (T0722)。 */
  idleRemembered = $state(false);
  /**
   * トラッキング中も、待機以外のモーションが動かすパラメータではモーションを優先する (T0721)。localStorage に覚える。
   * 切ると仕様の適用順のまま (トラッキングがモーションの値を置き換える)。
   */
  motionOverTracking = $state(true);
  /**
   * モーションを優先しても、トラッキングのままにするパラメータの ID (T0722。目は除く、など)。localStorage に覚える
   * (モデルによらない)。WebPuppet の `set_tracking_kept` に渡す。
   */
  trackingKept = $state<string[]>([]);
  /**
   * 開いているモデルのポーズのグループ (WebPuppet の `pose_json`。着せ替えで付けたモジュールのグループも入る。T0735)。
   * モデルを読んだときと、着せ替えの付け外しのたびに読み直す。
   */
  poseGroups = $state<PoseGroupInfo[]>([]);
  /**
   * グループごとに表示している要素のパーツ (`poseGroups` と同じ順)。モデルを読むと先頭の要素になり、付け外しでは要素の並びが
   * 同じグループの選択を保つ (Puppet と同じ規則。pose.ts の `carryPoseShown`)。モーションがパーツの不透明度のカーブで
   * 要素を切り替えると、毎フレーム読む表示中の要素 (`pose_visible_json`) に追従する (T0736。`followPoseShown`)。
   */
  poseShown = $state<string[]>([]);

  /** パラメータを毎フレーム外から動かすもの (drivers.ts)。 */
  readonly drivers = new DriverSet();
  /** 受け手のモードのときの、適用を待つフレーム (follower.ts)。`null` なら時計で進める (ふつうのモード)。 */
  #follower: FrameQueue | null = null;
  /** 受け手のモードで、1 回描くたびに呼ぶもの (`startFollowing` の引数)。 */
  #onFollowRender: ((stats: FollowRenderStats) => void) | null = null;
  /** 受け手のモードで、前に描いてからフレームの適用にかかった時間 (ミリ秒)。 */
  #followMs = 0;
  /** 受け手のモードで、溢れて捨てたフレームの数 (受け手を作ってからの合計)。 */
  #droppedFrames = 0;

  /** WebPuppet と canvas の寿命 (作成・GPU を失ったときの作り直し・破棄・devicePixelRatio の追従。surface.ts)。 */
  readonly #surface: PuppetSurface<WebPuppet>;
  /** 開いているファイル (GPU を作り直したときに読み直す)。 */
  #file: ModelFile | null = null;
  #parameters: ParameterDef[] = [];
  #cssSize: [number, number] = [1, 1];
  #frame = 0;
  #lastTime: number | null = null;
  /** 最後に読んだ `pose_visible_json` (変わったときだけ `poseShown` を合わせる。T0736)。 */
  #poseVisible: string | null = null;
  /** GPU を失ったときのパラメータの値 (作り直した WebPuppet に戻す。待つ間に別のモデルを開いたら戻さない)。 */
  #carry: Record<string, number> | null = null;
  /**
   * 作り直した WebPuppet にモデルを読み直す (`#setUp`)。PuppetSurface が渡し直したビューを保つので、次の `#afterLoad` は
   * fit しない (T0942)。
   */
  #keepView = false;
  /** 作り直しを待つ間に利用者がモデルを開いた (作り直した後は、そのモデルを fit で見せる)。 */
  #openedWhileDown = false;
  /** 次のフレームで渡すトラッキングの入力 (渡したら `null`)。 */
  #pendingTracking: TrackingFrame | null = null;
  #lastTracking: TrackingFrame | null = null;
  /** 読み込んだモジュールのファイルと鍵 (名前ごと。GPU を作り直したときに読み直す。組を覚えるときに使う)。 */
  #moduleFiles = new Map<string, { file: ModelFile; identity: ModelIdentity }>();
  /** 描画の準備ができる前に開いたモジュール。 */
  #pendingModules: ModuleToOpen[] = [];
  /** 付けているモジュールの名前 (合成の順。GPU を作り直したときに付け直す)。 */
  #attached: string[] = [];
  /** 開いているモデルの鍵 (modelMemory.ts)。 */
  #model: ModelIdentity | null = null;
  /** モデルごとに覚えた着せ替えの組。 */
  readonly #memory: ModelMemory<DressUpSet>;
  /** モデルごとに覚えた光源の設定 (T0709)。 */
  readonly #lightingMemory: ModelMemory<LightingSettings>;
  /** モデルごとに覚えた待機モーションの選択 (T0722)。 */
  readonly #idleMemory: ModelMemory<IdleChoice>;
  /** このセッションで読んだモジュールの中身 (指紋ごと。古いものから捨てる)。 */
  #sessionFiles = new Map<string, ModelFile>();
  /** 0 より大きい間は、着せ替えの変化を利用者の操作とみなさない (組を覚えず、尋ねているのを取り消さない)。 */
  #quiet = 0;
  /** ドラッグの途中で変えて、まだ覚えていない光源の設定があるか (`commitLighting` で覚える)。 */
  #lightingPending = false;
  /** `currentMotion` を始めた優先度 (繰り返すときに使う)。 */
  #currentPriority: MotionPriority = "normal";
  /** 直前に WebPuppet に渡した `set_motion_over_tracking` の値 (変わったときだけ渡す)。 */
  #overTracking = false;
  /** `trackingKept` を WebPuppet に渡し直す必要があるか (変えたときと、WebPuppet を作り直したとき)。 */
  #keptDirty = true;

  readonly #host: PlayerHost;
  /**
   * 覚えるものの読み書き (`PlayerHost.storagePrefix` の接頭辞。storage.ts)。アプリが Player の外で覚えるもの
   * (覚えたものの一覧の並びなど) も、これを渡して同じ接頭辞で覚える。
   */
  readonly storage: PlayerStorage;

  constructor(host: PlayerHost) {
    this.#host = host;
    const store = new PlayerStorage(host.storagePrefix, host.storage);
    this.storage = store;
    this.#memory = dressUpMemory(store);
    this.#lightingMemory = lightingMemory(store);
    this.#idleMemory = idleMemory(store);
    this.lighting = loadLighting(store);
    this.lightMarkers = store.getToggle(STORAGE_NAMES.lightMarkers, true);
    this.autoRestore = loadAutoRestore(store);
    this.autoIdle = store.getToggle(STORAGE_NAMES.autoIdle, true);
    this.motionOverTracking = store.getToggle(STORAGE_NAMES.motionOverTracking, true);
    this.trackingKept = loadTrackingKept(store);
    this.#surface = new PuppetSurface<WebPuppet>({
      loadWasm: () => host.loadWasm(),
      created: (puppet, recovered) => {
        this.waitingForGpu = false;
        return this.#setUp(puppet, recovered);
      },
      failed: (e) => {
        this.waitingForGpu = false;
        // GPU を続けて失ったときと、GPU やブラウザの都合で作れなかったとき (`CreateError`) は、作り直せば描けることがある。
        this.canRestart = e instanceof GpuLostRepeatedlyError || isCreateFailure(e);
        this.status = "unavailable";
        this.error = surfaceFailureText(e);
      },
      lost: () => {
        this.#carry = { ...this.values };
      },
      waiting: () => {
        this.waitingForGpu = true;
      },
      released: () => {
        // 作り直した WebPuppet は既定 (`false` と空) から始まる。
        this.#overTracking = false;
        this.#keptDirty = true;
      },
      pixelRatioChanged: () => this.#syncView(),
      canvasClass: "viewport-canvas",
    });
  }

  /** 描画に使っている WebPuppet (作っている間と `detach` の後は `null`)。 */
  get #puppet(): WebPuppet | null {
    return this.#surface.puppet;
  }

  /**
   * GPU の電力の希望 (T0499。surface.ts の `PowerPreference`)。`attach` と作り直し (GPU を失ったとき・`restart`) のたびに
   * 同じ値を渡す。変えても今の WebPuppet はそのままなので、効かせるには `restart()` を呼ぶ (モデルなどは戻る)。既定は `"default"`。
   */
  get powerPreference(): PowerPreference {
    return this.#surface.powerPreference;
  }

  set powerPreference(value: PowerPreference) {
    this.#surface.powerPreference = value;
  }

  /**
   * `container` の中に canvas を作り、描画を始める。前に `attach` していれば、先に `detach` する。
   * 待っている間に `detach` か次の `attach` があれば、作った WebPuppet は捨てる。
   */
  async attach(container: HTMLElement): Promise<void> {
    this.detach();
    const result = await this.#surface.attach(container);
    if (result === "superseded") return;
    if (result === "missing") {
      this.status = "unavailable";
      this.error = this.#host.wasmMissingMessage ?? t("errors.wasmMissing");
      return;
    }
    this.#frame = requestAnimationFrame(this.#tick);
  }

  detach(): void {
    cancelAnimationFrame(this.#frame);
    this.#surface.detach();
    this.#lastTime = null;
    this.waitingForGpu = false;
    this.canRestart = false;
  }

  /**
   * GPU を続けて失って描画を止めた後・作れなかった後 (`canRestart`) に、同じ場所で作り直す (「作り直す」ボタン)。開いていたモデルと
   * 付けていたモジュールを読み直し、パラメータの値を戻す (GPU を失ったときの作り直しと同じ)。作り直しの回数は数え直す。
   */
  async restart(): Promise<void> {
    this.canRestart = false;
    this.error = null;
    this.status = "starting";
    cancelAnimationFrame(this.#frame);
    this.#lastTime = null;
    this.#frame = requestAnimationFrame(this.#tick);
    await this.#surface.restart();
  }

  /**
   * モデルか着せ替えのモジュールを開く。形式は中身から判別する (modelFile.ts)。
   * モジュールなら読み込んで付ける (`loadModule`)。
   *
   * モデルなら、`modules` (見本やドロップでモデルと一緒に開くモジュール) を読み込んだ後で、そのモデルに前に付けていた
   * 組 (dressUpMemory.ts) と比べ、違えば戻すか尋ねる (`autoRestore` なら戻す)。別のモデルを開くときは、読み込んでいた
   * モジュールを一覧から除く (前のモデルの組として覚えてある)。同じモデル (modelMemory.ts の `sameModel`) を
   * 開き直すときは残し、wasm が付け直す (`modules` が無ければ比べない。付けられずに外れたものがあれば、いまの組を覚え直す。
   * 組が変わらなければ覚えている組はそのまま (戻すか尋ねている・戻さなかった・戻せなかった組を、空の組で上書きしない))。
   */
  async load(file: ModelFile, modules: readonly ModuleToOpen[] = []): Promise<void> {
    const kind = detectModelKind(file);
    if (kind === "module") {
      this.loadModule(file);
      return;
    }
    if (kind === null) {
      this.error = t("errors.notModel", { name: file.name });
      return;
    }
    // ドラッグの途中の光源は、前のモデルの設定として覚えてから開く。
    this.commitLighting();
    const puppet = this.#puppet;
    // まだ描画の準備中なら、準備ができたときに読む (#setUp)。
    if (puppet === null) {
      this.#openLater(file, modules);
      return;
    }
    const identity = modelIdentity(file);
    const switching = this.#model === null || !sameModel(this.#model, identity);
    // 一緒に開くモジュールが組を変えるので、覚えている組は先に読んでおく。受け手は覚えた組を使わない (組は指揮役が決める)。
    const remembered = this.#follower === null ? this.#memory.get(identity) : null;
    const attachedBefore = [...this.#attached];
    this.#quiet++;
    try {
      // モデルを開く前に読み込んだモジュールは、どのモデルの組でもないので残す。
      const previous = switching && this.#model !== null ? this.#clearModules(puppet) : null;
      try {
        // 同じモデルなら、付けていたモジュールは wasm が付け直す。付けられなかったものは外して警告に入る。
        const warnings =
          kind === "bundle"
            ? puppet.load_bundle(file.bytes)
            : puppet.load_json(new TextDecoder().decode(file.bytes));
        this.warnings = describeLoadWarnings(
          warnings,
          parseLoadWarnings(puppet.load_warnings_json()),
        );
      } catch (e) {
        // 読み込みの途中に GPU を失った (`device_lost`)。モデルの誤りではないので文を出さず、作り直した後に開く (T0955)。
        // 作り直しは次のフレームの `checkLost` (WebGL2 は復元の待ち) で始まる。前のモデルのモジュールは除いたまま。
        if (puppet.is_lost()) {
          this.#openLater(file, modules);
          return;
        }
        this.error = `${file.name}: ${describeLoadError(e)}`;
        // 前のモデルが残るので、除いたモジュールを戻す。GPU を作り直したときも前のモデルを読み直す (`#file` は変えない)。
        if (previous !== null) this.#reloadModules(previous.files, previous.attached);
        return;
      }
      this.#file = file;
      this.#model = identity;
      this.dressUpRemembered = remembered !== null;
      this.#recallLighting(identity);
      this.#afterLoad(file);
      for (const m of modules) this.loadModule(m.file, m.attach);
    } finally {
      this.#quiet--;
    }
    // 別のモデルか、同じモデルでも見本のようにモジュールと一緒に開き直したときは、覚えている組と比べる。
    if (switching || modules.length > 0) {
      this.restoreOffer = null;
      if (
        remembered !== null &&
        needsRestore(remembered, new Set(this.#moduleFiles.keys()), this.#attached)
      ) {
        if (this.autoRestore) await this.restoreDressUp(remembered);
        else this.restoreOffer = remembered;
        return;
      }
    } else if (sameAttached(attachedBefore, this.#attached)) {
      return;
    }
    this.#remember();
  }

  /**
   * 描画の準備ができたら (`#setUp`) モデルを開く。作り直しの読み直し (`#keepView`) の途中で GPU をまた失ったときは、
   * 利用者が開いたものではないので、次の作り直しでもビューとパラメータの値を保つ。
   */
  #openLater(file: ModelFile, modules: readonly ModuleToOpen[]): void {
    if (!this.#keepView) this.#openedWhileDown = true;
    this.#file = file;
    this.#pendingModules.push(...modules);
  }

  /** モデルを読んだ後の、パラメータ・トラッキング・光源・着せ替えの一覧の用意。 */
  #afterLoad(file: ModelFile): void {
    const puppet = this.#puppet;
    if (puppet === null) return;
    // 読み直しでは同じ ID の値が引き継がれるが、別のファイルを開いたときは既定値から始める。
    puppet.reset_parameters();
    const list = parseParameterList(puppet.parameters_json());
    this.#parameters = list.parameters;
    this.sections = groupParameters(list, t("parameters.ungrouped"));
    this.values = {};
    this.#syncValues();
    this.modelName = file.name;
    this.error = null;
    // WebPuppet は読み直すと Puppet を作り直すので、トラッキングと目パチの設定を渡し直す。
    this.trackingParameterIds = puppet.tracking_parameter_ids();
    puppet.set_enabled("eye_blink", this.autoEyeBlink);
    this.#pendingTracking = this.#lastTracking;
    // 点光源の位置はキャンバスの大きさに対する割合なので、モデルごとに渡し直す。
    this.usesLighting = puppet.uses_lighting();
    this.#applyLighting();
    this.dressUpError = null;
    // ポーズは読み直すと先頭の要素から始まる (付けていたモジュールのグループも)。前のモデルの選択は引き継がない。
    this.poseGroups = [];
    this.poseShown = [];
    this.#poseVisible = null;
    this.#syncDressUp();
    // 再生の状態は読み直すと初めからになる (crates/ugoe-wasm/README.md)。
    this.#syncAnimations(puppet);
    this.status = "ready";
    const size = puppet.canvas_size();
    this.canvasSize = [size[0] ?? 1, size[1] ?? 1];
    // 作り直しで保つのは、作り直した直後の 1 回の読み込みだけ (その間に利用者が開いたモデルは fit で見せる)。
    const keepView = this.#keepView;
    this.#keepView = false;
    if (keepView) {
      this.zoom = puppet.zoom();
      this.#syncView();
    } else {
      this.fit();
    }
  }

  /**
   * 覚えていた組を戻す (省くと尋ねている組)。モジュールは、読み込んでいるもの → このセッションで読んだ中身 →
   * 覚えていた場所の順に探す。戻せなかったものは `dressUpError` に書き、覚えている組は変えない。
   */
  async restoreDressUp(set: DressUpSet | null = this.restoreOffer): Promise<void> {
    this.restoreOffer = null;
    const puppet = this.#puppet;
    const model = this.#model;
    if (set === null || puppet === null || model === null) return;
    const failures: string[] = [];
    const plan = planRestore(set, new Set(this.#moduleFiles.keys()), (fp) =>
      this.#sessionFiles.get(fp),
    );
    const files = await Promise.all(
      plan.map(async ({ module, source }): Promise<[RememberedModule, ModelFile] | null> => {
        switch (source.kind) {
          case "loaded":
            return null;
          case "cached":
            return [module, source.file];
          case "location":
            try {
              return [module, await this.#host.readFile(source.location)];
            } catch (e) {
              failures.push(describeRestoreFailure(module, message(e)));
              return null;
            }
          case "unavailable":
            failures.push(describeRestoreFailure(module));
            return null;
        }
      }),
    );
    // 読んでいる間に別のモデルを開いたら、何もしない。
    if (this.#puppet !== puppet || this.#model !== model) return;
    this.#quiet++;
    try {
      // 覚えていた名前と、読み込んで wasm が返した名前 (ファイルが書き換わっていれば違うことがある)。
      const names = new Map<string, string>();
      for (const entry of files) {
        if (entry === null) continue;
        const name = this.loadModule(entry[1], false);
        if (name !== null) names.set(entry[0].name, name);
      }
      // 合成の順を揃えるため、付けているものをすべて外してから、覚えていた順に付ける。
      for (const name of [...this.#attached]) this.detachModule(name);
      for (const remembered of set.attached) {
        const name = names.get(remembered) ?? remembered;
        if (!this.#moduleFiles.has(name)) continue;
        if (!this.attachModule(name) && this.dressUpError !== null)
          failures.push(this.dressUpError);
      }
    } finally {
      this.#quiet--;
    }
    // 戻せなかったものがあれば、覚えている組はそのままにする (もう一度開けば戻せるように)。
    // 利用者が着せ替えを操作したら、そのときの組で覚え直す。
    this.dressUpError = failures.length > 0 ? failures.join("\n") : null;
    if (failures.length === 0) this.#remember();
  }

  /** 覚えていた組を今回は戻さない (覚えている組は、着せ替えを操作するまでそのまま)。 */
  dismissRestore(): void {
    this.restoreOffer = null;
  }

  /** モデルを開いたときに、前に付けていた組を尋ねずに戻すか。 */
  setAutoRestore(enabled: boolean): void {
    this.autoRestore = enabled;
    saveAutoRestore(enabled, this.storage);
  }

  // --- 着せ替え (T0706)。付け外しの規則は docs/spec/modules.md#puppet-での着せ替え ---

  /**
   * `.ugoemod` を読み込む。`attach` なら、モデルを開いていれば続けて付ける (同じスロットのモジュールは差し替わる)。
   * 失敗は `dressUpError` に入れる。読み込めたらモジュールの名前を返す。
   */
  loadModule(file: ModelFile, attach = true): string | null {
    const puppet = this.#puppet;
    if (puppet === null) {
      // 描画の準備ができたら読んで付ける (#setUp)。
      this.#pendingModules.push({ file, attach });
      return null;
    }
    let name: string;
    try {
      name = puppet.load_module(file.bytes);
    } catch (e) {
      // GPU を失った (`device_lost`) なら、モデルと同じく作り直した後に読む (T0955)。
      if (puppet.is_lost()) {
        this.#pendingModules.push({ file, attach });
        return null;
      }
      this.dressUpError = `${file.name}: ${describeDressUpError(e)}`;
      return null;
    }
    const identity = modelIdentity(file);
    this.#moduleFiles.set(name, { file, identity });
    this.#sessionFiles.delete(identity.fingerprint);
    this.#sessionFiles.set(identity.fingerprint, file);
    for (const fp of this.#sessionFiles.keys()) {
      if (this.#sessionFiles.size <= SESSION_FILES) break;
      this.#sessionFiles.delete(fp);
    }
    this.dressUpError = null;
    const hidden = attach && puppet.has_model() ? this.#hiddenSlot(puppet, name) : null;
    if (hidden !== null) {
      // 作者用の (公開していない) スロットには自動では付けない (T0715)。一覧には「公開されていない」として出る。
      this.#syncDressUp();
      this.dressUpError = t("dressUp.hiddenSlot", { module: name, slot: hidden });
    } else if (attach && puppet.has_model()) this.attachModule(name);
    else this.#syncDressUp();
    return name;
  }

  /** 読み込んだモジュール `name` のスロットを、本体のアバタープロファイルが公開していなければ、そのスロットの ID。 */
  #hiddenSlot(puppet: WebPuppet, name: string): string | null {
    try {
      const published = parseAvatarSlots(puppet.avatar_slots_json());
      const slot = parseModules(puppet.modules_json()).find((m) => m.name === name)?.slot;
      const inModel = parseSlots(puppet.slots_json()).some((s) => s.id === slot);
      return slot !== undefined && inModel && published !== null && !published.includes(slot)
        ? slot
        : null;
    } catch {
      return null;
    }
  }

  /** 読み込んだモジュールを付ける。同じスロットに付いているモジュールとは差し替える。付けられたら `true`。 */
  attachModule(name: string): boolean {
    return this.#dressUp((puppet) => puppet.attach_module(name));
  }

  /** モジュールを外す。 */
  detachModule(name: string): boolean {
    return this.#dressUp((puppet) => puppet.detach_module(name));
  }

  /** 読み込んだモジュールを一覧から除く (付けていれば外す)。 */
  unloadModule(name: string): boolean {
    const done = this.#dressUp((puppet) => puppet.unload_module(name));
    if (done) this.#moduleFiles.delete(name);
    return done;
  }

  /** スロット `slot` に付けるモジュールを `name` にする。`null` ならそのスロットのモジュールを外す。 */
  setSlotModule(slot: string, name: string | null): boolean {
    if (name !== null) return this.attachModule(name);
    const current = this.modules.find((m) => m.slot === slot && m.attached);
    return current === undefined || this.detachModule(current.name);
  }

  /** スライダーからの設定。ドライバが動かしているパラメータは無視する。 */
  setParameter(id: string, value: number): void {
    const puppet = this.#puppet;
    if (puppet === null || this.driven.has(id)) return;
    puppet.set_parameter(id, value);
    this.values[id] = puppet.parameter(id) ?? value;
  }

  resetParameter(id: string): void {
    const def = this.#parameters.find((p) => p.id === id);
    if (def) this.setParameter(id, def.default);
  }

  /**
   * フェイストラッキングの入力を設定する (次のフレームで Rust の対応付けに渡す)。`null` で止め、
   * トラッキングが書き込んでいたパラメータを既定値に戻す。値の約束は `@ugoe/face-tracking` の `frame.ts`。
   */
  setTracking(frame: TrackingFrame | null): void {
    if (frame === null) {
      if (this.#lastTracking === null) return;
      this.#lastTracking = null;
      this.#pendingTracking = null;
      this.tracking = false;
      this.#puppet?.clear_tracking();
      // 最後の値が基準値として残るので、既定値に戻す。
      const ids = new Set(this.trackingParameterIds);
      for (const def of this.#parameters) {
        if (ids.has(def.id)) this.#puppet?.set_parameter(def.id, def.default);
      }
      this.#syncValues();
      return;
    }
    this.#lastTracking = frame;
    this.#pendingTracking = frame;
    this.tracking = true;
  }

  /** モデルの自動の目パチを有効・無効にする (トラッキング中は目の開閉に重なるため止められる)。 */
  setAutoEyeBlink(enabled: boolean): void {
    this.autoEyeBlink = enabled;
    const puppet = this.#puppet;
    if (puppet?.has_model()) puppet.set_enabled("eye_blink", enabled);
  }

  resetAllParameters(): void {
    this.#puppet?.reset_parameters();
    this.#syncValues();
  }

  // --- モーションと表情 (T0720)。規則は docs/spec/motion.md ---

  /**
   * バンドルのモーション `name` を始める (優先度は省くと `motionPriority`)。再生中のモーションより優先度が低くて
   * 始まらなければ `motionNotice` に書いて `false`。
   */
  playMotion(name: string, priority: MotionPriority = this.motionPriority): boolean {
    const puppet = this.#puppet;
    if (puppet?.has_model() !== true) return false;
    let started: boolean;
    try {
      started = puppet.start_motion(name, priority);
    } catch (e) {
      this.motionNotice = message(e);
      return false;
    }
    if (!started) {
      this.motionNotice = describeRejectedMotion(name, priority);
      return false;
    }
    this.motionNotice = null;
    this.currentMotion = name;
    this.#currentPriority = priority;
    return true;
  }

  /** 再生中のモーションをすべてフェードアウトさせる。最後に適用した値はそのまま残る。 */
  stopMotions(): void {
    const stopped = this.currentMotion;
    this.#puppet?.stop_motions();
    this.currentMotion = null;
    this.motionNotice = null;
    // 待機モーションそのものを止めたのでなければ、待機に戻す (止めたモーションはフェードアウトしながら待機に移る)。
    if (stopped !== null && stopped !== this.idleMotion) this.#startIdle();
  }

  /** 待機モーションの自動再生を切り替える。入れたら (何も再生していなければ) 待機を始め、切ったら流している待機を止める。 */
  setAutoIdle(enabled: boolean): void {
    this.autoIdle = enabled;
    this.storage.setToggle(STORAGE_NAMES.autoIdle, enabled);
    if (enabled) {
      if (this.currentMotion === null) this.#startIdle();
    } else if (this.currentMotion !== null && this.currentMotion === this.idleMotion) {
      this.stopMotions();
    }
  }

  /**
   * 自動で流す待機モーションを選ぶ (`null` で流さない)。待機を流しているなら選んだものに替える。
   * 選択は開いているモデルの分として覚える (T0722)。
   */
  setIdleMotion(name: string | null): void {
    const playingIdle = this.currentMotion !== null && this.currentMotion === this.idleMotion;
    this.idleMotion = name;
    const model = this.#model;
    if (model !== null) {
      this.#idleMemory.set(model, { motion: name });
      this.idleRemembered = true;
    }
    if (playingIdle) {
      if (name === null) this.stopMotions();
      else this.#startIdle();
    } else if (this.currentMotion === null) {
      this.#startIdle();
    }
  }

  setMotionOverTracking(enabled: boolean): void {
    this.motionOverTracking = enabled;
    this.storage.setToggle(STORAGE_NAMES.motionOverTracking, enabled);
  }

  /** モーションを優先しても、トラッキングのままにするパラメータを `ids` にする (覚える。次のフレームから効く)。 */
  setTrackingKept(ids: readonly string[]): void {
    this.trackingKept = [...new Set(ids)];
    saveTrackingKept(this.trackingKept, this.storage);
    this.#keptDirty = true;
  }

  /** 開いているモデルに覚えた待機モーションの選択を忘れる (いまの選択はそのまま。次に選ぶとまた覚える)。 */
  forgetIdleMotion(): void {
    const model = this.#model;
    if (model !== null) this.#idleMemory.delete(model);
    this.idleRemembered = false;
  }

  /** 自動再生が入っていて待機モーションがあれば、待機の優先度で始める。 */
  #startIdle(): void {
    // 受け手はモーションを使わない (値は指揮役が決める)。
    if (this.#follower !== null) return;
    const name = this.idleMotion;
    if (!this.autoIdle || name === null || !this.motions.some((m) => m.name === name)) return;
    const puppet = this.#puppet;
    if (puppet?.has_model() !== true) return;
    try {
      if (!puppet.start_motion(name, "idle")) return;
    } catch (e) {
      this.motionNotice = message(e);
      return;
    }
    this.currentMotion = name;
    this.#currentPriority = "idle";
  }

  setMotionPriority(priority: MotionPriority): void {
    this.motionPriority = priority;
  }

  setRepeatMotion(repeat: boolean): void {
    this.repeatMotion = repeat;
  }

  /** 表情を `name` だけにする (使っている表情はフェードアウトする)。 */
  setExpression(name: string): void {
    this.#expressionOp((p) => p.set_expression(name));
  }

  /** 使っている表情の上に `name` を重ねる。使っていれば何もしない。 */
  addExpression(name: string): void {
    if (this.activeExpressions.includes(name)) return;
    this.#expressionOp((p) => p.add_expression(name));
  }

  /** 表情 `name` をフェードアウトさせる。 */
  stopExpression(name: string): void {
    this.#expressionOp((p) => p.stop_expression(name));
  }

  /** 使っている表情をすべてフェードアウトさせる。 */
  stopExpressions(): void {
    this.#expressionOp((p) => p.stop_expressions());
  }

  /** 表情の操作を 1 つ行い、使っている表情の一覧を WebPuppet から読み直す (T0721)。 */
  #expressionOp(action: (puppet: WebPuppet) => void): void {
    const puppet = this.#puppet;
    if (puppet?.has_model() !== true) return;
    try {
      action(puppet);
      this.motionNotice = null;
    } catch (e) {
      this.motionNotice = message(e);
    }
    this.activeExpressions = puppet.active_expressions();
  }

  // --- ポーズ (T0735)。規則は docs/spec/motion.md#ポーズ ---

  /**
   * ポーズの要素 `part` (`poseGroups` の要素のパーツ) を、そのグループの表示中の要素にする (`show_pose`。次のフレームから
   * フェードで切り替わる)。`immediately` なら、フェードを飛ばしてすぐに表示する (`show_pose_immediately`。読み込みの直後など、
   * 切り替えの途中を見せたくないとき。docs/spec/motion.md#ポーズ)。要素でなければ何もせず `false`。
   */
  showPose(part: string, options: { immediately?: boolean } = {}): boolean {
    const puppet = this.#puppet;
    if (puppet?.has_model() !== true) return false;
    const index = this.poseGroups.findIndex((g) => g.parts.includes(part));
    if (index < 0) return false;
    const shown = options.immediately ? puppet.show_pose_immediately(part) : puppet.show_pose(part);
    if (!shown) return false;
    this.poseShown = this.poseShown.map((shown, i) => (i === index ? part : shown));
    return true;
  }

  /**
   * ポーズのグループを WebPuppet から読み直す (着せ替えの一覧と一緒に)。要素の並びが前と同じグループは選択を保ち、ほかは
   * 先頭の要素にする。選んだ要素は WebPuppet にも渡し直す (読み直した後で、欄と描く姿が食い違わないように)。
   */
  #syncPose(puppet: WebPuppet): void {
    let groups: PoseGroupInfo[] = [];
    try {
      if (puppet.has_model()) groups = parsePose(puppet.pose_json());
    } catch (e) {
      console.warn("ugoe:", e);
    }
    const shown = carryPoseShown(groups, this.poseGroups, this.poseShown);
    for (const part of shown) {
      try {
        puppet.show_pose(part);
      } catch {
        // モデルが無ければ何もしない。
      }
    }
    this.poseGroups = groups;
    this.poseShown = shown;
    // 次のフレームで表示中の要素を読み直す (グループが変わると、同じ JSON でも並びの意味が変わる)。
    this.#poseVisible = null;
  }

  /**
   * 表示中の要素を読み、モーションが切り替えた要素に `poseShown` を合わせる (T0736)。読んだ JSON が前のフレームと同じなら
   * 何もしない (毎フレームの読み出しで欄を作り直さないため)。利用者が選んだ要素も `show_pose` の直後から表示中なので食い違わない。
   */
  #followPose(puppet: WebPuppet): void {
    if (this.poseGroups.length === 0) return;
    let json: string | undefined;
    try {
      json = puppet.pose_visible_json();
    } catch (e) {
      console.warn("ugoe:", e);
      return;
    }
    if (json === undefined || json === this.#poseVisible) return;
    this.#poseVisible = json;
    const shown = followPoseShown(this.poseGroups, json, this.poseShown);
    if (shown !== this.poseShown) this.poseShown = [...shown];
  }

  // --- 受け手のモード (スタジオの出力ウィンドウ。follower.ts、ADR 0028 の 3.) ---

  /**
   * 受け手のモードにする。以後は時計の `dt` で `update` せず、`pushFrame` で届いたフレームを順に `follow` で適用する
   * (ドライバ・トラッキング・モーション・目パチなどは使わない。値は指揮役が決める)。光源の設定と着せ替えの組は
   * 覚えず、覚えたものも使わない (シーンが決める)。モデルは `load` で読み、
   * 読み終えたら `followScene(revision)` でそのシーンのフレームを受け付ける。
   */
  startFollowing(onRender?: (stats: FollowRenderStats) => void): void {
    this.#follower ??= new FrameQueue();
    this.#onFollowRender = onRender ?? null;
  }

  /** 受け手のモードか。 */
  get following(): boolean {
    return this.#follower !== null;
  }

  /**
   * シーン `revision` のモデルを読み終えた。そのシーンのフレームを最初から適用する (前のシーンのフレームは捨てる)。
   * `partIds` はシーンのパーツの並び (`SceneMessage.partIds`。読んだモデルとモジュールの並びと同じ。T0853)。
   */
  followScene(revision: number, partIds: readonly string[] = []): void {
    this.#follower?.setScene(revision, partIds);
    this.#followPending();
  }

  /**
   * モデルを読み直さずにシーン `revision` に移る (見せ方や着せ替えだけが変わった)。前のシーンのフレームも捨てずに
   * 順に適用する (follower.ts の `FrameQueue.continueScene`)。前のシーンのフレームのパーツの番号は、`partIds`
   * (このシーンのパーツの並び) の番号に引き直す (T0853)。モジュールを付け直したなら、付け直した後に呼ぶ。
   */
  continueScene(revision: number, partIds: readonly string[] = []): void {
    this.#follower?.continueScene(revision, partIds);
    this.#followPending();
  }

  /**
   * 指揮役のフレームを受け取り、適用できるものはその場で `follow` まで適用する (描くのは次の `requestAnimationFrame`)。
   * ページが隠れて `requestAnimationFrame` が止まっても、受け手の物理は止まらない (docs/studio.md#隠れても止まらない)。
   * 受け手のモードでなければ何もしない。
   */
  pushFrame(frame: FollowFrame): void {
    const queue = this.#follower;
    if (queue === null) return;
    const before = queue.pending;
    queue.push(frame);
    // 溢れて捨てた分 (待っているフレームが増えなかった)。
    this.#droppedFrames += before + 1 - queue.pending;
    this.#followPending();
  }

  /** 待っているフレームを、モデルを読み終えていれば今すぐ適用する。 */
  #followPending(): void {
    const puppet = this.#puppet;
    const queue = this.#follower;
    if (puppet === null || queue === null || !puppet.has_model() || puppet.is_lost()) return;
    const started = performance.now();
    this.#follow(puppet, queue);
    this.#followMs += performance.now() - started;
  }

  // --- ビュー。座標は canvas 要素の左上からの CSS ピクセル ---

  fit(): void {
    this.#puppet?.fit();
    this.zoom = this.#puppet?.zoom() ?? 1;
    this.#syncView();
  }

  /** `(x, y)` を中心に `factor` 倍する。位置を省くと中央。 */
  zoomBy(factor: number, x = this.#cssSize[0] / 2, y = this.#cssSize[1] / 2): void {
    const puppet = this.#puppet;
    if (puppet === null) return;
    const dpr = this.#surface.pixelRatio;
    puppet.zoom_at(factor, x * dpr, y * dpr);
    this.zoom = puppet.zoom();
    this.#syncView();
  }

  panBy(dx: number, dy: number): void {
    const dpr = this.#surface.pixelRatio;
    this.#puppet?.pan_by(dx * dpr, dy * dpr);
    this.#syncView();
  }

  /** canvas 要素の大きさ (CSS ピクセル) が変わった。 */
  resize(width: number, height: number): void {
    this.#cssSize = [Math.max(1, width), Math.max(1, height)];
    this.viewportSize = [...this.#cssSize];
    this.#surface.resize(...this.#cssSize);
    this.#syncView();
  }

  setBackground(color: string, transparent: boolean): void {
    this.background = color;
    this.transparent = transparent;
    this.#applyBackground();
  }

  /**
   * 今の表示を画像で読み戻す (スクリーンショット、T0420。PNG への変換は screenshot.ts の encodePng、保存はアプリ)。
   * `WebPuppet.read_pixels()` で、canvas と同じ大きさ (描画バッファの画素)・ビュー・背景・光源で別のテクスチャに描いたものを取る。ビューポートに重ねる HTML
   * (光源の印・市松模様) は写らない。モデルを開いていなければ `null`。
   */
  async capture(): Promise<Screenshot | null> {
    const puppet = this.#puppet;
    if (puppet === null || !puppet.has_model()) return null;
    const width = puppet.width();
    const height = puppet.height();
    const pixels = await puppet.read_pixels();
    return { width, height, pixels };
  }

  /** 光源の設定を変える (次のフレームから効く)。最後に使った設定と、開いているモデルの設定として覚える。 */
  setLighting(settings: LightingSettings): void {
    this.#lightingPending = false;
    this.lighting = settings;
    // 受け手は光源を覚えない (シーンが決める)。
    if (this.#follower !== null) {
      this.#applyLighting();
      return;
    }
    saveLighting(settings, this.storage);
    const model = this.#model;
    if (model !== null) {
      this.#lightingMemory.set(model, settings);
      this.lightingRemembered = true;
    }
    this.#applyLighting();
  }

  /**
   * 開いているモデルの光源を既定 (lighting.ts の `defaultLighting`、「標準」) に戻し、そのモデルの設定として覚える
   * (次に開いたときも「標準」で始まる。T0714)。覚えた設定を忘れるのは `forgetRemembered`。
   */
  resetLighting(): void {
    this.setLighting(defaultLighting());
  }

  /**
   * 開いているモデルに覚えた着せ替えの組を忘れる。付けているモジュールはそのままで、尋ねている組は取り消す。
   * 次に着せ替えを操作すると、そのときの組をまた覚える。
   */
  forgetDressUp(): void {
    const model = this.#model;
    if (model !== null) this.#memory.delete(model);
    this.dressUpRemembered = false;
    this.restoreOffer = null;
  }

  /**
   * 光源の設定の一部を書き換える (`edit` は写しを書き換える)。`preview` なら描画だけを変えて覚えない
   * (ドラッグの途中。離したときに `commitLighting` で覚える。localStorage に毎回書かないため)。
   */
  updateLighting(edit: (settings: LightingSettings) => void, { preview = false } = {}): void {
    const next = $state.snapshot(this.lighting) as LightingSettings;
    edit(next);
    if (!preview) {
      this.setLighting(next);
      return;
    }
    this.lighting = next;
    this.#lightingPending = true;
    this.#applyLighting();
  }

  /** `preview` で変えた光源の設定を、最後に使った設定と開いているモデルの設定として覚える。変えていなければ何もしない。 */
  commitLighting(): void {
    if (this.#lightingPending) this.setLighting($state.snapshot(this.lighting) as LightingSettings);
  }

  /**
   * モデルごとに覚えたもの (光源の設定・着せ替えの組・待機モーションの選択) の一覧 (rememberedModels.ts、T0714)。
   * 並びは `order` (T0717)。
   */
  rememberedModels(order: RememberedOrder = "recent"): RememberedModel[] {
    return mergeRemembered(
      {
        lighting: this.#lightingMemory.entries(),
        dressUp: this.#memory.entries(),
        idle: this.#idleMemory.entries(),
      },
      this.#model,
      order,
    );
  }

  /**
   * 一覧の `model` に覚えたものを忘れる (`lighting`・`dressUp`・`idle` で選ぶ)。開いているモデルなら、いまの光源・
   * 付けているモジュール・待機モーションはそのままで、パネルの「覚えた」の表示を消す (次に変えるとまた覚える)。
   */
  forgetRemembered(
    model: ModelIdentity,
    {
      lighting = true,
      dressUp = true,
      idle = true,
    }: { lighting?: boolean; dressUp?: boolean; idle?: boolean } = {},
  ): void {
    if (lighting) this.#lightingMemory.delete(model);
    if (dressUp) this.#memory.delete(model);
    if (idle) this.#idleMemory.delete(model);
    this.#afterForget((current) => sameModel(current, model), { lighting, dressUp, idle });
  }

  /** モデルごとに覚えたものをすべて忘れる (最後に使った光源と、印・自動で戻す・自動で流すの設定は残す)。 */
  forgetAllRemembered(): void {
    this.#lightingMemory.clear();
    this.#memory.clear();
    this.#idleMemory.clear();
    this.#afterForget(() => true, { lighting: true, dressUp: true, idle: true });
  }

  /** ビューポートに光源の印を出すかを変える (覚える)。 */
  setLightMarkers(show: boolean): void {
    this.lightMarkers = show;
    this.storage.setToggle(STORAGE_NAMES.lightMarkers, show);
  }

  // --- 内部 ---

  /** 覚えたものを忘れた後、開いているモデルの分なら表示を合わせる。 */
  #afterForget(
    affects: (current: ModelIdentity) => boolean,
    { lighting, dressUp, idle }: { lighting: boolean; dressUp: boolean; idle: boolean },
  ): void {
    const current = this.#model;
    if (current === null || !affects(current)) return;
    if (lighting) {
      // ドラッグの途中の設定を、忘れた直後に覚え直さない。
      this.#lightingPending = false;
      this.lightingRemembered = false;
    }
    if (dressUp) {
      this.dressUpRemembered = false;
      this.restoreOffer = null;
    }
    if (idle) this.idleRemembered = false;
  }

  /**
   * 読み込んでいたモジュールをすべて一覧から除く (別のモデルを開く前)。除いたファイルと付けていたものを返す
   * (開けなかったときに戻す)。中身はこのセッションの間は手元に残る (`#sessionFiles`)。
   */
  #clearModules(puppet: WebPuppet): { files: ModelFile[]; attached: string[] } {
    const previous = {
      files: [...this.#moduleFiles.values()].map((m) => m.file),
      attached: [...this.#attached],
    };
    for (const name of this.#moduleFiles.keys()) {
      try {
        puppet.unload_module(name);
      } catch {
        // 読み込んでいなければ何もしない。
      }
    }
    this.#moduleFiles.clear();
    this.#attached = [];
    this.dressUpError = null;
    this.#syncDressUp();
    return previous;
  }

  /** モジュールを読み込み直し、`attached` を (合成の順に) 付ける。 */
  #reloadModules(files: ModelFile[], attached: string[]): void {
    this.#quiet++;
    try {
      for (const file of files) this.loadModule(file, false);
      if (this.#puppet?.has_model()) for (const name of attached) this.attachModule(name);
    } finally {
      this.#quiet--;
    }
  }

  /** 開いているモデルに、いま読み込んでいるモジュールの組を覚える。何も読み込んでいなければ忘れる。 */
  #remember(): void {
    const model = this.#model;
    if (model === null || this.#quiet > 0 || this.#puppet?.has_model() !== true) return;
    // 受け手は着せ替えの組を覚えない (組は指揮役が決める)。
    if (this.#follower !== null) return;
    const modules = [...this.#moduleFiles].map(([name, m]) => ({ name, file: m.identity }));
    if (modules.length === 0) this.#memory.delete(model);
    else this.#memory.set(model, { modules, attached: [...this.#attached] });
    this.dressUpRemembered = modules.length > 0;
  }

  /**
   * 開いたモデルに覚えた光源の設定があれば、それにする (無ければ最後に使った設定のまま)。覚えた設定は最近使った
   * ものにし、鍵も新しくする (ドロップで覚えたモデルを「開く」で開くと場所が付く)。渡すのは `#afterLoad`。
   */
  #recallLighting(model: ModelIdentity): void {
    if (this.#follower !== null) return;
    const saved = this.#lightingMemory.get(model);
    this.lightingRemembered = saved !== null;
    if (saved === null) return;
    this.#lightingMemory.set(model, saved);
    this.lighting = saved;
    saveLighting(saved, this.storage);
  }

  /**
   * 付け外しを 1 つ行い、一覧を読み直す。失敗したら (wasm は何も変えない) 理由を `dressUpError` に入れる。GPU を失っていたら
   * (`device_lost`)、WebPuppet が無い間と同じく何もせずに `false` を返す (作り直しは付けていた組で戻す。T0955)。
   */
  #dressUp(action: (puppet: WebPuppet) => void): boolean {
    const puppet = this.#puppet;
    if (puppet === null) return false;
    try {
      action(puppet);
      this.dressUpError = null;
      return true;
    } catch (e) {
      if (puppet.is_lost()) return false;
      this.dressUpError = describeDressUpError(e);
      return false;
    } finally {
      this.#syncDressUp();
    }
  }

  /** モーションと表情の一覧を読み直し、再生の状態を初めに戻す (モデルを読んだ後)。 */
  #syncAnimations(puppet: WebPuppet): void {
    try {
      this.motions = parseMotions(puppet.motions_json());
      this.expressions = parseExpressions(puppet.expressions_json());
    } catch (e) {
      this.motions = [];
      this.expressions = [];
      this.motionNotice = message(e);
      return;
    }
    this.currentMotion = null;
    this.activeExpressions = puppet.active_expressions();
    this.motionEvents = [];
    this.motionNotice = null;
    this.#recallIdleMotion();
    this.#startIdle();
  }

  /**
   * 開いているモデルの待機モーションを、覚えた選択か既定にする (T0722)。覚えた選択は最近使ったものにし、鍵も新しくする
   * (光源の `#recallLighting` と同じ)。受け手は待機モーションを使わないので、覚えた選択を読まず書き直さない。
   */
  #recallIdleMotion(): void {
    if (this.#follower !== null) {
      this.idleRemembered = false;
      this.idleMotion = null;
      return;
    }
    const model = this.#model;
    const saved = model === null ? null : this.#idleMemory.get(model);
    this.idleRemembered = saved !== null;
    if (model !== null && saved !== null) this.#idleMemory.set(model, saved);
    this.idleMotion = initialIdleMotion(this.motions, saved);
  }

  /** `update` の後で、モーションの終わりとイベントを読む。 */
  #afterMotionUpdate(puppet: WebPuppet): void {
    const fired = parseMotionEvents(puppet.motion_events_json());
    if (fired.length > 0)
      this.motionEvents = [...fired.reverse(), ...this.motionEvents].slice(0, MOTION_EVENTS);
    const current = this.currentMotion;
    if (current === null || puppet.is_motion_playing()) return;
    // 終わった。繰り返すなら同じ優先度で始め直す (終わったので、どの優先度でも始まる)。
    if (this.repeatMotion && puppet.start_motion(current, this.#currentPriority)) return;
    this.currentMotion = null;
    // 始められなかった理由 (再生中のモーションの優先度) は、そのモーションが終わったので消す。
    this.motionNotice = null;
    this.#startIdle();
  }

  /** 着せ替えの一覧を WebPuppet から読み直す。モジュールのメッシュでライティングの有無も変わる。 */
  #syncDressUp(): void {
    const puppet = this.#puppet;
    if (puppet === null) return;
    try {
      this.modules = parseModules(puppet.modules_json());
      this.slots = parseSlots(puppet.slots_json());
      this.publishedSlots = parseAvatarSlots(puppet.avatar_slots_json());
    } catch (e) {
      this.dressUpError = message(e);
    }
    this.#attached = puppet.attached_modules();
    if (puppet.has_model()) this.usesLighting = puppet.uses_lighting();
    this.#syncPose(puppet);
    // 利用者が着せ替えを操作した: 尋ねている組は取り消し、いまの組を覚える。
    if (this.#quiet === 0) {
      this.restoreOffer = null;
      this.#remember();
    }
  }

  #tick = (now: number): void => {
    this.#frame = requestAnimationFrame(this.#tick);
    const puppet = this.#puppet;
    if (puppet === null || this.#surface.checkLost()) return;
    const dt = this.#lastTime === null ? 0 : Math.min((now - this.#lastTime) / 1000, MAX_DT);
    this.#lastTime = now;
    const started = performance.now();
    if (this.#follower !== null) {
      // ふつうは届いたときに適用済み。モデルを読み終える前に届いたものなどが残っていれば、ここで適用する。
      if (puppet.has_model()) this.#follow(puppet, this.#follower);
    } else if (puppet.has_model()) this.#advance(puppet, dt);
    try {
      puppet.render();
    } catch (e) {
      const text = message(e);
      if (this.error !== text) this.error = text;
    }
    if (this.#onFollowRender !== null) {
      const renderMs = this.#followMs + performance.now() - started;
      this.#followMs = 0;
      this.#onFollowRender({ at: now, renderMs, dropped: this.#droppedFrames });
    }
  };

  /**
   * 1 フレーム分進める: ドライバの値を設定 → トラッキングの入力を渡す → モーションとトラッキングの設定 → `update(dt)` →
   * モーションの終わりとイベント → スライダーの値を同期。時計 (`dt`) と描画は `#tick` が受け持つ。
   */
  #advance(puppet: WebPuppet, dt: number): void {
    const driven = this.drivers.collect(dt);
    for (const [id, value] of driven) puppet.set_parameter(id, value);
    const frame = this.#pendingTracking;
    if (frame !== null) {
      this.#pendingTracking = null;
      setPuppetTracking(puppet, frame);
    }
    const ids = new Set(driven.keys());
    if (this.tracking) {
      // トラッキングは update の中で書き込むので、ドライバの値より優先される。
      for (const id of this.trackingParameterIds) ids.add(id);
    }
    // 再生中のモーションが動かすパラメータは、スライダーで変えても上書きされる (docs/spec/motion.md#手で動かしたパラメータとの関係)。
    const playing = this.currentMotion;
    if (playing !== null) {
      for (const id of this.motions.find((m) => m.name === playing)?.parameters ?? []) ids.add(id);
    }
    if (ids.size !== this.driven.size || [...ids].some((id) => !this.driven.has(id))) {
      this.driven = ids;
    }
    const over = motionOverTrackingActive(
      this.motionOverTracking,
      this.tracking,
      this.currentMotion,
      this.#currentPriority,
    );
    if (this.#keptDirty) {
      this.#keptDirty = false;
      puppet.set_tracking_kept($state.snapshot(this.trackingKept));
    }
    if (over !== this.#overTracking) {
      this.#overTracking = over;
      puppet.set_motion_over_tracking(over);
    }
    puppet.update(dt);
    this.#afterMotionUpdate(puppet);
    this.#syncValues();
    this.#followPose(puppet);
  }

  /**
   * 受け手のモードで、届いたフレームを順にすべて適用する (描くのは `#tick` が最後に 1 回)。時計の `dt` は使わない。
   * 値の表示 (`values`) は同期しない (出力ウィンドウは値を出さないため)。
   */
  #follow(puppet: WebPuppet, queue: FrameQueue): void {
    const { follow, reset_physics } = puppet;
    if (follow === undefined || reset_physics === undefined) {
      this.error = t("errors.noFollowerApi");
      return;
    }
    const follower = {
      follow: follow.bind(puppet),
      reset_physics: reset_physics.bind(puppet),
    };
    for (const { frame, gap, parts } of queue.take()) {
      try {
        applyFrame(follower, frame, gap, parts);
      } catch (e) {
        // 値の数が合わない (シーンとモデルが食い違う) など。そのフレームは飛ばし、次で物理をリセットする。
        const text = message(e);
        if (this.error !== text) this.error = text;
      }
    }
  }

  /** WebPuppet の値をスライダーの表示へ写す (変わったものだけ)。 */
  #syncValues(): void {
    const puppet = this.#puppet;
    if (puppet === null) return;
    for (const { id } of this.#parameters) {
      const value = puppet.parameter(id);
      if (value === undefined) continue;
      const shown = this.values[id];
      if (shown === undefined || Math.abs(shown - value) > EPSILON) this.values[id] = value;
    }
  }

  #applyLighting(): void {
    const puppet = this.#puppet;
    if (puppet === null) return;
    const size = puppet.canvas_size();
    const json = toLightingJson(this.lighting, [size[0] ?? 1, size[1] ?? 1]);
    try {
      puppet.set_lighting_json(json === null ? undefined : JSON.stringify(json));
    } catch (e) {
      this.error = t("lighting.error", { message: message(e) });
    }
  }

  /** ビューの変換を読み直す (ズーム・パン・大きさ・モデルが変わったとき)。 */
  #syncView(): void {
    const t = this.#puppet?.has_model() ? this.#puppet.view_transform() : [];
    const [scale, x, y] = t;
    if (scale === undefined || x === undefined || y === undefined) {
      this.view = null;
      return;
    }
    // WebPuppet の画面座標は描画先の画素 (デバイスピクセル)。
    const dpr = this.#surface.pixelRatio;
    this.view = { scale: scale / dpr, x: x / dpr, y: y / dpr };
  }

  #applyBackground(): void {
    const rgb = parseHexColor(this.background) ?? [1, 1, 1];
    this.#puppet?.set_background(rgb[0], rgb[1], rgb[2], this.transparent ? 0 : 1);
  }

  /**
   * できた WebPuppet (attach と、GPU を失った後の作り直し) を使い始める。作り直しなら、同じモデルを読み直してから、
   * モジュールを読み直して付けていたもの (合成の順) を付け直し、パラメータの値を戻す (待つ間に別のモデルを開いたら戻さない)。
   */
  async #setUp(puppet: WebPuppet, recovered: boolean): Promise<void> {
    this.backend = puppet.backend();
    this.adapterName = puppet.adapter_name();
    this.#syncView();
    this.#applyBackground();
    this.status = "empty";
    // 準備中に開いたモジュールは、モデルと一緒に開く (モデルが無ければ読み込むだけ)。
    const pending = this.#pendingModules.splice(0);
    // 作り直しは利用者の操作ではないので、組を覚え直さない。
    const attached = [...this.#attached];
    const known = [...this.#moduleFiles.values()].map((m) => m.file);
    this.#moduleFiles.clear();
    if (recovered) this.#quiet++;
    // 作り直しでは、PuppetSurface が渡し直したビュー (利用者のズームとパン) とパラメータの値を保つ。待つ間に開いたモデルは
    // fit で見せ、値も既定から始める (前のモデルの値を同じ ID のパラメータに戻さない。T0945)。
    const openedWhileDown = this.#openedWhileDown;
    this.#keepView = recovered && !openedWhileDown;
    this.#openedWhileDown = false;
    try {
      if (this.#file !== null) await this.load(this.#file, pending);
      else for (const m of pending) this.loadModule(m.file, m.attach);
      this.#reloadModules(known, attached);
    } finally {
      this.#keepView = false;
      if (recovered) this.#quiet--;
    }
    const carry = this.#carry;
    this.#carry = null;
    // 作り直している間に detach された。
    if (!recovered || openedWhileDown || carry === null || this.#puppet !== puppet) return;
    for (const [id, value] of Object.entries(carry)) puppet.set_parameter(id, value);
    this.#syncValues();
  }
}
