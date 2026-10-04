// ツールとインスペクタが使う、モデル JSON (ugoe_model::Model) のメッシュまわりの要約。
import type { BlendMode, MaskMode, MaskRef, Model } from "../../generated/types";

export type { BlendMode, MaskMode, MaskRef };

export interface MeshInfo {
  id: string;
  name: string;
  /** Model::textures の添字。 */
  texture: number;
  /** 頂点数 (uvs の長さ)。 */
  vertices: number;
  indices: number[];
  blend: BlendMode;
  culling: boolean;
  masks: MaskRef[];
  maskMode: MaskMode;
  maskOnly: boolean;
}

export interface ModelSummary {
  /** Model::meshes の順 (WebPuppet の描画可能要素の添字と同じ)。 */
  meshes: MeshInfo[];
  /** テクスチャの名前 (Model::textures の順)。 */
  textures: string[];
}

export const EMPTY_MODEL: ModelSummary = { meshes: [], textures: [] };

/** 解析したモデル (`EditorDocument.parsedModel`) を要約する。無ければ空。 */
export function summarizeModel(model: Model | null | undefined): ModelSummary {
  if (!model) return EMPTY_MODEL;
  try {
    return {
      meshes: model.meshes.map((m) => ({
        id: m.id,
        name: m.name,
        texture: m.texture,
        vertices: m.uvs.length,
        indices: m.indices,
        blend: m.blend,
        culling: m.culling,
        masks: m.masks,
        maskMode: m.mask_mode,
        maskOnly: m.mask_only,
      })),
      textures: model.textures.map((t) => t.name),
    };
  } catch {
    return EMPTY_MODEL;
  }
}
