// バックエンド (src-tauri/src/lib.rs) との口。Tauri の外 (ヘッドレスのブラウザでの確認) では
// `window.__UGOE_STUDIO_HOST__` に置いたものを使う (README.md#ヘッドレスのブラウザで確かめる)。
import type { LightingSettings, SceneMessage } from "@ugoe/player";
import type { Background, ProfileView, SceneSettings, Transform } from "./profile";

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
  | ({ kind: "output" } & OutputState);

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
  /** OS のダイアログで保存先を選ぶ (プロファイル)。やめたら `null`。 */
  pickSavePath(filterName: string, defaultName: string): Promise<string | null>;
  // プロファイル (エラーは profile.ts の `StudioError` を投げる)
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
  /** 刻みの速さを選ぶ。選べない値なら例外。 */
  setTickRate(rate: number): Promise<number>;
  /** OS の言語 (BCP 47)。取れなければ `null`。 */
  systemLocale(): Promise<string | null>;
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
}

declare global {
  interface Window {
    /** 差し替えた口 (ヘッドレスのブラウザでの確認)。 */
    __UGOE_STUDIO_HOST__?: StudioHost;
  }
}

async function tauriHost(): Promise<StudioHost> {
  const { Channel, invoke } = await import("@tauri-apps/api/core");
  async function subscribe<T>(command: string, onMessage: (message: T) => void): Promise<void> {
    const channel = new Channel<T>();
    channel.onmessage = onMessage;
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
      return save({
        defaultPath: defaultName,
        filters: [{ name: filterName, extensions: ["json"] }],
      });
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
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().startDragging();
    },
    reportRenderStats: (stats) => invoke("report_render_stats", { stats }),
  };
}

let pending: Promise<StudioHost> | undefined;

/** 使う口。差し替えが無く Tauri の中なら Tauri の口。 */
export function studioHost(): Promise<StudioHost> {
  pending ??= (async () => {
    const replaced = window.__UGOE_STUDIO_HOST__;
    if (replaced !== undefined) return replaced;
    if ("__TAURI_INTERNALS__" in window) return tauriHost();
    throw new Error("ugoe studio runs inside its Tauri app (or set window.__UGOE_STUDIO_HOST__)");
  })();
  return pending;
}
