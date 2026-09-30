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

/** 設定の短い名前: 切っていれば「切」、既定の組と同じならその名前、違えば「カスタム」。 */
export function lightingLabel(settings: LightingSettings): string {
  if (!settings.enabled) return "切";
  return PRESETS.find((p) => sameLighting(p.settings, settings))?.name ?? "カスタム";
}

/** 2 つの設定が同じか (フィールドの並びによらない)。どの既定の組を選んでいるかの表示に使う。 */
export function sameLighting(a: LightingSettings, b: LightingSettings): boolean {
  return JSON.stringify(sanitizeLighting(a)) === JSON.stringify(sanitizeLighting(b));
}

// --- ビューポートの光源の印 (T0708) ---

/** キャンバス座標 → 画面座標 (canvas 要素の左上からの CSS ピクセル) の変換。`screen = offset + scale × canvas`。 */
export interface ScreenTransform {
  scale: number;
  x: number;
  y: number;
}

/** 平行光源の向きを選ぶ円 (画面座標)。中心がキャンバスの中央、中心が真正面・縁が真横から来る光を表す。 */
export interface LightDial {
  cx: number;
  cy: number;
  r: number;
  /**
   * 円を画面の内側へ寄せたか (中心がキャンバスの中央を示していない)。寄せたときは見た目を変え、
   * 中心からキャンバスの中央の向きに矢印を出す (T0717)。
   */
  shifted: boolean;
  /** キャンバスの中央 (画面座標)。寄せていなければ `[cx, cy]`。 */
  canvasCenter: [number, number];
}

/** 平行光源の円の半径の範囲 (CSS ピクセル)。拡大縮小に合わせるが、小さすぎて掴めない・大きすぎて見えないことがないように収める。 */
const DIAL_RADIUS = [60, 220] as const;

/** 画面の端に寄せた印と、画面の縁との間 (CSS ピクセル。取っ手の半径より少し大きく)。 */
export const EDGE_MARGIN = 14;

/**
 * `value` を画面の `[margin, size − margin]` に収める。画面が狭すぎて収まらなければ画面の中央。
 * 大きく拡大して印が画面の外に出ても、取っ手を掴めるようにするため (T0714)。
 */
function keepOnScreen(value: number, size: number, margin: number): number {
  if (size < 2 * margin) return size / 2;
  return Math.min(size - margin, Math.max(margin, value));
}

/**
 * 平行光源の円を、表示しているキャンバスの中央に置く。半径はキャンバスの短い辺の 35% (エディタの取っ手と同じ)。
 * `bounds` (ビューポートの大きさ `[幅, 高さ]`) を渡すと、円が画面からはみ出すときは円ごと画面の内側へ寄せる
 * (大きく拡大してキャンバスの中央が画面の外に出ても、取っ手が見えるように。T0714)。
 */
export function lightDial(
  canvas: readonly [number, number],
  view: ScreenTransform,
  bounds?: readonly [number, number],
): LightDial {
  const [w, h] = canvas;
  const r = Math.min(DIAL_RADIUS[1], Math.max(DIAL_RADIUS[0], Math.min(w, h) * view.scale * 0.35));
  const center: [number, number] = [view.x + (view.scale * w) / 2, view.y + (view.scale * h) / 2];
  let [cx, cy] = center;
  if (bounds !== undefined) {
    cx = keepOnScreen(cx, bounds[0], r + EDGE_MARGIN);
    cy = keepOnScreen(cy, bounds[1], r + EDGE_MARGIN);
  }
  const shifted = Math.abs(cx - center[0]) > 0.5 || Math.abs(cy - center[1]) > 0.5;
  return { cx, cy, r, shifted, canvasCenter: center };
}

/** 平行光源の取っ手の位置 (画面座標)。取っ手のある向きから光が来る。 */
export function directionalHandle(dial: LightDial, d: DirectionalSettings): [number, number] {
  const [x, y] = padFromAngles(d.azimuth, d.elevation);
  return [dial.cx + dial.r * x, dial.cy + dial.r * y];
}

/** 画面の位置に取っ手を動かしたときの平行光源の角度 `[azimuth, elevation]`。円の外では縁 (真横から) にする。 */
export function anglesFromScreen(dial: LightDial, x: number, y: number): [number, number] {
  return anglesFromPad((x - dial.cx) / dial.r, (y - dial.cy) / dial.r);
}

/** 点光源の位置 (画面座標)。 */
export function pointHandle(
  canvas: readonly [number, number],
  view: ScreenTransform,
  p: PointSettings,
): [number, number] {
  return [view.x + view.scale * p.x * canvas[0], view.y + view.scale * p.y * canvas[1]];
}

