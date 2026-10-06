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

/** ツリーの空の状態の次の操作 (「File」のメニューの項目)。 */
export type TreeEmptyAction = "newFromPsd" | "importPsd";

/**
 * ツリーに行が無いときの案内: 何が無いかの 1 文と、次の操作 (docs/ui.md#デザイン の「空の状態」)。
 * 空のプロジェクトは PSD から新規、パーツ (デフォーマ・メッシュ) の無いモデルは PSD の取り込み。モデルを読めないときは操作なし。
 */
export function treeEmptyState(state: {
  model: boolean;
  empty: boolean;
  view: "parts" | "deformers";
}): { text: string; action: TreeEmptyAction | null } {
  if (!state.model) return { text: t("tree.empty.noModel"), action: null };
  if (state.empty) return { text: t("emptyState.tree"), action: "newFromPsd" };
  return {
    text: state.view === "parts" ? t("tree.empty.parts") : t("tree.empty.deformers"),
    action: "importPsd",
  };
}
