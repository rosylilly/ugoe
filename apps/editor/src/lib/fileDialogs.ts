// OS のファイルダイアログ (tauri-plugin-dialog) と既定の場所 (ドキュメント、なければホーム)。
// ヘッドレスのブラウザでの確認のため差し替えられる: `window.__UGOE_FILE_DIALOGS__` に
// `FileDialogs` を置くと、ダイアログの代わりにそれを呼ぶ (docs/editor.md#ファイルダイアログ)。
import { documentDir, homeDir } from "@tauri-apps/api/path";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { FileFilter } from "./filePaths";

export interface FileDialogOptions {
  title: string;
  /** 最初に開く場所。ディレクトリか、ファイル名まで含むパス。 */
  defaultPath?: string;
  filters?: FileFilter[];
}

/** 選ばれた絶対パスを返す。取り消したら `null`。 */
export interface FileDialogs {
  /** 既存のファイルを 1 つ選ぶ。 */
  openFile(options: FileDialogOptions): Promise<string | null>;
  /** 既存のディレクトリを 1 つ選ぶ (`.ugoeproj` を開く)。 */
  openDirectory(options: FileDialogOptions): Promise<string | null>;
  /** 保存先の名前を決める (ファイルが存在しなくてもよい)。 */
  saveFile(options: FileDialogOptions): Promise<string | null>;
  /** 既定の場所 (ドキュメント、なければホーム)。分からなければ `null`。 */
  defaultDirectory(): Promise<string | null>;
}

declare global {
  interface Window {
    /** テスト用の差し替え。 */
    __UGOE_FILE_DIALOGS__?: FileDialogs;
  }
}

const tauriDialogs: FileDialogs = {
  openFile: (options) => open({ ...options, multiple: false, directory: false }),
  openDirectory: (options) => open({ ...options, multiple: false, directory: true }),
  saveFile: (options) => save({ ...options, canCreateDirectories: true }),
  async defaultDirectory() {
    for (const dir of [documentDir, homeDir]) {
      try {
        return await dir();
      } catch {
        // 次の候補へ
      }
    }
    return null;
  },
};

/** 今使うダイアログ。差し替えがあればそれ。 */
export function fileDialogs(): FileDialogs {
  return window.__UGOE_FILE_DIALOGS__ ?? tauriDialogs;
}
