// バックエンド (src-tauri/src/lib.rs) との口。Tauri の外 (ヘッドレスのブラウザでの確認) では
// `window.__UGOE_STUDIO_HOST__` に置いたものを使う (README.md#ヘッドレスのブラウザで確かめる)。
import type { TrackingFrame } from "@ugoe/face-tracking";
import type {
  LightingSettings,
  MotionPriority,
  PoseGroupInfo,
  PowerPreference,
  SceneMessage,
} from "@ugoe/player";
import type { ApiPermission, ApiView } from "./api";
import { safeChannelHandler } from "./channel";
import type { CommandMessage } from "./commandMessage";
import type {
  AnimationView,
  Hotkey,
  HotkeyAction,
  HotkeyProblem,
  InputSettings,
  MotionSettings,
} from "./inputs";
import type { Background, ProfileView, SceneSettings, Transform } from "./profile";
import type { VirtualCameraProfile, VirtualCameraStatus } from "./virtualCamera";

/** 操作ウィンドウに出すパラメータの定義 (lib.rs の `ParameterInfo`)。 */
export interface ParameterInfo {
  id: string;
  name: string;
  min: number;
  max: number;
  default: number;
  /** 中継用のパラメータ (一覧では既定で隠す)。 */
  internal: boolean;
}

/** 出力ウィンドウの状態 (lib.rs の `OutputState`)。大きさは物理ピクセル。 */
export interface OutputState {
  visible: boolean;
  onTop: boolean;
  width: number;
  height: number;
  /**
   * 透過が画面で効くか (lib.rs の `Transparency`。docs/studio.md#出力ウィンドウの透過)。`unsupported` は画面を合成していない
   * (X11 でコンポジタが無いなど) か、ウィンドウが透過の色を持たない。配信ソフトの取り込みが透過を拾うかは含まない。
   */
  transparency: "supported" | "unsupported" | "unknown";
}

/** 出力が報告する描画の負荷 (lib.rs の `RenderStats`。直近の約 1 秒)。 */
export interface RenderStats {
  /** 描いた回数 (回/秒)。出力ウィンドウとブラウザソースは変わったときだけ描くので、刻みの速さを上限に、届いたフレームの数に従う。 */
  fps: number;
  /**
   * 描く機会 (`requestAnimationFrame`) の回数 (回/秒。画面のリフレッシュレート、描画が重ければそれより少ない)。
   * 出力ウィンドウとブラウザソースだけで、仮想カメラは `null` (描く機会ごとに描く)。
   */
  refreshFps: number | null;
  /** 1 回の描画 (届いたフレームの適用と描画) の CPU の時間の平均 (ミリ秒)。 */
  renderMs: number;
  /** 1 回の描画の CPU の時間の最大 (ミリ秒)。 */
  renderMaxMs: number;
  /** 溢れて捨てたフレームの数 (受け手を作ってからの合計)。 */
  dropped: number;
  /**
   * 描画に使っている API (WebView は `webgpu`・`webgl2`、仮想カメラは wgpu のバックエンドの `vulkan`・`dx12`・`metal`・`gl`)。
   * 分からなければ `null` か省く (T0860)。
   */
  backend?: string | null;
  /** 使っている GPU の名前 (ブラウザが伏せると空文字)。分からなければ `null` か省く (T0860)。 */
  adapter?: string | null;
}

/** 使う GPU の希望 (gpu.rs の `GpuPreference`。T0860、docs/studio.md#使う-gpu)。 */
export type GpuPreference = "auto" | "lowPower" | "highPerformance";

/** GPU の優先度 (gpu.rs の `GpuPriority`。Windows だけ。T0861、docs/studio.md#gpu-の優先度)。 */
export type GpuPriority = "normal" | "low";

