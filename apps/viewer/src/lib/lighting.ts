// ライティングの光源の設定 (T0705)。画面で扱う形 (向きは角度、点光源の位置はキャンバスに対する割合) と、
// `WebPuppet.set_lighting_json` に渡す JSON (キャンバス座標) の変換、既定の光源の組、localStorage への保存
// (最後に使った設定と、モデルごとの設定 (T0709))。
// 光源の意味は docs/spec/rendering.md#ライティング。
import { parseHexColor } from "./color";
import { ModelMemory } from "./modelMemory";

/** 色と強さ。渡す色は `color × intensity` (1 を超えてよい)。 */
export interface LightColor {
  /** `#rrggbb`。 */
  color: string;
  intensity: number;
}

/** 平行光源。向きは光が来る側で表す。 */
export interface DirectionalSettings extends LightColor {
  enabled: boolean;
  /** 画面での光が来る方向 (度)。0 = 右、90 = 上、180 = 左、−90 = 下。 */
  azimuth: number;
  /** キャンバスの面からの高さ (度)。0 = 真横 (面に沿う)、90 = 真正面 (手前) から。 */
  elevation: number;
}

/** 点光源。位置と範囲はキャンバスの大きさに対する割合で、モデルが変わっても同じ見え方になる。 */
export interface PointSettings extends LightColor {
  enabled: boolean;
  /** キャンバスの幅に対する位置 (0 = 左端、1 = 右端)。 */
  x: number;
  /** キャンバスの高さに対する位置 (0 = 上端、1 = 下端)。 */
  y: number;
  /** 手前への距離 (キャンバスの長い辺に対する割合)。 */
  z: number;
  /** 光が届く距離 (キャンバスの長い辺に対する割合)。 */
  range: number;
}

export interface LightingSettings {
  /** 光源を渡すか。切ると既定 (環境光が白で光源なし。ライティングを受けない場合と同じ色) に戻す。 */
  enabled: boolean;
  ambient: LightColor;
  directional: DirectionalSettings;
  point: PointSettings;
}

/** スライダーの範囲。読み込んだ値もこの範囲に収める。 */
export const LIMITS = {
  intensity: [0, 3],
  azimuth: [-180, 180],
  elevation: [0, 90],
  x: [-0.5, 1.5],
  y: [-0.5, 1.5],
  z: [0, 2],
  range: [0.1, 4],
} as const satisfies Record<string, readonly [number, number]>;

export interface LightingPreset {
  id: string;
  name: string;
  settings: LightingSettings;
}

const OFF_POINT: PointSettings = {
  enabled: false,
  color: "#ffffff",
  intensity: 1,
  x: 0.3,
  y: 0.3,
  z: 0.4,
  range: 1.5,
};

/** 既定の光源の組。先頭が、はじめて開いたときの設定。 */
export const PRESETS: readonly LightingPreset[] = [
  {
    id: "standard",
    name: "標準",
    settings: {
      enabled: true,
      ambient: { color: "#ffffff", intensity: 0.55 },
      directional: {
        enabled: true,
        color: "#ffffff",
        intensity: 0.6,
        azimuth: 135,
        elevation: 45,
      },
      point: OFF_POINT,
    },
  },
  {
    id: "sunset",
    name: "夕日",
    settings: {
      enabled: true,
      ambient: { color: "#8c9bd0", intensity: 0.45 },
      directional: {
        enabled: true,
        color: "#ffb070",
        intensity: 1,
        azimuth: 10,
        elevation: 20,
      },
      point: OFF_POINT,
    },
  },
  {
    id: "lamp",
    name: "夜のランプ",
    settings: {
      enabled: true,
      ambient: { color: "#5a6aa0", intensity: 0.3 },
      directional: {
        enabled: false,
        color: "#ffffff",
        intensity: 0.6,
        azimuth: 135,
        elevation: 45,
      },
      point: {
        enabled: true,
        color: "#ffd9a0",
        intensity: 1.6,
        x: 0.25,
        y: 0.3,
        z: 0.35,
        range: 1.4,
      },
    },
  },
];

