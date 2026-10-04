// 仮想カメラの設定と状態 (docs/studio.md#仮想カメラ、src-tauri/src/virtual_camera.rs)。
// 設定はプロファイルの `output.virtualCamera` に持つ (T0827。lib/profile.ts)。

/** 仮想カメラの設定 (virtual_camera.rs の `Settings`。状態の通知に載る)。 */
export interface VirtualCameraSettings {
  width: number;
  height: number;
  /** フレームレート (30 / 60)。 */
  fps: number;
  /** 透過のところを埋める色 (0〜255 の RGB)。 */
  background: [number, number, number];
}

/** 仮想カメラの状態 (virtual_camera.rs の `Status`)。 */
export type VirtualCameraStatus =
  /** 止めている。`unavailable` は始められない理由のコード (`null` なら始められる)。 */
  | { state: "stopped"; unavailable: string | null }
  | { state: "starting"; settings: VirtualCameraSettings }
  | {
      state: "running";
      settings: VirtualCameraSettings;
      /** 書いている先 (Linux は `/dev/videoN`、Windows はカメラの名前)。 */
      device: string;
      /** 受け取るアプリがいる (Linux では分からないので、いつも `true`)。 */
      consumer: boolean;
      /** 描画がフレームレートに追いついていない。 */
      late: boolean;
    }
  | { state: "failed"; settings: VirtualCameraSettings; code: string; detail: string };

/** 選べる解像度 (ugoe-studio-vcam の `RESOLUTIONS`)。 */
export const CAMERA_RESOLUTIONS: readonly (readonly [number, number])[] = [
  [1920, 1080],
  [1280, 720],
];

/** 選べるフレームレート (ugoe-studio-vcam の `FRAME_RATES`)。 */
export const CAMERA_FRAME_RATES: readonly number[] = [30, 60];

/** プロファイルに持つ設定 (profile.rs の `VirtualCameraSettings`。色は `#rrggbb`)。 */
export interface VirtualCameraProfile {
  width: number;
  height: number;
  /** フレームレート (30 / 60)。 */
  fps: number;
  /** 透過のところを埋める色 (`#rrggbb`)。 */
  background: string;
}

/** 既定の設定 (virtual_camera.rs の `Settings::default`)。 */
export const DEFAULT_CAMERA_PROFILE: VirtualCameraProfile = {
  width: 1280,
  height: 720,
  fps: 30,
  background: "#f0f0f0",
};

/** 動いている (始めている途中を含む) か。 */
export function isCameraActive(status: VirtualCameraStatus): boolean {
  return status.state === "running" || status.state === "starting";
}

/**
 * 操作ウィンドウに出す知らせ。文言は呼び手が翻訳する (`virtualCamera.notices.<kind>`・`virtualCamera.errors.<code>`)。
 * `guide` は案内を出す OS の手順 (`linux` = v4l2loopback の入れ方、`windows` = Windows 11 と OBS の代わりの手順)。
 */
export type CameraNotice =
  | { kind: "unavailable"; tone: "warning"; code: string; guide: CameraGuide | null }
  | { kind: "failed"; tone: "danger"; code: string; detail: string; guide: CameraGuide | null }
  | { kind: "noConsumer"; tone: "info" }
  | { kind: "late"; tone: "warning" };

export type CameraGuide = "linux" | "windows" | "reinstall";

/** 理由のコードに合う案内。 */
export function guideFor(code: string): CameraGuide | null {
  switch (code) {
    case "noDevice":
    case "busy":
      return "linux";
    case "windowsTooOld":
      return "windows";
    case "notRegistered":
      return "reinstall";
    default:
      return null;
  }
}

/** 今の状態から出す知らせ (止まった・遅れた・受け取るアプリがいない)。 */
export function cameraNotices(status: VirtualCameraStatus): CameraNotice[] {
  switch (status.state) {
    case "stopped":
      return status.unavailable === null
        ? []
        : [
            {
              kind: "unavailable",
              tone: "warning",
              code: status.unavailable,
              guide: guideFor(status.unavailable),
            },
          ];
    case "failed":
      return [
        {
          kind: "failed",
          tone: "danger",
          code: status.code,
          detail: status.detail,
          guide: guideFor(status.code),
        },
      ];
    case "running": {
      const out: CameraNotice[] = [];
      if (!status.consumer) out.push({ kind: "noConsumer", tone: "info" });
      if (status.late) out.push({ kind: "late", tone: "warning" });
      return out;
    }
    case "starting":
      return [];
  }
}