/** 使う GPU の様子 (gpu.rs の `GpuView`)。 */
export interface GpuView {
  /** 今の希望 (仮想カメラは次に始めるときにこれを使う)。 */
  preference: GpuPreference;
  /** 起動したときの希望 (出力ウィンドウが使っている。今の希望と違えば、起動し直すと効く)。 */
  launched: GpuPreference;
  /** OS の「グラフィックの設定」を開けるか (Windows だけ)。 */
  graphicsSettings: boolean;
  /** 選んでいる GPU の優先度 (T0861)。 */
  priority: GpuPriority;
  /** GPU の優先度を選べるか (Windows だけ)。 */
  prioritySupported: boolean;
  /** GPU の優先度を当てられなかった理由 (スタジオのプロセス・WebView2 の GPU のプロセスの順。当てられていれば空)。 */
  priorityErrors: CommandMessage[];
}

/** 出力ごとの描画の負荷 (lib.rs の `RendererLoad`)。 */
export interface RendererLoad extends RenderStats {
  /** 出力のラベル (`output` は出力ウィンドウ)。 */
  label: string;
  /** 報告が途絶えている (隠れていて描いていない・閉じた)。 */
  stale: boolean;
  /** この出力の目標の描く回数 (回/秒)。`null` なら刻みの速さ (出力ウィンドウ)。仮想カメラはそのフレームレート。 */
  targetFps: number | null;
}

/**
 * 受け手が今のシーンのファイルを読めなかったこと (load_failure.rs の `LoadFailure`。docs/studio.md#受け手が読めなかったとき)。
 * 次のシーンに移ったための失敗は報告しない。
 */
export interface LoadFailure {
  /** シーンの版。今のシーンでなければバックエンドが捨てる。 */
  revision: number;
  asset: "model" | "module" | "background";
  /** モジュールなら、その番号 (`StudioScene.modules` の順)。ほかは `null`。 */
  index: number | null;
  /**
   * 理由。`unavailable` は受け取れなかった、`unreadable` は受け取ったが読めなかった、
   * `noRenderer` は受け手が描画を始められない (wasm・WebGPU・WebGL2 が使えない)。
   */
  code: "unavailable" | "unreadable" | "noRenderer";
}

/** 操作ウィンドウに出す、受け手ごとの読めなかったこと (load_failure.rs の `ReceiverFailure`)。 */
export interface ReceiverFailure extends LoadFailure {
  /** 受け手のラベル (`RendererLoad.label` と同じ)。 */
  label: string;
}

/** 間引いた今の値と負荷 (lib.rs の `Monitor::Status`)。 */
export interface StatusMessage {
  kind: "status";
  seq: number;
  values: number[];
  /** 直近の 1 刻みにかかった時間 (ミリ秒)。 */
  tickMs: number;
  /** フレームを受けている出力の数。 */
  outputs: number;
  /** 設定した刻みの速さ (回/秒)。 */
  tickRate: number;
  /** 実際の刻みの速さ (回/秒。測り終えるまでは `null`)。 */
  measuredRate: number | null;
  /** 直近 1 秒の刻みの間隔の最大 (ミリ秒)。 */
  maxIntervalMs: number | null;
  /** 直近 1 秒の、トラッキングの値が届いてから刻みで使われるまでの待ち (値が届いていなければ `null`。T0863)。 */
  trackingWait: TrackingWait | null;
  renderers: RendererLoad[];
}

/** トラッキングの値が届いてから刻みで使われるまでの待ち (pacing.rs の `TrackingWait`)。 */
export interface TrackingWait {
  /** 刻みで使われた値の待ちの平均 (ミリ秒)。 */
  meanMs: number;
  /** 待ちの最大 (ミリ秒)。 */
  maxMs: number;
  /** 届いた値のうち、狙った刻みに遅れた割合 (0〜1)。 */
  late: number;
  /** 今の促しの余裕 (ミリ秒。1〜4)。促しを寄せていなければ `null` (T0867)。 */
  marginMs: number | null;
}

/**
 * シーンの通知 (stream.rs の `Scene`)。受け手は版の番号を比べて、モデルの読み直し・モジュールの付け直し・
 * 見せ方だけの変更を選ぶ (docs/studio.md#シーンの切り替えと受け手)。
 */
