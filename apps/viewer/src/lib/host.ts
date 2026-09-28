// ビューアの外の世界 (ファイルの選択と読み込み) とのやりとり。
//
// 実行する場所ごとに実装を切り替える:
// - Tauri の中: OS のダイアログ (tauri-plugin-dialog) で選び、バックエンドの `read_model_file` で読む。
// - ふつうのブラウザ (`pnpm dev` を直接開いたとき): `<input type="file">` で選ぶ。
//   `?model=<URL>` を付けると、起動時にその URL のモデルを開く。
// - ヘッドレスのブラウザでの確認: `window.__UGOE_VIEWER_HOST__` に `ViewerHost` を置くと、それを使う。
//
// ドラッグ・アンド・ドロップはどこでも HTML の File API で読むので、ここを通らない
// (Tauri のウィンドウは `dragDropEnabled: false` にして、webview にドロップを渡している)。
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { baseName, MODEL_EXTENSIONS, type ModelFile } from "./modelFile";

export interface ViewerHost {
  /** モデルのファイルを選んで読む。取り消したら `null`。 */
  pickModelFile(): Promise<ModelFile | null>;
  /** 起動時に開くモデル (コマンドライン引数など)。無ければ `null`。 */
  initialModelFile(): Promise<ModelFile | null>;
}

declare global {
  interface Window {
    /** 差し替え用 (ヘッドレスのブラウザでの確認など)。 */
    __UGOE_VIEWER_HOST__?: ViewerHost;
  }
}

async function readTauriFile(path: string): Promise<ModelFile> {
  const buffer = await invoke<ArrayBuffer>("read_model_file", { path });
  return { name: baseName(path), bytes: new Uint8Array(buffer) };
}

const tauriHost: ViewerHost = {
  async pickModelFile() {
    const path = await open({
      title: "モデルを開く",
      multiple: false,
      directory: false,
      filters: [{ name: "ugoe モデル", extensions: [...MODEL_EXTENSIONS] }],
    });
    return path === null ? null : readTauriFile(path);
  },
  async initialModelFile() {
    const path = await invoke<string | null>("initial_file");
    return path === null ? null : readTauriFile(path);
  },
};

/** File API の `File` を読む (ドロップ・`<input type="file">`)。 */
export async function readBrowserFile(file: File): Promise<ModelFile> {
  return { name: file.name, bytes: new Uint8Array(await file.arrayBuffer()) };
}

const browserHost: ViewerHost = {
  pickModelFile() {
    return new Promise((resolve, reject) => {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = MODEL_EXTENSIONS.map((e) => `.${e}`).join(",");
      input.addEventListener("change", () => {
        const file = input.files?.[0];
        if (file === undefined) resolve(null);
        else readBrowserFile(file).then(resolve, reject);
      });
      input.addEventListener("cancel", () => resolve(null));
      input.click();
    });
  },
  async initialModelFile() {
    const url = new URLSearchParams(location.search).get("model");
    if (!url) return null;
    const response = await fetch(url);
    if (!response.ok) throw new Error(`${url}: ${response.status} ${response.statusText}`);
    const name = baseName(new URL(url, location.href).pathname);
    return { name, bytes: new Uint8Array(await response.arrayBuffer()) };
  },
};

/** 今使う `ViewerHost`。 */
export function viewerHost(): ViewerHost {
  return window.__UGOE_VIEWER_HOST__ ?? (isTauri() ? tauriHost : browserHost);
}
