// 書き出しの設定の範囲と、書き出しの結果の要約の文言 (docs/editor.md#書き出し)。
import type { ExportReport } from "./ipc";

/** アトラス 1 枚の大きさの上限として選べる値 (ugoe_editor_core::export::MAX_ATLAS_SIZE まで)。 */
export const ATLAS_SIZES = [512, 1024, 2048, 4096, 8192, 16384] as const;
/** 余白 (端の画素の複製の幅) の上限 (ugoe_editor_core::export::MAX_ATLAS_PADDING)。 */
export const MAX_ATLAS_PADDING = 64;

/** `before → after` (変わらなければ 1 つだけ)。 */
function change(before: number, after: number): string {
  return before === after ? String(before) : `${before} → ${after}`;
}

function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** まとめずに残した理由の表示 (知らない理由はそのまま出す)。 */
const KEPT_REASONS: Record<string, string> = {
  too_large: "too large",
  uv_out_of_range: "UV outside 0–1",
  has_normal_map: "has a normal map",
};

/** 書き出しの結果の要約の行: テクスチャの枚数・状態の切り替え・描画の回数と、アトラス化の詳細。 */
export function exportSummary(report: ExportReport): string[] {
  const { before, after, atlas } = report;
  const lines = [
    `Textures: ${change(before.textures, after.textures)}`,
    `State changes: ${change(before.state_changes, after.state_changes)}`,
    `Draw calls: ${change(before.draw_calls, after.draw_calls)}`,
  ];
  if (atlas) {
    const sizes = atlas.atlases.map(([w, h]) => `${w}×${h}`).join(", ");
    const into = plural(atlas.atlases.length, "atlas", "atlases");
    lines.push(
      `Packed ${plural(atlas.packed, "texture")} into ${into}${sizes ? ` (${sizes})` : ""}`,
    );
    for (const [name, first] of atlas.deduplicated) lines.push(`Same image: ${name} → ${first}`);
    for (const [name, reason] of atlas.kept) {
      lines.push(`Kept separate: ${name} (${KEPT_REASONS[reason] ?? reason})`);
    }
    if (atlas.removed.length > 0) lines.push(`Removed unused: ${atlas.removed.join(", ")}`);
  }
  return lines;
}
