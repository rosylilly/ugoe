// 出力ウィンドウの受け手が、シーンの通知を受けてすること (docs/studio.md#シーンの切り替えと受け手) と、
// モデルの位置と大きさと回転を、描画の要素 (舞台) の大きさとビュー (ズーム・パン) と CSS の回転に分ける計算。
import type { StudioScene } from "./host";
import type { Background, Transform } from "./profile";

/** 受け手が今持っているもの。 */
export interface FollowedScene {
  /** 読んだモデルの `modelRevision`。モデルが無ければ 0。 */
  modelRevision: number;
  /** 付けたモジュールの `modulesRevision`。 */
  modulesRevision: number;
}

/**
 * シーンの通知を受けてすること。
 * - `empty`: モデルの無いシーン (描かない)。
 * - `reload`: モデルを読み直し、そのシーンのフレームを最初から適用する。
 * - `modules`: モジュールを付け直す (物理のリセットはフレームの印が揃える)。前のシーンのフレームも続けて適用する。
 * - `look`: 見せ方だけを変える。前のシーンのフレームも続けて適用する。
 */
export type ScenePlan = "empty" | "reload" | "modules" | "look";

export function planScene(current: FollowedScene | null, scene: StudioScene): ScenePlan {
  if (scene.modelRevision === 0) return "empty";
  if (current === null || current.modelRevision !== scene.modelRevision) return "reload";
  if (current.modulesRevision !== scene.modulesRevision) return "modules";
  return "look";
}

/** 描画の要素 (舞台) の置き方と、ビューの合わせ方。長さは CSS ピクセル。 */
export interface StageLayout {
  /** 舞台の大きさ。回転しないときは出力と同じ、回転するときは回しても隅が欠けない正方形。 */
  width: number;
  height: number;
  /** 出力の左上からの舞台の位置 (舞台の中心は出力の中心)。 */
  left: number;
  top: number;
  /** モデルを置く位置 (出力の中心からのずれ)。パンで動かし、回転の中心にもする。 */
  offsetX: number;
  offsetY: number;
  rotation: number;
}

/** 出力 `width`×`height` に、配置 `transform` で置くときの舞台。 */
export function stageLayout(width: number, height: number, transform: Transform): StageLayout {
  const offsetX = transform.x * width;
  const offsetY = transform.y * height;
  if (transform.rotation === 0) {
    return { width, height, left: 0, top: 0, offsetX, offsetY, rotation: 0 };
  }
  // 回転の中心がずれていても出力を覆うように、対角線にずれの分を足した正方形にする。
  const side = Math.ceil(
    Math.hypot(width, height) + 2 * Math.max(Math.abs(offsetX), Math.abs(offsetY)),
  );
  return {
    width: side,
    height: side,
    left: (width - side) / 2,
    top: (height - side) / 2,
    offsetX,
    offsetY,
    rotation: transform.rotation,
  };
}

/**
 * 舞台に合わせた (`fit`) 後に掛けるズームの倍率。大きさの基準は舞台ではなく出力に合わせた大きさにする
 * (回転して舞台が大きくなっても、モデルの大きさが変わらないように)。`canvas` はモデルのキャンバスの大きさ。
 */
export function zoomFactor(
  output: readonly [number, number],
  stage: readonly [number, number],
  canvas: readonly [number, number],
  scale: number,
): number {
  const fit = (w: number, h: number) => Math.min(w / canvas[0], h / canvas[1]);
  return (scale * fit(output[0], output[1])) / fit(stage[0], stage[1]);
}

/** 背景の CSS (`background`)。画像は `imageUrl` (読んだ画像の blob URL。まだなら無し)。 */
export function backgroundCss(background: Background, imageUrl: string | null): string {
  switch (background.kind) {
    case "transparent":
      return "transparent";
    case "color":
    case "chroma":
      return background.color;
    case "image":
      return imageUrl === null ? "transparent" : `center / cover no-repeat url("${imageUrl}")`;
  }
}
