// バックエンド (src-tauri/src/lib.rs) との口。Tauri の外 (ヘッドレスのブラウザでの確認) では
// `window.__UGOE_STUDIO_HOST__` に置いたものを使う (README.md#ヘッドレスのブラウザで確かめる)。
import type { SceneMessage } from "@ugoe/player";

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

/** 操作ウィンドウへの通知 (lib.rs の `Monitor`)。 */
export type MonitorMessage =
  | { kind: "scene"; scene: SceneMessage; parameters: ParameterInfo[] }
  | StatusMessage
  | ({ kind: "output" } & OutputState);

/** 出力ウィンドウへの配信: シーンの通知 (JSON) かフレームのバイト列 (@ugoe/player の `decodeFrame`)。 */
export type StreamMessage = SceneMessage | ArrayBuffer;

export interface StudioHost {
  // --- 操作ウィンドウ ---
  /** OS のダイアログでモデルのファイルを選ぶ。やめたら `null`。 */
  pickModelPath(filterName: string): Promise<string | null>;
  openModel(path: string): Promise<void>;
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
    async pickModelPath(filterName) {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{ name: filterName, extensions: ["ugoe", "json"] }],
      });
      return typeof path === "string" ? path : null;
    },
    openModel: (path) => invoke("open_model", { path }),
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
      const bytes = await invoke<ArrayBuffer>("scene_model", { revision });
      return new Uint8Array(bytes);
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
