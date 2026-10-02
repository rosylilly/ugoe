// 書き出しの設定の範囲と、書き出しの結果の要約の文言 (docs/editor.md#書き出し)。文言はカタログの `export.summary`。
import { formatNumber } from "@ugoe/i18n";
import { t } from "./i18n";
import type { AtlasReport, ExportReport, ModuleExportReport } from "./ipc";

/** アトラス 1 枚の大きさの上限として選べる値 (ugoe_editor_core::export::MAX_ATLAS_SIZE まで)。 */
export const ATLAS_SIZES = [512, 1024, 2048, 4096, 8192, 16384] as const;
/** 余白 (端の画素の複製の幅) の上限 (ugoe_editor_core::export::MAX_ATLAS_PADDING)。 */
export const MAX_ATLAS_PADDING = 64;

/** `before → after` (変わらなければ 1 つだけ)。 */
function change(before: number, after: number): string {
  return before === after ? String(before) : `${before} → ${after}`;
}

/** まとめずに残した理由の表示 (知らない理由はそのまま出す)。 */
function keptReason(reason: string): string {
  switch (reason) {
    case "too_large":
      return t("export.summary.kept.tooLarge");
    case "uv_out_of_range":
      return t("export.summary.kept.uvOutOfRange");
    case "has_normal_map":
      return t("export.summary.kept.hasNormalMap");
    default:
      return reason;
  }
}

/** 書き出しの結果の要約の行: テクスチャの枚数・状態の切り替え・描画の回数と、アトラス化の詳細。 */
export function exportSummary(report: ExportReport): string[] {
  const { before, after, atlas } = report;
  const lines = [
    t("export.summary.textures", { value: change(before.textures, after.textures) }),
    t("export.summary.stateChanges", {
      value: change(before.state_changes, after.state_changes),
    }),
    t("export.summary.drawCalls", { value: change(before.draw_calls, after.draw_calls) }),
  ];
  if (atlas) lines.push(...atlasSummary(atlas));
  return lines;
}

/**
 * モジュールの書き出し (.ugoemod) の結果の要約の行: 大きさと、アトラス化の詳細 (T0654)。書き出しの警告は
 * 呼び出し側が別に出す。
 */
export function moduleExportSummary(report: ModuleExportReport): string[] {
  const lines = [
    t("export.summary.size", { count: report.bytes, bytes: formatNumber(report.bytes) }),
  ];
  if (report.atlas) lines.push(...atlasSummary(report.atlas));
  return lines;
}

/** アトラス化の詳細の行。 */
function atlasSummary(atlas: AtlasReport): string[] {
  const sizes = atlas.atlases.map(([w, h]) => `${w}×${h}`).join(", ");
  const atlases = t("export.summary.atlases", { count: atlas.atlases.length });
  const lines = [
    sizes
      ? t("export.summary.packedSizes", { count: atlas.packed, atlases, sizes })
      : t("export.summary.packed", { count: atlas.packed, atlases }),
  ];
  for (const [name, first] of atlas.deduplicated) {
    lines.push(t("export.summary.deduplicated", { name, first }));
  }
  for (const [name, reason] of atlas.kept) {
    lines.push(t("export.summary.kept.line", { name, reason: keptReason(reason) }));
  }
  if (atlas.removed.length > 0) {
    lines.push(t("export.summary.removed", { names: atlas.removed.join(", ") }));
  }
  return lines;
}