export interface StudioScene extends SceneMessage {
  /** モデルを読んだときの `revision`。モデルが無ければ 0。 */
  modelRevision: number;
  /** 付けるモジュールの組を変えたときの `revision`。 */
  modulesRevision: number;
  /** 付けているモジュールの名前 (合成の順。`sceneModule` の添字の順)。 */
  modules: string[];
  lighting: LightingSettings | null;
  transform: Transform;
  background: Background;
  /**
   * ポーズのグループごとに表示する要素のパーツ (合成したモデルのポーズのグループの順。T0842)。受け手は読み直し・付け直しの後に
   * これを表示する要素にする。シーンを出したときはシーンの選択、接続したときは指揮役が今表示している要素 (モーションが切り替えた
   * 要素を含む。T0848)。
   */
  pose: string[];
  /**
   * 描く解像度の倍率 (プロファイルの `output.renderScale`。1 / 0.75 / 0.5。T0858)。出力ウィンドウとブラウザソースは Player の
   * `renderScale` に当てる。変わっても `revision` は上がらない (同じシーンが配り直される)。
   */
  renderScale: number;
}

/** 操作ウィンドウへの通知 (lib.rs の `Monitor`)。 */
export type MonitorMessage =
  | {
      kind: "scene";
      scene: StudioScene;
      parameters: ParameterInfo[];
      usesLighting: boolean;
      /** モデル (と付けているモジュール) のポーズのグループ (T0842)。 */
      poseGroups: PoseGroupInfo[];
      /** グループごとに今表示している要素 (モーションが切り替えた要素を含む。`scene.pose` はシーンの選択。T0848)。 */
      poseShown: string[];
    }
  | {
      kind: "pose";
      /** ポーズの表示中の要素か選択が変わった: グループごとに今表示している要素 (T0848)。 */
      shown: string[];
    }
  | ({ kind: "profile" } & ProfileView)
  | StatusMessage
  | ({ kind: "output" } & OutputState)
  | ({ kind: "animation" } & AnimationView)
  | {
      kind: "hotkeys";
      /** 登録できなかった割り当ての位置と理由。 */
      problems: [number, HotkeyProblem][];
      /** 割り当ての画面でキーを押している間、登録を止めている。 */
      suspended: boolean;
      /** Wayland のセッション (アプリが前面に無いとホットキーを受けられない)。 */
      wayland: boolean;
    }
  | { kind: "hotkey"; action: HotkeyAction; error: CommandMessage | null }
  | ({ kind: "virtualCamera" } & VirtualCameraStatus)
  /** 使う GPU の様子が変わった (GPU の優先度を WebView2 の GPU のプロセスに当て直した・当てられなかった。T0861)。 */
  | ({ kind: "gpu" } & GpuView)
  | ({ kind: "api" } & ApiView)
  | {
      kind: "loadFailures";
      /**
       * 受け手が今のシーンで読めなかったこと (無ければ空。シーンが変わると空になる)。1 つの受け手で読めなかったものは
       * すべて並ぶ (ラベルの順。同じ受け手ではモデル・モジュール (番号の順)・背景の画像の順。T0840)。
       */
      failures: ReceiverFailure[];
    }
  | {
      kind: "sceneLoading";
      /**
       * 読んでいる切り替えのシーンの ID (どこから始めたものも。最後に始めた切り替え)。終わった (失敗・捨てたものも含む) ら `null`
       * (T0841。docs/studio.md#シーンの切り替えと舞台のロック)。
       */
      scene: string | null;
    };

/** 出力ウィンドウへの配信: シーンの通知 (JSON) かフレームのバイト列 (@ugoe/player の `decodeFrame`)。 */
export type StreamMessage = StudioScene | ArrayBuffer;

/** OS のダイアログで選ぶファイルの種類 (フィルタの名前は翻訳した文言を渡す)。 */
export type FileKind = "model" | "module" | "image" | "profile";

