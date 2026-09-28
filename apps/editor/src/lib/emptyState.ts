// 空のプロジェクト (起動時・New の直後) の判定と、次の操作の案内 (docs/editor.md#プロジェクト)。
/**
 * パーツ・メッシュ・デフォーマ・テクスチャが 1 つもないモデルか。
 * PSD の取り込みがキャンバスを PSD の大きさにする条件 (ugoe_editor_core::import) と同じ。
 */
export function isEmptyModel(
  model: Record<"parts" | "meshes" | "deformers" | "textures", unknown[]>,
) {
  return (
    model.parts.length === 0 &&
    model.meshes.length === 0 &&
    model.deformers.length === 0 &&
    model.textures.length === 0
  );
}

/** 空のプロジェクトでビューポートに出す案内。 */
export const EMPTY_VIEWPORT_HINT =
  "This project is empty. Use File > New from PSD… to start from a PSD file.";

/** 空のプロジェクトでツリーに出す案内。 */
export const EMPTY_TREE_HINT = "No parts yet. Start with File > New from PSD…";
