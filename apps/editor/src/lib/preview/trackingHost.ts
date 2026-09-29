// フェイストラッキングの記録の保存 (T0636)。OS の保存のダイアログで選んだ `.json` にバックエンドが書く
// (apps/editor/src-tauri/src/tracking.rs)。ヘッドレスのブラウザでは `window.__UGOE_FILE_DIALOGS__` で差し替える。
import { fileDialogs } from "../fileDialogs";
import { joinPath, withExtension } from "../filePaths";
import { ipc } from "../ipc";

/** 記録 (JSON のテキスト) を保存する。保存したら `true`、取り消したら `false`。 */
export async function saveTrackingRecording(name: string, text: string): Promise<boolean> {
  const dialogs = fileDialogs();
  const dir = await dialogs.defaultDirectory();
  const path = await dialogs.saveFile({
    title: "Save face tracking recording",
    defaultPath: dir === null ? name : joinPath(dir, name),
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (path === null) return false;
  await ipc.writeJsonFile(withExtension(path, "json"), text);
  return true;
}