/** 種類ごとの拡張子 (stage.rs の `MODEL_EXTENSIONS` など)。 */
export const FILE_EXTENSIONS: Record<FileKind, string[]> = {
  model: ["ugoe", "json"],
  module: ["ugoemod"],
  image: ["png", "jpg", "jpeg", "webp"],
  profile: ["json"],
};

export interface StudioHost {
  // --- 操作ウィンドウ ---
  /** OS のダイアログでファイルを選ぶ。やめたら `null`。選んだパスはコマンドが読める (src-tauri/src/access.rs)。 */
  pickFile(kind: FileKind, filterName: string): Promise<string | null>;
  /** OS のダイアログで保存先を選ぶ (プロファイル)。拡張子 `.json` が無ければ付ける。やめたら `null`。選んだパスは `saveProfile` で 1 回だけ書ける。 */
  pickSavePath(filterName: string, defaultName: string): Promise<string | null>;
  // プロファイル (コマンドはどれも、失敗すると commandMessage.ts の `CommandMessage` を投げる)
  restoreProfile(): Promise<void>;
  newProfile(): Promise<void>;
  openProfile(path: string): Promise<void>;
  /** 保存する。`path` を省くと今の保存先 (無ければ何もせず `null`)。保存先を返す。 */
  saveProfile(path?: string): Promise<string | null>;
  forgetRecentProfile(path: string): Promise<void>;
  setOutputPreset(preset: string): Promise<void>;
  // シーン
  addScene(name: string): Promise<string>;
  duplicateScene(id: string, name: string): Promise<string>;
  moveScene(id: string, index: number): Promise<void>;
  removeScene(id: string): Promise<void>;
  updateScene(scene: SceneSettings): Promise<void>;
  activateScene(id: string): Promise<void>;
  /** 出しているシーンのポーズで、要素 `part` をそのグループの表示する要素にし、シーンに覚える (T0842)。 */
  showPose(part: string): Promise<void>;
  subscribeStatus(onMessage: (message: MonitorMessage) => void): Promise<void>;
  setParameter(id: string, value: number): Promise<boolean>;
  resetParameters(): Promise<void>;
  outputState(): Promise<OutputState>;
  setOutputVisible(visible: boolean): Promise<OutputState>;
  setOutputOnTop(onTop: boolean): Promise<OutputState>;
  setOutputSize(width: number, height: number): Promise<OutputState>;
  /** 刻みの速さ (30 / 60 / 120 回/秒)。 */
  tickRate(): Promise<number>;
  /** 刻みの速さを選ぶ (プロファイルに持つ)。選べない値なら例外。 */
  setTickRate(rate: number): Promise<number>;
  /** 使う GPU の様子 (T0860)。 */
  gpuSettings(): Promise<GpuView>;
  /** 使う GPU を選ぶ (機械ごとの設定ファイルに覚える。出力ウィンドウは次の起動から、仮想カメラは次に始めるときから)。 */
  setGpuPreference(preference: GpuPreference): Promise<GpuView>;
  /** Windows の「グラフィックの設定」を開く (ほかの OS では例外)。 */
  openGraphicsSettings(): Promise<void>;
  /** 出力ウィンドウとブラウザソースの描く解像度の倍率を選ぶ (1 / 0.75 / 0.5。プロファイルに持つ。T0858)。選べない値なら例外。 */
  setRenderScale(scale: number): Promise<void>;
  /**
   * GPU の優先度を選ぶ (機械ごとの設定ファイルに覚え、すぐに当てる。Windows だけ。ほかの OS では例外。T0861)。
   * 当てられなかった理由は戻り値の `priorityErrors`。
   */
  setGpuPriority(priority: GpuPriority): Promise<GpuView>;
  /** OS の言語 (BCP 47)。取れなければ `null`。 */
  systemLocale(): Promise<string | null>;
  // 入力 (T0803)
  /** 刻みごとに `onPulse` を促しの番号で呼ぶ (入力の処理。ウィンドウが隠れても止まらない)。 */
  subscribePulse(onPulse: (seq: number) => void): Promise<void>;
  /**
   * トラッキングの入力 (`null` で外す)。`pulse` は検出を始めた促しの番号 (バックエンドが促しの時刻を寄せるのに使う。
   * 促しで回していないときは `null`。T0854)。
   */
  setTracking(frame: TrackingFrame | null, pulse: number | null): Promise<void>;
  setTrackingPaused(paused: boolean): Promise<void>;
  setAutoEyeBlink(enabled: boolean): Promise<void>;
  /** マイクの音量 (0〜1。`null` で口パクを止める)。 */
  setLipSyncLevel(level: number | null): Promise<void>;
  setLipSyncPaused(paused: boolean): Promise<void>;
  /** モーションを始める。優先度が低くて始まらなければ `false`。 */
  playMotion(name: string, priority: MotionPriority): Promise<boolean>;
  stopMotions(): Promise<void>;
  setExpression(name: string): Promise<void>;
  addExpression(name: string): Promise<void>;
  removeExpression(name: string): Promise<void>;
  clearExpressions(): Promise<void>;
  setMotionSettings(settings: MotionSettings): Promise<void>;
  setInputSettings(settings: InputSettings): Promise<void>;
  setHotkeys(hotkeys: Hotkey[]): Promise<void>;
  /** 割り当ての画面でキーを押している間、ホットキーの登録を外す。 */
  suspendHotkeys(suspended: boolean): Promise<void>;
  /** モジュールが付くスロットの名前。 */
  moduleSlot(path: string): Promise<string>;
  /** 仮想カメラの今の状態 (以後の変化は `subscribeStatus` の `virtualCamera` で届く)。 */
  virtualCameraState(): Promise<VirtualCameraStatus>;
  /** 仮想カメラを、プロファイルの設定で始める (動いていれば始め直す)。 */
  startVirtualCamera(): Promise<void>;
  stopVirtualCamera(): Promise<void>;
  /**
   * 仮想カメラの設定を変える (プロファイルに持つ)。動いていれば効かせる
   * (解像度とフレームレートは始め直し、色は次の画から)。
   */
  setVirtualCameraSettings(settings: VirtualCameraProfile): Promise<void>;
  // 操作 API (T0805。様子は `api` の通知でも届く)
  apiState(): Promise<ApiView>;
  /** 待ち受けを始める・止める (覚える)。 */
  setApiEnabled(enabled: boolean): Promise<ApiView>;
  /** ポートを変える (1024〜65535。待ち受けていれば始め直す)。 */
  setApiPort(port: number): Promise<ApiView>;
  /** トークン (クライアントに入れてもらうために写す)。 */
  apiToken(): Promise<string>;
  /** トークンを作り直す (古いトークンは使えなくなる)。 */
  regenerateApiToken(): Promise<string>;
  /**
   * 承認を待つクライアント `id` を承認する。承認したクライアントの ID (`ApiView.clients` の `id`。承認を待つ間の ID とは違う)
   * を返す。
   */
  approveApiClient(id: string, permission: ApiPermission): Promise<string>;
  denyApiClient(id: string): Promise<void>;
  /** 承認したクライアントの鍵を取り消す (つないでいれば切る)。 */
  revokeApiClient(id: string): Promise<void>;
  /** 承認したクライアントの今の接続を切る (鍵は残す)。 */
  disconnectApiClient(id: string): Promise<void>;
  // ブラウザソース (T0808。様子は `api` の通知の `source` でも届く)
  /** ページを出す・やめる (覚える。やめるとページの接続はすべて閉じる)。 */
  setSourceEnabled(enabled: boolean): Promise<ApiView>;
  /** OBS のブラウザソースに入れる URL (トークンを含む)。 */
  sourceUrl(): Promise<string>;
  /** トークンを作り直す (古い URL は使えなくなる)。新しい URL を返す。 */
  regenerateSourceToken(): Promise<string>;
  // --- 出力ウィンドウ ---
  subscribeFrames(onMessage: (message: StreamMessage) => void): Promise<void>;
  /** シーン `revision` のモデルのバイト列。今のシーンでなければ例外。 */
  sceneModel(revision: number): Promise<Uint8Array>;
  /** シーン `revision` の `index` 番目 (合成の順) のモジュールのバイト列。 */
  sceneModule(revision: number, index: number): Promise<Uint8Array>;
  /** シーン `revision` の背景の画像のバイト列。 */
  sceneBackground(revision: number): Promise<Uint8Array>;
  /** 枠の無い出力ウィンドウを、押しているポインタで動かし始める。 */
  startDragging(): Promise<void>;
  /** 描画の負荷を報告する (操作ウィンドウに出す)。 */
  reportRenderStats(stats: RenderStats): Promise<void>;
  /** 出力ウィンドウが `WebPuppet.create` に渡す GPU の電力の希望 (起動したときの「使う GPU」。T0860)。 */
  outputPowerPreference(): Promise<PowerPreference>;
  /** 今のシーンのファイルを読めなかったことを報告する (操作ウィンドウに出す)。 */
  reportLoadFailure(failure: LoadFailure): Promise<void>;
}

