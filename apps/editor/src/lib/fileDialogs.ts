// OS のファイルダイアログと既定の場所 (ドキュメント、なければホーム)。ダイアログはバックエンドが開き
// (`open_file_dialog`・`save_file_dialog`)、選ばれたパスだけをコマンドが読み書きできる (src-tauri/src/access.rs。T1616)。
// ヘッドレスのブラウザでの確認のため差し替えられる: `window.__UGOE_FILE_DIALOGS__` に
// `FileDialogs` を置くと、ダイアログの代わりにそれを呼ぶ (docs/editor.md#ファイルダイアログ)。保存のダイアログの答えに
// 拡張子を付けるのは、本物はバックエンド (`save_file_dialog`)、差し替えはここ (`withStubExtension`) で、画面は付けない (T1625)。
import { documentDir, homeDir } from "@tauri-apps/api/path";
import { type FileFilter, withExtension } from "./filePaths";
import { ipc } from "./ipc";

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
  /** 保存先の名前を決める (ファイルが存在しなくてもよい)。名前に最初のフィルタの最初の拡張子が無ければ付ける。 */
  saveFile(options: FileDialogOptions): Promise<string | null>;
  /** 既定の場所 (ドキュメント、なければホーム)。分からなければ `null`。 */
  defaultDirectory(): Promise<string | null>;
}

declare global {
  interface Window {
    /** テスト用の差し替え。`saveFile` は拡張子の無い名前を返してよい (`fileDialogs` が付ける)。 */
    __UGOE_FILE_DIALOGS__?: FileDialogs;
  }
}

const tauriDialogs: FileDialogs = {
  openFile: ({ title, defaultPath, filters }) =>
    ipc.openFileDialog(title, defaultPath ?? null, filters ?? [], false),
  openDirectory: ({ title, defaultPath, filters }) =>
    ipc.openFileDialog(title, defaultPath ?? null, filters ?? [], true),
  saveFile: ({ title, defaultPath, filters }) =>
    ipc.saveFileDialog(title, defaultPath ?? null, filters ?? []),
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

/** 差し替えの保存のダイアログの答えに、バックエンドと同じ規則 (最初のフィルタの最初の拡張子) で拡張子を付ける。 */
function withStubExtension(stub: FileDialogs): FileDialogs {
  return {
    openFile: (options) => stub.openFile(options),
    openDirectory: (options) => stub.openDirectory(options),
    async saveFile(options) {
      const path = await stub.saveFile(options);
      const extension = options.filters?.[0]?.extensions[0];
      return path === null || extension === undefined ? path : withExtension(path, extension);
    },
    defaultDirectory: () => stub.defaultDirectory(),
  };
}

/** 今使うダイアログ。差し替えがあればそれ (保存の答えには拡張子を付ける)。 */
export function fileDialogs(): FileDialogs {
  const stub = window.__UGOE_FILE_DIALOGS__;
  return stub ? withStubExtension(stub) : tauriDialogs;
}
