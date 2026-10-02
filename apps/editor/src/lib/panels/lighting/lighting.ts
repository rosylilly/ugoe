// ライティングの画面 (docs/editor.md#ライティングの画面) の、DOM に依存しない部分。
//
// - プレビューの光源 (PreviewLighting、project.json の editor.lighting) を WebPuppet.set_lighting_json の JSON にする。
// - 平行光源の向きと、ビューポートの円の上の取っ手の位置の変換。
// - 法線マップのふくらみの設定 (BulgeSettings) の、画面の単位 (キャンバスの px) との変換。
// - 光源の座標はキャンバス座標 (ピクセル・Y 下向き) + 奥行き (手前が +z)。`direction` は光が進む向き
//   (docs/spec/rendering.md#ライティング)。
import { t } from "../../i18n";
import type {
  BulgeSettings,
  Mesh,
  Model,
  NormalMapGroupView,
  PreviewLight,
  PreviewLighting,
} from "../../ipc";

export type Rgb = [number, number, number];
export type Vec3 = [number, number, number];

/** 光源の数の上限 (ugoe_editor_core::lighting::MAX_PREVIEW_LIGHTS)。 */
export const MAX_PREVIEW_LIGHTS = 8;

/** ugoe_editor_core::lighting::PreviewLighting の既定値と同じもの (読み込む前の表示用)。 */
export const DEFAULT_PREVIEW_LIGHTING: PreviewLighting = {
  enabled: true,
  ambient: [1, 1, 1],
  ambient_intensity: 0.45,
  lights: [{ type: "directional", direction: [1, 1, -1], color: [1, 1, 1], intensity: 0.7 }],
};

function scaled(color: Rgb, intensity: number): Rgb {
  return [color[0] * intensity, color[1] * intensity, color[2] * intensity];
}

/**
 * WebPuppet.set_lighting_json に渡す JSON。`enabled` でなければ null (光源を渡さない = 環境光が白で光源なし)。
 * 色には強さを掛ける (1 を超えてよい)。
 */
export function lightingJson(lighting: PreviewLighting): string | null {
  if (!lighting.enabled) return null;
  const lights = lighting.lights.map((light) =>
    light.type === "directional"
      ? {
          type: "directional",
          direction: light.direction,
          color: scaled(light.color, light.intensity),
        }
      : {
          type: "point",
          position: light.position,
          color: scaled(light.color, light.intensity),
          ...(light.range === null ? {} : { range: light.range }),
        },
  );
  return JSON.stringify({ ambient: scaled(lighting.ambient, lighting.ambient_intensity), lights });
}

/**
 * 平行光源の取っ手: 光源へ向かう単位ベクトル `l = −normalize(direction)` の xy (単位円の中) と、
 * 光源が奥 (キャンバスの裏) にあるか。
 */
export function directionToHandle(direction: Vec3): { x: number; y: number; behind: boolean } {
  const len = Math.hypot(...direction);
  if (!(len > 0)) return { x: 0, y: 0, behind: false };
  return { x: -direction[0] / len, y: -direction[1] / len, behind: direction[2] > 0 };
}

/** 取っ手の位置 (単位円の中。外なら円の上に寄せる) から平行光源の向き (長さ 1。小数 3 桁) を作る。 */
export function handleToDirection(x: number, y: number, behind: boolean): Vec3 {
  const r = Math.hypot(x, y);
  const [hx, hy] = r > 1 ? [x / r, y / r] : [x, y];
  const z = Math.sqrt(Math.max(0, 1 - hx * hx - hy * hy));
  const out: Vec3 = [-hx, -hy, behind ? z : -z];
  // プロジェクトに残す値を読みやすくするため、小数 3 桁に丸める。
  return out.map((v) => Math.round(v * 1000) / 1000 || 0) as Vec3;
}

/** 平行光源の仰角 (度)。キャンバスの正面 (手前) から当てると 90、横からなら 0、裏からなら負。 */
export function elevationDegrees(direction: Vec3): number {
  const len = Math.hypot(...direction);
  if (!(len > 0)) return 0;
  return (Math.asin(Math.max(-1, Math.min(1, -direction[2] / len))) * 180) / Math.PI;
}

/** 点光源の光が届く範囲の、キャンバスの上での半径 (届かなければ 0、減衰しなければ null)。 */
export function rangeOnCanvas(light: PreviewLight): number | null {
  if (light.type !== "point" || light.range === null) return null;
  const z = light.position[2];
  return Math.sqrt(Math.max(0, light.range * light.range - z * z));
}