/** はじめて開いたとき (と保存した値が読めないとき) の設定。 */
export function defaultLighting(): LightingSettings {
  return structuredClone(PRESETS[0]?.settings) as LightingSettings;
}

/** `set_lighting_json` に渡す JSON の形 (crates/ugoe-wasm/src/lighting.rs)。 */
export type LightingJson = {
  ambient: [number, number, number];
  lights: (
    | { type: "directional"; direction: [number, number, number]; color: [number, number, number] }
    | {
        type: "point";
        position: [number, number, number];
        color: [number, number, number];
        range: number;
      }
  )[];
};

function scaled({ color, intensity }: LightColor): [number, number, number] {
  const rgb = parseHexColor(color) ?? [1, 1, 1];
  return rgb.map((c) => c * intensity) as [number, number, number];
}

/**
 * 光が来る側の単位ベクトル (キャンバス座標。Y 下向き、手前が +z)。
 * `direction` (光が進む向き) はこの逆。
 */
export function towardLight(azimuth: number, elevation: number): [number, number, number] {
  const az = (azimuth * Math.PI) / 180;
  const el = (elevation * Math.PI) / 180;
  const flat = Math.cos(el);
  return [flat * Math.cos(az), -flat * Math.sin(az), Math.sin(el)];
}

/**
 * 設定を `set_lighting_json` の JSON にする。`canvas` はモデルのキャンバスの大きさ `[幅, 高さ]`。
 * 切ってあれば `null` (既定に戻す)。
 */
export function toLightingJson(
  settings: LightingSettings,
  canvas: readonly [number, number],
): LightingJson | null {
  if (!settings.enabled) return null;
  const json: LightingJson = { ambient: scaled(settings.ambient), lights: [] };
  const d = settings.directional;
  if (d.enabled) {
    const [x, y, z] = towardLight(d.azimuth, d.elevation);
    json.lights.push({ type: "directional", direction: [-x, -y, -z], color: scaled(d) });
  }
  const p = settings.point;
  if (p.enabled) {
    const [w, h] = canvas;
    const size = Math.max(w, h, 1);
    json.lights.push({
      type: "point",
      position: [p.x * w, p.y * h, p.z * size],
      color: scaled(p),
      range: Math.max(p.range, LIMITS.range[0]) * size,
    });
  }
  return json;
}

// --- 保存 ---

const STORAGE_KEY = "ugoe-viewer.lighting";
const STORAGE_VERSION = 1;

function clamp(value: unknown, [min, max]: readonly [number, number], fallback: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return fallback;
  return Math.min(max, Math.max(min, value));
}

function color(value: unknown, fallback: string): string {
  return typeof value === "string" && parseHexColor(value) !== null && value.startsWith("#")
    ? value.toLowerCase()
    : fallback;
}