declare global {
  interface Window {
    /** 差し替えた口 (ヘッドレスのブラウザでの確認)。 */
    __UGOE_STUDIO_HOST__?: StudioHost;
  }
}

async function tauriHost(): Promise<StudioHost> {
  const { Channel, invoke } = await import("@tauri-apps/api/core");
  const windowModule = import("@tauri-apps/api/window");
  async function subscribe<T>(command: string, onMessage: (message: T) => void): Promise<void> {
    const channel = new Channel<T>();
    channel.onmessage = safeChannelHandler(onMessage);
    await invoke(command, { channel });
  }
  return {
    // ダイアログはバックエンドが開き、選ばれたパスだけをコマンドが読み書きできるようにする (src-tauri/src/access.rs。T0838)
    pickFile: (kind, filterName) =>
      invoke("open_file_dialog", { filterName, extensions: FILE_EXTENSIONS[kind] }),
    pickSavePath: (filterName, defaultName) =>
      invoke("save_file_dialog", { filterName, defaultName }),
    restoreProfile: () => invoke("restore_profile"),
    newProfile: () => invoke("new_profile"),
    openProfile: (path) => invoke("open_profile", { path }),
    saveProfile: (path) => invoke("save_profile", { path: path ?? null }),
    forgetRecentProfile: (path) => invoke("forget_recent_profile", { path }),
    setOutputPreset: (preset) => invoke("set_output_preset", { preset }),
    addScene: (name) => invoke("add_scene", { name }),
    duplicateScene: (id, name) => invoke("duplicate_scene", { id, name }),
    moveScene: (id, index) => invoke("move_scene", { id, index }),
    removeScene: (id) => invoke("remove_scene", { id }),
    updateScene: (scene) => invoke("update_scene", { scene }),
    activateScene: (id) => invoke("activate_scene", { id }),
    showPose: (part) => invoke("show_pose", { part }),
    subscribeStatus: (onMessage) => subscribe("subscribe_status", onMessage),
    setParameter: (id, value) => invoke("set_parameter", { id, value }),
    resetParameters: () => invoke("reset_parameters"),
    outputState: () => invoke("output_state"),
    setOutputVisible: (visible) => invoke("set_output_visible", { visible }),
    setOutputOnTop: (onTop) => invoke("set_output_on_top", { onTop }),
    setOutputSize: (width, height) => invoke("set_output_size", { width, height }),
    tickRate: () => invoke("tick_rate"),
    setTickRate: (rate) => invoke("set_tick_rate", { rate }),
    gpuSettings: () => invoke("gpu_settings"),
    setGpuPreference: (preference) => invoke("set_gpu_preference", { preference }),
    openGraphicsSettings: () => invoke("open_graphics_settings"),
    setRenderScale: (scale) => invoke("set_render_scale", { scale }),
    setGpuPriority: (priority) => invoke("set_gpu_priority", { priority }),
    async systemLocale() {
      const { locale } = await import("@tauri-apps/plugin-os");
      return locale();
    },
    subscribePulse: (onPulse) => subscribe("subscribe_pulse", onPulse),
    setTracking: (frame, pulse) => invoke("set_tracking", { frame, pulse }),
    setTrackingPaused: (paused) => invoke("set_tracking_paused", { paused }),
    setAutoEyeBlink: (enabled) => invoke("set_auto_eye_blink", { enabled }),
    setLipSyncLevel: (level) => invoke("set_lip_sync_level", { level }),
    setLipSyncPaused: (paused) => invoke("set_lip_sync_paused", { paused }),
    playMotion: (name, priority) => invoke("play_motion", { name, priority }),
    stopMotions: () => invoke("stop_motions"),
    setExpression: (name) => invoke("set_expression", { name }),
    addExpression: (name) => invoke("add_expression", { name }),
    removeExpression: (name) => invoke("remove_expression", { name }),
    clearExpressions: () => invoke("clear_expressions"),
    setMotionSettings: (settings) => invoke("set_motion_settings", { settings }),
    setInputSettings: (settings) => invoke("set_input_settings", { settings }),
    setHotkeys: (hotkeys) => invoke("set_hotkeys", { hotkeys }),
    suspendHotkeys: (suspended) => invoke("suspend_hotkeys", { suspended }),
    moduleSlot: (path) => invoke("module_slot", { path }),
    virtualCameraState: () => invoke("virtual_camera_state"),
    startVirtualCamera: () => invoke("start_virtual_camera"),
    stopVirtualCamera: () => invoke("stop_virtual_camera"),
    setVirtualCameraSettings: (settings) => invoke("set_virtual_camera_settings", { settings }),
    apiState: () => invoke("api_state"),
    setApiEnabled: (enabled) => invoke("set_api_enabled", { enabled }),
    setApiPort: (port) => invoke("set_api_port", { port }),
    apiToken: () => invoke("api_token"),
    regenerateApiToken: () => invoke("regenerate_api_token"),
    approveApiClient: (id, permission) => invoke("approve_api_client", { id, permission }),
    denyApiClient: (id) => invoke("deny_api_client", { id }),
    revokeApiClient: (id) => invoke("revoke_api_client", { id }),
    disconnectApiClient: (id) => invoke("disconnect_api_client", { id }),
    setSourceEnabled: (enabled) => invoke("set_source_enabled", { enabled }),
    sourceUrl: () => invoke("source_url"),
    regenerateSourceToken: () => invoke("regenerate_source_token"),
    subscribeFrames: (onMessage) => subscribe("subscribe_frames", onMessage),
    async sceneModel(revision) {
      return new Uint8Array(await invoke<ArrayBuffer>("scene_model", { revision }));
    },
    async sceneModule(revision, index) {
      return new Uint8Array(await invoke<ArrayBuffer>("scene_module", { revision, index }));
    },
    async sceneBackground(revision) {
      return new Uint8Array(await invoke<ArrayBuffer>("scene_background", { revision }));
    },
    async startDragging() {
      // 押した直後に呼ばれる (ボタンを放す前に OS へ渡す必要がある)。モジュールは先に読み込んでおく
      const { getCurrentWindow } = await windowModule;
      await getCurrentWindow().startDragging();
    },
    reportRenderStats: (stats) => invoke("report_render_stats", { stats }),
    outputPowerPreference: () => invoke("output_power_preference"),
    reportLoadFailure: (failure) => invoke("report_load_failure", { failure }),
  };
}

let pending: Promise<StudioHost> | undefined;

/** 使う口。差し替えが無く Tauri の中なら Tauri の口。 */
export function studioHost(): Promise<StudioHost> {
  pending ??= (async () => {
    const replaced = window.__UGOE_STUDIO_HOST__;
    if (replaced !== undefined) return replaced;
    if ("__TAURI_INTERNALS__" in window) return tauriHost();
    // i18n-ignore: 開発者向け (Tauri の外で開いたとき)
    throw new Error("ugoe studio runs inside its Tauri app (or set window.__UGOE_STUDIO_HOST__)");
  })();
  return pending;
}