/** 新しく足す光源。点光源はキャンバスの中央の手前に、キャンバスの大きさに応じた距離で置く。 */
export function newLight(
  kind: PreviewLight["type"],
  canvas: { width: number; height: number },
): PreviewLight {
  if (kind === "directional") {
    return { type: "directional", direction: [1, 1, -1], color: [1, 1, 1], intensity: 0.7 };
  }
  const size = Math.max(canvas.width, canvas.height);
  return {
    type: "point",
    position: [canvas.width / 2, canvas.height / 3, Math.round(size * 0.3)],
    color: [1, 0.95, 0.85],
    intensity: 1,
    range: Math.round(size * 1.2),
  };
}

/** 0〜1 の色を `#rrggbb` に。 */
export function rgbToHex(color: Rgb): string {
  return `#${color
    .map((c) =>
      Math.round(Math.max(0, Math.min(1, c)) * 255)
        .toString(16)
        .padStart(2, "0"),
    )
    .join("")}`;
}

/** `#rrggbb` を 0〜1 の色に。読めなければ null。 */
export function hexToRgb(hex: string): Rgb | null {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!m) return null;
  return [m[1], m[2], m[3]].map((h) => Number.parseInt(h ?? "0", 16) / 255) as Rgb;
}

/** メッシュがライティングを受けるか (`lit` は既定 true で、true は JSON で省略される)。 */
export function isLit(mesh: Mesh): boolean {
  return mesh.lit !== false;
}

/** ライティングを受けるかを選べるメッシュ (マスク専用のメッシュは描かないので除く)。 */
export function litCandidates(model: Model): Mesh[] {
  return model.meshes.filter((m) => !m.mask_only);
}

/** モデルがライティングを受けないときの案内 (受けるなら null)。 */
export function lightingHint(model: Model): string | null {
  if (model.lighting !== true) {
    return t("lighting.hint.modelOff");
  }
  if (!model.meshes.some((m) => isLit(m) && !m.mask_only)) {
    return t("lighting.hint.noLitMesh");
  }
  return null;
}

// ---- 法線マップのふくらみの設定 (docs/editor.md#ライティングの画面) ----

/**
 * 画面で選ぶふくらみの設定。縁の幅 `edge` とならし `smoothing` は画面の単位 ({@link bulgeUnit}): まとまりならキャンバスの
 * px、単独でふくらませるものならテクスチャの px。`edge` は `roundEdges` でないときも欄の値として残す。
 */
export interface BulgeForm {
  strength: number;
  /** 縁だけを丸めるか (偽なら形全体を 1 つのふくらみにする)。 */
  roundEdges: boolean;
  edge: number;
  smoothing: number;
}

/** 既定で縁を丸めないまとまりで、縁を丸めにしたときの幅 (画面の単位)。 */
const DEFAULT_EDGE = 12;

/** 画面の単位の 1 がバックエンドの長さ (まとまりの格子の画素) のいくつか (= 格子の 1 画素のキャンバスの px)。 */
function bulgeScale(view: NormalMapGroupView | null): number {
  const scale = view?.canvas_per_pixel;
  return scale != null && scale > 0 ? scale : 1;
}

/** 長さの単位の表示 (まとまりはキャンバスの px、単独のものはテクスチャの px)。 */
export function bulgeUnit(view: NormalMapGroupView | null): string {
  return view?.canvas_per_pixel != null ? t("lighting.unit.px") : t("lighting.unit.texturePx");
}

/** まとまりの既定の設定を画面の単位にしたもの (まとまりが分からなければ既定値)。 */
export function bulgeForm(view: NormalMapGroupView | null): BulgeForm {
  const scale = bulgeScale(view);
  const d = view?.defaults;
  return {
    strength: d?.strength ?? 0.5,
    roundEdges: d?.edge != null,
    edge: d?.edge != null ? d.edge * scale : DEFAULT_EDGE,
    smoothing: (d?.smoothing ?? 1) * scale,
  };
}

/** 画面の設定を、バックエンドに送る設定 (長さはまとまりの格子の画素) にする。 */
export function bulgeSettings(form: BulgeForm, view: NormalMapGroupView | null): BulgeSettings {
  const scale = bulgeScale(view);
  return {
    strength: form.strength,
    edge: form.roundEdges ? form.edge / scale : null,
    smoothing: form.smoothing / scale,
  };
}

/** 縁の幅とならしの欄の上限 (画面の単位。既定の値が大きいまとまりでも選べるように)。 */
export function bulgeLimits(view: NormalMapGroupView | null): { edge: number; smoothing: number } {
  const d = bulgeForm(view);
  return {
    edge: Math.max(64, Math.ceil(d.edge * 2)),
    smoothing: Math.max(10, Math.ceil(d.smoothing * 2)),
  };
}
