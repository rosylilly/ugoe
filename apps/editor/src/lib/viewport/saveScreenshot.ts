// ビューポートのスクリーンショットを保存する流れ (T0420。docs/editor.md#スクリーンショット)。
// 読み戻し (ViewportController.capture) → PNG への変換 (@ugoe/player の encodePng) → 保存のダイアログ (fileDialogs.ts) →
// バックエンドの `write_png_file` (src-tauri/src/screenshot.rs)。ヘッドレスのブラウザでは `window.__UGOE_FILE_DIALOGS__` で
// ダイアログを差し替える。
import { describeRenderError, encodePng, errorCode } from "@ugoe/player";
import { errorText } from "../errors";
import { fileDialogs } from "../fileDialogs";
import { joinPath, PNG_FILTER, parentDir, withExtension } from "../filePaths";
import { t } from "../i18n";
import { ipc } from "../ipc";
import type { ViewportController } from "./controller.svelte";
import { screenshotFileName } from "./screenshot";

/**
 * 今の表示を PNG で保存する。ダイアログを開く前に読み戻すので、写るのは押したときの姿。
 * 保存したパスを返し、取り消した・モデルが無いときは `null`。失敗は例外。
 */
export async function saveViewportScreenshot(
  controller: ViewportController,
): Promise<string | null> {
  const shot = await controller.capture();
  if (shot === null) return null;
  const png = await encodePng(shot);
  const status = await ipc.projectStatus();
  const dialogs = fileDialogs();
  // 最初の場所: 開いているプロジェクトの隣、なければドキュメント (なければホーム)。
  const dir = (status.path ? parentDir(status.path) : null) ?? (await dialogs.defaultDirectory());
  const name = screenshotFileName(status.name, new Date());
  const picked = await dialogs.saveFile({
    title: t("viewport.screenshot.dialogTitle"),
    defaultPath: dir === null ? name : joinPath(dir, name),
    filters: [PNG_FILTER],
  });
  if (picked === null) return null;
  const path = withExtension(picked, "png");
  await ipc.writePngFile(path, png);
  return path;
}

/**
 * スクリーンショットの保存の失敗の理由の文。読み戻しの失敗 (`RenderError`。T0465) のうち GPU が止まったものはエディタの
 * カタログの文 (起動し直す案内)、知らないコードは `describeRenderError` (例外の文)、ほかは `errorText` (T0940)。
 */
export function screenshotErrorText(e: unknown): string {
  const code = errorCode(e, ["RenderError"]);
  if (code === "device_lost") return t("viewport.screenshot.deviceLost");
  return code !== null ? describeRenderError(e) : errorText(e);
}
