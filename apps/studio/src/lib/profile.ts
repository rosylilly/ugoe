// プロファイルとシーンの形 (src-tauri/src/profile.rs・stage.rs と同じ。docs/studio.md#プロファイルの形式) と、
// 画面で使う小さな関数 (シーンの表示名・エラーの文・背景や配置の既定値)。
import { defaultLighting, type LightingSettings, sanitizeLighting } from "@ugoe/player";
import type { Hotkey, InputSettings, MotionSettings } from "./inputs";

/** モデルの位置と大きさと回転 (profile.rs の `Transform`)。位置は出力の大きさに対する割合。 */
export interface Transform {
  x: number;
  y: number;
  scale: number;
  /** 度。時計回りが正。 */
  rotation: number;
}

/** 背景 (profile.rs の `Background`)。 */
export type Background =
  | { kind: "transparent" }
  | { kind: "color"; color: string }
  | { kind: "chroma"; color: string }
  | { kind: "image"; path: string };

export type BackgroundKind = Background["kind"];

/** シーン (profile.rs の `SceneSettings`)。 */
export interface SceneSettings {
  id: string;
  name: string;
  /** モデルの絶対パス。まだ選んでいなければ `null`。 */
  model: string | null;
  /** 付けるモジュールの絶対パス (合成の順)。 */
  modules: string[];
  /** 光源の設定。`null` は既定の光源。 */
  lighting: LightingSettings | null;
  transform: Transform;
  background: Background;
}

/** プロファイル (profile.rs の `Profile`)。 */
export interface Profile {
  format: "ugoe-studio-profile";
  version: number;
  scenes: SceneSettings[];
  activeScene: string | null;
  output: { size: string };
  /** ホットキーの割り当て (T0803)。 */
  hotkeys: Hotkey[];
  /** 入力の機器と調整 (T0803)。 */
  input: InputSettings;
  /** 待機モーションとトラッキング中のモーションの優先 (T0803)。 */
  motion: MotionSettings;
}

/** 操作ウィンドウに届くプロファイルの状態 (stage.rs の `ProfileView`)。 */
export interface ProfileView {
  profile: Profile;
  /** 保存先。まだ保存していなければ `null`。 */
  path: string | null;
  /** 保存した後に変えたか。 */
  dirty: boolean;
  /** 指揮役に出しているシーンの ID。 */
  live: string | null;
  /** 最近のプロファイル (新しいものが先頭)。 */
  recent: string[];
}

/** シーンのファイルの種類 (stage.rs の `Asset`)。 */
export type Asset = "model" | "module" | "background";

/** バックエンドのエラー (profile.rs の `ProfileError`・stage.rs の `SceneError`)。`code` で文言を選ぶ。 */
export type StudioError =
  | { code: "io"; path: string; message: string }
  | { code: "tooLarge"; path: string; bytes: number; asset?: Asset }
  | { code: "notJson"; path: string; message: string }
  | { code: "notProfile"; path: string }
  | { code: "futureVersion"; path: string; version: number; supported: number }
  | { code: "invalid"; path: string; message: string }
  | { code: "unknownScene"; id: string }
  | { code: "missing"; asset: Asset; path: string }
  | { code: "unreadable"; asset: Asset; path: string; message: string }
  | { code: "wrongType"; asset: Asset; path: string }
  | { code: "dressUp"; message: string }
  | { code: "noModel" }
  | { code: "unknownMotion"; name: string }
  | { code: "unknownExpression"; name: string }
  | { code: "motionRejected" };

const CODES = new Set<string>([
  "io",
  "tooLarge",
  "notJson",
  "notProfile",
  "futureVersion",
  "invalid",
  "unknownScene",
  "missing",
  "unreadable",
  "wrongType",
  "dressUp",
  "noModel",
  "unknownMotion",
  "unknownExpression",
  "motionRejected",
]);

/** バックエンドのエラーか (Tauri の invoke はエラーの値をそのまま投げる)。 */
export function isStudioError(value: unknown): value is StudioError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof value.code === "string" &&
    CODES.has(value.code)
  );
}

/** 位置の範囲 (出力の大きさに対する割合。profile.rs の `POSITION_LIMIT`)。 */
export const POSITION_LIMIT = 2;
/** 大きさの範囲 (profile.rs の `SCALE_RANGE`)。 */
export const SCALE_RANGE = [0.05, 20] as const;
/** 回転の範囲 (度。profile.rs の `ROTATION_LIMIT` より狭くしてよい。スライダーの範囲)。 */
export const ROTATION_RANGE = [-180, 180] as const;

/** 既定の配置 (中央・出力に合わせた大きさ・回転なし)。 */
export const DEFAULT_TRANSFORM: Transform = { x: 0, y: 0, scale: 1, rotation: 0 };

/** 背景の種類を替えたときの既定 (クロマキーは緑、単色は黒)。 */
export const DEFAULT_COLORS = { color: "#000000", chroma: "#00ff00" } as const;

/** クロマキーで選べる色 (緑・青)。 */
export const CHROMA_COLORS = ["#00ff00", "#0000ff"] as const;

/** 背景の種類を `kind` にする。同じ種類なら色を保ち、違えば既定の色。画像は `path` (省くと今の背景のまま)。 */
export function withBackgroundKind(
  current: Background,
  kind: BackgroundKind,
  path?: string,
): Background {
  switch (kind) {
    case "transparent":
      return { kind };
    case "color":
    case "chroma":
      return { kind, color: current.kind === kind ? current.color : DEFAULT_COLORS[kind] };
    case "image":
      return path === undefined ? current : { kind, path };
  }
}

/** パスの最後の名前 (`/a/b/avatar.ugoe` → `avatar.ugoe`)。Windows の区切りも扱う。 */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** シーンの光源の設定 (`null` なら既定の光源)。範囲の外の値は収める。 */
export function sceneLighting(scene: Pick<SceneSettings, "lighting">): LightingSettings {
  return scene.lighting === null ? defaultLighting() : sanitizeLighting(scene.lighting);
}

/** シーンの写し (画面で書き換えてから `updateScene` に渡す)。 */
export function cloneScene(scene: SceneSettings): SceneSettings {
  return structuredClone(scene);
}
