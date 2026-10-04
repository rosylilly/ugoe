// バックエンド (src-tauri/src/lib.rs) との口。Tauri の外 (ヘッドレスのブラウザでの確認) では
// `window.__UGOE_STUDIO_HOST__` に置いたものを使う (README.md#ヘッドレスのブラウザで確かめる)。
import type { TrackingFrame } from "@ugoe/face-tracking";
import type { LightingSettings, MotionPriority, SceneMessage } from "@ugoe/player";
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
import {
  type Background,
  type ProfileView,
  type SceneSettings,
  type Transform,
  withExtension,
} from "./profile";
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
}

/** 出力が報告する描画の負荷 (lib.rs の `RenderStats`。直近の約 1 秒)。 */
export interface RenderStats {
  /** 描いた回数 (回/秒)。 */
  fps: number;
  /** 1 回の描画 (届いたフレームの適用と描画) の CPU の時間の平均 (ミリ秒)。 */
  renderMs: number;
  /** 1 回の描画の CPU の時間の最大 (ミリ秒)。 */
  renderMaxMs: number;
  /** 溢れて捨てたフレームの数 (受け手を作ってからの合計)。 */
  dropped: number;
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
  renderers: RendererLoad[];
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
}

/** 操作ウィンドウへの通知 (lib.rs の `Monitor`)。 */
export type MonitorMessage =
  | { kind: "scene"; scene: StudioScene; parameters: ParameterInfo[]; usesLighting: boolean }
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
  | ({ kind: "api" } & ApiView)
  | {
      kind: "loadFailures";
      /** 受け手が今のシーンで読めなかったこと (無ければ空。シーンが変わると空になる)。 */
      failures: ReceiverFailure[];
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
  /** OS のダイアログでファイルを選ぶ。やめたら `null`。 */
  pickFile(kind: FileKind, filterName: string): Promise<string | null>;
  /** OS のダイアログで保存先を選ぶ (プロファイル)。拡張子 `.json` が無ければ付ける。やめたら `null`。 */
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
  /** OS の言語 (BCP 47)。取れなければ `null`。 */
  systemLocale(): Promise<string | null>;
  // 入力 (T0803)
  /** 刻みごとに `onPulse` を呼ぶ (入力の処理。ウィンドウが隠れても止まらない)。 */
  subscribePulse(onPulse: () => void): Promise<void>;
  /** トラッキングの入力 (`null` で外す)。 */
  setTracking(frame: TrackingFrame | null): Promise<void>;
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
  approveApiClient(id: string, permission: ApiPermission): Promise<void>;
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
    async pickFile(kind, filterName) {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{ name: filterName, extensions: FILE_EXTENSIONS[kind] }],
      });
      return typeof path === "string" ? path : null;
    },
    async pickSavePath(filterName, defaultName) {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({
        defaultPath: defaultName,
        filters: [{ name: filterName, extensions: ["json"] }],
      });
      // Linux のダイアログはフィルタの拡張子を付けずに返すことがある。
      return path && withExtension(path, "json");
    },
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
    subscribeStatus: (onMessage) => subscribe("subscribe_status", onMessage),
    setParameter: (id, value) => invoke("set_parameter", { id, value }),
    resetParameters: () => invoke("reset_parameters"),
    outputState: () => invoke("output_state"),
    setOutputVisible: (visible) => invoke("set_output_visible", { visible }),
    setOutputOnTop: (onTop) => invoke("set_output_on_top", { onTop }),
    setOutputSize: (width, height) => invoke("set_output_size", { width, height }),
    tickRate: () => invoke("tick_rate"),
    setTickRate: (rate) => invoke("set_tick_rate", { rate }),
    async systemLocale() {
      const { locale } = await import("@tauri-apps/plugin-os");
      return locale();
    },
    subscribePulse: (onPulse) => subscribe("subscribe_pulse", onPulse),
    setTracking: (frame) => invoke("set_tracking", { frame }),
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