/**
 * 点光源の取っ手を描く位置 (画面座標)。光源が画面の外にあれば画面の縁に寄せ、`offscreen` を真にする
 * (寄せた取っ手を掴んで動かすと、光源は掴んだ位置へ移る)。
 */
export function pointHandleOnScreen(
  canvas: readonly [number, number],
  view: ScreenTransform,
  p: PointSettings,
  bounds: readonly [number, number],
): { x: number; y: number; offscreen: boolean } {
  const [px, py] = pointHandle(canvas, view, p);
  const x = keepOnScreen(px, bounds[0], EDGE_MARGIN);
  const y = keepOnScreen(py, bounds[1], EDGE_MARGIN);
  return { x, y, offscreen: Math.abs(x - px) > 0.5 || Math.abs(y - py) > 0.5 };
}

/**
 * 点光源の範囲の円の縁の代わりに、画面の端に出す小さな取っ手 (T0717)。`dir` は取っ手を動かすと範囲が広がる向き (長さ 1)。
 */
export interface RangeGrip {
  x: number;
  y: number;
  dir: [number, number];
}

/**
 * 範囲の円 (中心 `center`・半径 `radius`、画面座標) の縁が画面 (縁から `EDGE_MARGIN` の内側) を通らないときの取っ手。
 * 縁が画面を通るなら (縁を直接掴めるので) `null`。
 *
 * 縁が見えないのは、円が画面をすっぽり覆うとき (範囲が大きい既定の 1.4〜1.5 ではよくある) と、円がまるごと画面の外にあるとき。
 * どちらも、光源から画面の中央へ向かう向きの縁の点を画面の内側に収めた位置に置く。取っ手をその向きへ動かすと範囲が広がる
 * (`rangeRadiusFromGrip`)。
 */
export function rangeGrip(
  center: readonly [number, number],
  radius: number,
  bounds: readonly [number, number],
): RangeGrip | null {
  const [w, h] = bounds;
  const m = EDGE_MARGIN;
  if (radius <= 0 || w < 2 * m || h < 2 * m) return null;
  const [px, py] = center;
  // 画面の内側の四角形までの最も近い距離と、最も遠い角までの距離。
  const nearest = Math.hypot(px - clampTo(px, m, w - m), py - clampTo(py, m, h - m));
  const farthest = Math.hypot(Math.max(px - m, w - m - px), Math.max(py - m, h - m - py));
  if (nearest <= radius && radius <= farthest) return null;
  let dx = w / 2 - px;
  let dy = h / 2 - py;
  const length = Math.hypot(dx, dy);
  if (length < 1) [dx, dy] = [1, 0];
  else [dx, dy] = [dx / length, dy / length];
  return {
    x: clampTo(px + dx * radius, m, w - m),
    y: clampTo(py + dy * radius, m, h - m),
    dir: [dx, dy],
  };
}

/**
 * 範囲の取っ手 (`rangeGrip`) を掴んだ位置 `start` から `to` へ動かしたときの、範囲の円の半径 (画面座標)。
 * 取っ手の向き `dir` に沿って動かした分だけ、掴んだときの半径 `startRadius` から増やす (逆向きなら減らす)。
 */
export function rangeRadiusFromGrip(
  startRadius: number,
  dir: readonly [number, number],
  start: readonly [number, number],
  to: readonly [number, number],
): number {
  return Math.max(0, startRadius + (to[0] - start[0]) * dir[0] + (to[1] - start[1]) * dir[1]);
}

