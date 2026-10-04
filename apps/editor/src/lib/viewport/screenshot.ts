// ビューポートのスクリーンショット (表示中のモデルを PNG で保存する、T0420。docs/editor.md#スクリーンショット) の
// 既定のファイル名とダイアログのフィルタ。
//
// 画像は canvas の読み戻しではなく `WebPuppet.read_pixels()` で取る (ViewportController.capture)。画素の検査と
// PNG への変換 (`encodePng`)・読み戻した画像の型 (`Screenshot`)・バックエンドへのヘッダー (`PNG_PATH_HEADER`) は、
// ビューアと共有する @ugoe/player の screenshot.ts にある (T0730)。保存の流れは saveScreenshot.ts。
import { timestampedPngName } from "@ugoe/player";
import { safeFileName } from "../filePaths";

/**
 * 保存のダイアログの既定のファイル名: `<プロジェクト名>-<年月日>-<時分秒>.png` (時刻は地方時)。
 * プロジェクト名のうちファイル名に使えない文字は `_` にし、空なら `Untitled` (filePaths.ts の `safeFileName`)。
 */
export function screenshotFileName(projectName: string | null, now: Date): string {
  return timestampedPngName(safeFileName(projectName ?? ""), now);
}

/** ダイアログのフィルタ。 */
export const PNG_FILTER = { name: "PNG image (.png)", extensions: ["png"] };