function bool(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function record(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
}

function lightColor(value: unknown, fallback: LightColor): LightColor {
  const v = record(value);
  return {
    color: color(v.color, fallback.color),
    intensity: clamp(v.intensity, LIMITS.intensity, fallback.intensity),
  };
}

/** 読んだ値を設定にする。足りない・壊れた値は既定値で埋め、範囲の外の値は収める。 */
export function sanitizeLighting(value: unknown): LightingSettings {
  const base = defaultLighting();
  const v = record(value);
  const d = record(v.directional);
  const p = record(v.point);
  return {
    enabled: bool(v.enabled, base.enabled),
    ambient: lightColor(v.ambient, base.ambient),
    directional: {
      ...lightColor(d, base.directional),
      enabled: bool(d.enabled, base.directional.enabled),
      azimuth: clamp(d.azimuth, LIMITS.azimuth, base.directional.azimuth),
      elevation: clamp(d.elevation, LIMITS.elevation, base.directional.elevation),
    },
    point: {
      ...lightColor(p, base.point),
      enabled: bool(p.enabled, base.point.enabled),
      x: clamp(p.x, LIMITS.x, base.point.x),
      y: clamp(p.y, LIMITS.y, base.point.y),
      z: clamp(p.z, LIMITS.z, base.point.z),
      range: clamp(p.range, LIMITS.range, base.point.range),
    },
  };
}

/** 保存した設定を読む。無い・読めない (壊れた JSON、使えない localStorage) なら既定の設定。 */
export function loadLighting(storage?: Storage): LightingSettings {
  try {
    // localStorage を読むだけで例外になる環境 (保存を禁止したブラウザ) もあるので、try の中で引く。
    const text = (storage ?? globalThis.localStorage)?.getItem(STORAGE_KEY);
    if (text == null) return defaultLighting();
    const saved = record(JSON.parse(text));
    if (saved.version !== STORAGE_VERSION) return defaultLighting();
    return sanitizeLighting(saved.settings);
  } catch {
    return defaultLighting();
  }
}

/** 設定を保存する。保存できなくても (容量・無効な localStorage) 無視する。 */
export function saveLighting(settings: LightingSettings, storage?: Storage): void {
  try {
    (storage ?? globalThis.localStorage)?.setItem(
      STORAGE_KEY,
      JSON.stringify({ version: STORAGE_VERSION, settings }),
    );
  } catch {
    // 覚えられないだけで、表示には影響しない。
  }
}

// --- モデルごとの設定 (T0709) ---

/** モデルごとの光源の設定の localStorage のキー。入れ物は modelMemory.ts の `ModelMemory`。 */
export const MODEL_LIGHTING_STORAGE_KEY = "ugoe-viewer.lighting.models";
/** 光源の設定を覚えるモデルの数の上限 (1 体で 1 KB 足らず)。超えたら最も長く開いていないものから消す。 */
export const MAX_LIGHTING_MODELS = 50;

/** モデルごとに覚えた設定を読む。オブジェクトでなければ `null` (読み飛ばす)、中の壊れた値は `sanitizeLighting` が直す。 */
export function parseModelLighting(value: unknown): LightingSettings | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? sanitizeLighting(value)
    : null;
}

/** モデルごとの光源の設定の入れ物。 */
export function lightingMemory(storage?: Storage): ModelMemory<LightingSettings> {
  return new ModelMemory(MODEL_LIGHTING_STORAGE_KEY, parseModelLighting, {
    limit: MAX_LIGHTING_MODELS,
    storage,
  });
}

// --- 向きの面 (LightPad) との変換 ---

/** 平行光源の角度を、向きの面の位置 (-1..1、右・下が正。中心が真正面) にする。 */
export function padFromAngles(azimuth: number, elevation: number): [number, number] {
  const [x, y] = towardLight(azimuth, elevation);
  return [x, y];
}

/** 向きの面の位置を平行光源の角度 `[azimuth, elevation]` にする (度、小数 1 桁に丸める)。 */
export function anglesFromPad(x: number, y: number): [number, number] {
  const r = Math.min(1, Math.hypot(x, y));
  const round = (v: number) => Math.round(v * 10) / 10;
  const elevation = round((Math.acos(r) * 180) / Math.PI);
  // 真正面では方位が決まらないので 0 にする。
  const azimuth = r < 1e-6 ? 0 : round((Math.atan2(-y, x) * 180) / Math.PI);
  return [azimuth, elevation];
}

/** 2 つの設定が同じか (フィールドの並びによらない)。どの既定の組を選んでいるかの表示に使う。 */
export function sameLighting(a: LightingSettings, b: LightingSettings): boolean {
  return JSON.stringify(sanitizeLighting(a)) === JSON.stringify(sanitizeLighting(b));
}