function clampTo(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

/** 画面の位置に点光源を動かしたときの位置 `[x, y]` (キャンバスに対する割合。範囲に収め、小数 3 桁に丸める)。 */
export function pointFromScreen(
  canvas: readonly [number, number],
  view: ScreenTransform,
  x: number,
  y: number,
): [number, number] {
  const fraction = (
    screen: number,
    offset: number,
    size: number,
    limits: readonly [number, number],
  ) =>
    Math.round(clamp((screen - offset) / view.scale / Math.max(size, 1), limits, 0.5) * 1000) /
    1000;
  return [fraction(x, view.x, canvas[0], LIMITS.x), fraction(y, view.y, canvas[1], LIMITS.y)];
}

/**
 * 点光源の光がキャンバスの面の上で届く範囲の半径 (キャンバス座標)。光源は面から `z` だけ手前にあるので、
 * 面の上では `√(range² − z²)`。面まで届かなければ 0。
 */
export function pointRangeOnCanvas(canvas: readonly [number, number], p: PointSettings): number {
  const size = Math.max(canvas[0], canvas[1], 1);
  const range = Math.max(p.range, LIMITS.range[0]) * size;
  const z = p.z * size;
  return Math.sqrt(Math.max(0, range * range - z * z));
}

/**
 * 点光源の面の上の範囲の円の半径 (画面の CSS ピクセル) を `radius` にしたときの「範囲」(長い辺に対する割合)。
 * 手前の距離は変えないので `√((radius / scale)² + z²)`。範囲に収め、小数 3 桁に丸める。
 */
export function rangeFromScreenRadius(
  canvas: readonly [number, number],
  view: ScreenTransform,
  p: PointSettings,
  radius: number,
): number {
  const size = Math.max(canvas[0], canvas[1], 1);
  const onCanvas = Math.max(0, radius) / view.scale / size;
  const range = Math.hypot(onCanvas, p.z);
  return Math.round(clamp(range, LIMITS.range, p.range) * 1000) / 1000;
}

/** ビューポートの取っ手をキーボードやホイールで動かす量。Shift を押すと大きく動かす。 */
export const MARKER_STEPS = {
  /** 平行光源の取っ手 (向きの面の座標。LightPad と同じ)。 */
  pad: [0.05, 0.2],
  /** 点光源の横・縦 (キャンバスに対する割合)。 */
  position: [0.01, 0.1],
  /** 点光源の手前 (長い辺に対する割合)。 */
  z: [0.02, 0.1],
  /** 点光源の範囲 (長い辺に対する割合)。 */
  range: [0.05, 0.2],
} as const satisfies Record<string, readonly [number, number]>;

const round3 = (v: number) => Math.round(v * 1000) / 1000;

/**
 * ビューポートの光源の取っ手でキーを押したときの変更を `next` に書く。動かすキーでなければ `false` (何もしない)。
 *
 * - 平行光源: 矢印キーで取っ手を上下左右に動かす (パネルの丸い面と同じ。円の外へは出ない)。
 * - 点光源: 矢印キーで横・縦、PageUp / PageDown で手前 (PageUp で手前へ離す)、`+` / `-` で範囲。
 */
export function stepMarker(
  next: LightingSettings,
  kind: "directional" | "point",
  key: string,
  large: boolean,
): boolean {
  const pick = (steps: readonly [number, number]) => (large ? steps[1] : steps[0]);
  const arrows: Record<string, [number, number]> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, -1],
    ArrowDown: [0, 1],
  };
  const arrow = arrows[key];
  if (kind === "directional") {
    if (arrow === undefined) return false;
    const d = next.directional;
    const step = pick(MARKER_STEPS.pad);
    let [x, y] = padFromAngles(d.azimuth, d.elevation);
    x += arrow[0] * step;
    y += arrow[1] * step;
    const r = Math.hypot(x, y);
    if (r > 1) [x, y] = [x / r, y / r];
    const [azimuth, elevation] = anglesFromPad(x, y);
    d.azimuth = azimuth;
    d.elevation = elevation;
    return true;
  }
  const p = next.point;
  if (arrow !== undefined) {
    const step = pick(MARKER_STEPS.position);
    p.x = round3(clamp(p.x + arrow[0] * step, LIMITS.x, p.x));
    p.y = round3(clamp(p.y + arrow[1] * step, LIMITS.y, p.y));
    return true;
  }
  if (key === "PageUp" || key === "PageDown") {
    p.z = round3(clamp(p.z + (key === "PageUp" ? 1 : -1) * pick(MARKER_STEPS.z), LIMITS.z, p.z));
    return true;
  }
  // `+` は配列によって Shift が要る (`=` や `;` のキー) ので、Shift の有無で量を変えない。
  const grow = key === "+" || key === "=" ? 1 : key === "-" || key === "_" ? -1 : 0;
  if (grow === 0) return false;
  p.range = round3(clamp(p.range + grow * MARKER_STEPS.range[0], LIMITS.range, p.range));
  return true;
}

/** 点光源の手前をホイールで変えた値。`delta` が負 (奥へ回す) なら手前へ離す。1 回の量はキーボードと同じ。 */
export function wheelPointZ(p: PointSettings, delta: number): number {
  const step = MARKER_STEPS.z[0];
  return round3(clamp(p.z + (delta < 0 ? step : delta > 0 ? -step : 0), LIMITS.z, p.z));
}

const MARKERS_STORAGE_KEY = "ugoe-viewer.lighting.markers";

/** ビューポートに光源の印を出すか (既定は出す)。読めなければ既定。 */
export function loadLightMarkers(storage?: Storage): boolean {
  try {
    return (storage ?? globalThis.localStorage)?.getItem(MARKERS_STORAGE_KEY) !== "off";
  } catch {
    return true;
  }
}

/** 光源の印を出すかを覚える。覚えられなくても無視する。 */
export function saveLightMarkers(show: boolean, storage?: Storage): void {
  try {
    (storage ?? globalThis.localStorage)?.setItem(MARKERS_STORAGE_KEY, show ? "on" : "off");
  } catch {
    // 覚えられないだけで、表示には影響しない。
  }
}
