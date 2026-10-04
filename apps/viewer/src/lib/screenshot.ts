// スクリーンショット (表示中のモデルを PNG で保存する、T0420) の既定のファイル名。
//
// 画素の検査と PNG への変換 (`encodePng`)・バックエンドへのヘッダー (`PNG_PATH_HEADER`) は、エディタと共有する
// @ugoe/player の screenshot.ts にある (T0730)。
import { timestampedPngName } from "@ugoe/player";

/** 既定のファイル名に使えない文字 (Windows の禁止文字と制御文字)。 */
// biome-ignore lint/suspicious/noControlCharactersInRegex: 制御文字を取り除くため
const UNSAFE = /[\\/:*?"<>|\u0000-\u001f]/g;

/**
 * 保存のダイアログの既定のファイル名: `<モデルの名前>-<年月日>-<時分秒>.png` (時刻は地方時)。
 * モデルの名前はファイル名から拡張子と「 (サンプル)」のような後ろの括弧書きを除き、ファイル名に使えない文字を `_` にする。
 * モデルが無い・名前が空なら `screenshot`。
 */
export function screenshotFileName(modelName: string | null, now: Date): string {
  let base = (modelName ?? "").replace(/\s*\([^)]*\)\s*$/, "").trim();
  base = base.replace(/\.(ugoe|ugoemod|json)$/i, "");
  base = base.replace(UNSAFE, "_").trim();
  if (base === "" || base === "." || base === "..") base = "screenshot";
  return timestampedPngName(base, now);
}
