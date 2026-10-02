// 入力・モーション・ホットキーの形 (src-tauri/src/input.rs・hotkeys.rs・animation.rs と同じ。docs/studio.md#入力、T0803)。
import type { MotionInfo, MotionPriority } from "@ugoe/player";

/** 入力の機器 (input.rs の `Device`)。 */
export interface Device {
  id: string;
  label: string;
}

/** 入力の設定 (input.rs の `InputSettings`。プロファイルの `input`)。 */
export interface InputSettings {
  camera: Device | null;
  /** トラッキングの平滑化の強さ (0〜1)。 */
  smoothing: number;
  swapSides: boolean;
  trackBody: boolean;
  stopEyeBlink: boolean;
  microphone: Device | null;
  /** マイクの感度 (0〜1。0.5 で等倍)。 */
  sensitivity: number;
  /** ノイズの閾値 (0〜1)。 */
  noiseGate: number;
}

export const DEFAULT_INPUT: InputSettings = {
  camera: null,
  smoothing: 0.5,
  swapSides: false,
  trackBody: true,
  stopEyeBlink: true,
  microphone: null,
  sensitivity: 0.5,
  noiseGate: 0.3,
};

/** モーションの設定 (input.rs の `MotionSettings`。プロファイルの `motion`)。 */
export interface MotionSettings {
  autoIdle: boolean;
  /** 待機モーションの名前。`null` は名前で選ぶ。 */
  idleMotion: string | null;
  overTracking: boolean;
  trackingKept: string[];
}

export const DEFAULT_MOTION: MotionSettings = {
  autoIdle: true,
  idleMotion: null,
  overTracking: true,
  trackingKept: [],
};

/** ホットキーの操作 (hotkeys.rs の `HotkeyAction`)。 */
export type HotkeyAction =
  | { kind: "scene"; scene: string }
  | { kind: "expression"; expression: string; mode: "set" | "toggle" }
  | { kind: "clearExpressions" }
  | { kind: "motion"; motion: string; priority: MotionPriority }
  | { kind: "stopMotions" }
  | { kind: "dressUp"; slot: string; module: string | null }
  | { kind: "trackingPause" }
  | { kind: "trackingCalibrate" }
  | { kind: "lipSyncPause" }
  | { kind: "unknown" };

export type HotkeyKind = HotkeyAction["kind"];

/** ホットキーの割り当て (hotkeys.rs の `Hotkey`)。`keys` は `Ctrl+Shift+KeyA` の形 (空ならまだ決めていない)。 */
export interface Hotkey {
  keys: string;
  action: HotkeyAction;
}

/** 登録できなかった理由 (hotkeys.rs の `HotkeyProblem`)。 */
export type HotkeyProblem =
  | { code: "unassigned" }
  | { code: "invalidKeys" }
  | { code: "duplicate"; other: number }
  | { code: "registerFailed"; message: string }
  | { code: "unknownAction" };

/** 再生中のモーション (animation.rs の `CurrentMotion`)。 */
export interface CurrentMotion {
  name: string;
  priority: MotionPriority;
}

/** 演技の様子 (animation.rs の `AnimationView`)。 */
export interface AnimationView {
  motions: MotionInfo[];
  expressions: string[];
  current: CurrentMotion | null;
  activeExpressions: string[];
  idleMotion: string | null;
  tracking: boolean;
  trackingPaused: boolean;
  lipSyncPaused: boolean;
  trackingParameters: string[];
  slots: string[];
}

export const EMPTY_ANIMATION: AnimationView = {
  motions: [],
  expressions: [],
  current: null,
  activeExpressions: [],
  idleMotion: null,
  tracking: false,
  trackingPaused: false,
  lipSyncPaused: false,
  trackingParameters: [],
  slots: [],
};
