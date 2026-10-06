// 空のプロジェクト (起動時・New の直後) の判定と、次の操作の案内 (docs/editor.md#プロジェクト)。
import { t } from "./i18n";

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

/** 空のプロジェクトでツリーに出す案内。 */
export function emptyTreeHint(): string {
  return t("emptyState.tree", { action: t("fileMenu.newFromPsd.label") });
}
