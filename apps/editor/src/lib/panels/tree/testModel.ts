// テスト用の小さなモデル (ツリーが読むフィールドだけ)。
//
// パーツ:      Root ─┬─ Face ── Eyes
//                    └─ Hidden (不透明度 0)
// デフォーマ:  FaceWarp (Face) ── HeadRot (Face) ── EyeRot (Eyes)
// メッシュ:    Skin (Face, FaceWarp, 描画順 500)、EyeL / EyeR (Eyes, EyeRot, 600)、
//              EyeMask (Eyes, EyeRot, 600, マスク専用)、Ghost (Hidden, なし, 500)
import type { TreeModel } from "./model";

const constant = <T>(k: T) => ({ bindings: [], keyforms: [k] });
const mesh = (id: string, part: string, parent: string | null, drawOrder: number, extra = {}) => ({
  id,
  name: id,
  part,
  parent,
  keyforms: constant({ opacity: 1, draw_order: drawOrder }),
  ...extra,
});

export function testModel(): TreeModel {
  return {
    parameters: [{ id: "Angle", default: 0 }],
    parts: [
      { id: "Root", name: "Root", parent: null, keyforms: constant({ opacity: 1 }) },
      { id: "Face", name: "Face", parent: "Root", keyforms: constant({ opacity: 1 }) },
      { id: "Eyes", name: "Eyes", parent: "Face", keyforms: constant({ opacity: 1 }) },
      { id: "Hidden", name: "Hidden", parent: "Root", keyforms: constant({ opacity: 0 }) },
    ],
    deformers: [
      {
        id: "FaceWarp",
        name: "FaceWarp",
        part: "Face",
        parent: null,
        kind: { type: "warp" },
        keyforms: constant({ opacity: 1 }),
      },
      {
        id: "HeadRot",
        name: "HeadRot",
        part: "Face",
        parent: "FaceWarp",
        kind: { type: "rotation" },
        keyforms: constant({ opacity: 1 }),
      },
      {
        id: "EyeRot",
        name: "EyeRot",
        part: "Eyes",
        parent: "HeadRot",
        kind: { type: "rotation" },
        keyforms: constant({ opacity: 1 }),
      },
    ],
    meshes: [
      mesh("Skin", "Face", "FaceWarp", 500),
      mesh("EyeL", "Eyes", "EyeRot", 600),
      mesh("EyeMask", "Eyes", "EyeRot", 600, { mask_only: true }),
      mesh("EyeR", "Eyes", "EyeRot", 600),
      mesh("Ghost", "Hidden", null, 500),
    ],
  };
}

/** 配列の要素 (無ければテストを失敗させる)。 */
export function at<T>(list: T[], i: number): T {
  const x = list[i];
  if (x === undefined) throw new Error(`no element at ${i}`);
  return x;
}
