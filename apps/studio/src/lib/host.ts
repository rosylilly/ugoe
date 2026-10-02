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

/** 操作ウィンドウへの通知 (lib.rs の `Monitor`)。 */
export type MonitorMessage =
  | { kind: "scene"; scene: SceneMessage; parameters: ParameterInfo[] }
  | { kind: "status"; seq: number; values: number[]; tickMs: number; outputs: number }
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
  /** OS の言語 (BCP 47)。取れなければ `null`。 */
  systemLocale(): Promise<string | null>;
  // --- 出力ウィンドウ ---
  subscribeFrames(onMessage: (message: StreamMessage) => void): Promise<void>;
  /** シーン `revision` のモデルのバイト列。今のシーンでなければ例外。 */
  sceneModel(revision: number): Promise<Uint8Array>;
  /** 枠の無い出力ウィンドウを、押しているポインタで動かし始める。 */
  startDragging(): Promise<void>;
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
