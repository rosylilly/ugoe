// ビューアの外の世界 (ファイルの選択と読み込み・書き出し) とのやりとり。
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
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  baseName,
  type FileLocation,
  MODEL_EXTENSIONS,
  MODULE_EXTENSIONS,
  type ModelFile,
} from "./modelFile";

export interface ViewerHost {
  /** モデル (か着せ替えのモジュール) のファイルを選んで読む。取り消したら `null`。 */
  pickModelFile(): Promise<ModelFile | null>;
  /** 着せ替えのモジュール (`.ugoemod`) を選んで読む (複数選べる)。取り消したら空。 */
  pickModuleFiles(): Promise<ModelFile[]>;
  /** 起動時に開くモデル (コマンドライン引数など)。無ければ `null`。 */
  initialModelFile(): Promise<ModelFile | null>;
  /**
   * JSON のテキストを保存する (トラッキングの記録など)。`name` は既定のファイル名。
   * 保存したら `true`、取り消したら `false`。省くとブラウザのダウンロードで保存する。
   */
  saveJsonFile?(name: string, text: string): Promise<boolean>;
  /**
   * パス (`FileLocation` の `path:` の後ろ) のファイルを読み直す (覚えていた着せ替えのモジュールを戻すときなど)。
   * 無ければ `path:` の場所は読み直せない (ブラウザ)。
   */
  readPath?(path: string): Promise<ModelFile>;
}

declare global {
  interface Window {
    /** 差し替え用 (ヘッドレスのブラウザでの確認など)。 */
    __UGOE_VIEWER_HOST__?: ViewerHost;
  }
}

async function readTauriFile(path: string): Promise<ModelFile> {
  const buffer = await invoke<ArrayBuffer>("read_model_file", { path });
  return { name: baseName(path), bytes: new Uint8Array(buffer), location: `path:${path}` };
}

const tauriHost: ViewerHost = {
  async pickModelFile() {
    const path = await open({
      title: "モデルを開く",
      multiple: false,
      directory: false,
      filters: [
        { name: "ugoe モデル・着せ替え", extensions: [...MODEL_EXTENSIONS, ...MODULE_EXTENSIONS] },
      ],
    });
    return path === null ? null : readTauriFile(path);
  },
  async pickModuleFiles() {
    const paths = await open({
      title: "着せ替えのモジュールを開く",
      multiple: true,
      directory: false,
      filters: [{ name: "着せ替えのモジュール", extensions: [...MODULE_EXTENSIONS] }],
    });
    return paths === null ? [] : Promise.all(paths.map(readTauriFile));
  },
  async initialModelFile() {
    const path = await invoke<string | null>("initial_file");
    return path === null ? null : readTauriFile(path);
  },
  async saveJsonFile(name, text) {
    const path = await save({
      title: "保存",
      defaultPath: name,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (path === null) return false;
    await invoke("write_json_file", { path, contents: text });
    return true;
  },
  readPath: readTauriFile,
};

/** ブラウザのダウンロードで保存する。 */
function downloadJson(name: string, text: string): void {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** JSON のテキストを、今の `ViewerHost` で保存する。 */
export async function saveJsonFile(name: string, text: string): Promise<boolean> {
  const host = viewerHost();
  if (host.saveJsonFile) return host.saveJsonFile(name, text);
  downloadJson(name, text);
  return true;
}

/** File API の `File` を読む (ドロップ・`<input type="file">`)。 */
export async function readBrowserFile(file: File): Promise<ModelFile> {
  return { name: file.name, bytes: new Uint8Array(await file.arrayBuffer()) };
}

/** `<input type="file">` で選ぶ。取り消したら空。 */
function pickBrowserFiles(extensions: readonly string[], multiple: boolean): Promise<ModelFile[]> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = multiple;
    input.accept = extensions.map((e) => `.${e}`).join(",");
    input.addEventListener("change", () => {
      Promise.all([...(input.files ?? [])].map(readBrowserFile)).then(resolve, reject);
    });
    input.addEventListener("cancel", () => resolve([]));
    input.click();
  });
}

const browserHost: ViewerHost = {
  async pickModelFile() {
    const files = await pickBrowserFiles([...MODEL_EXTENSIONS, ...MODULE_EXTENSIONS], false);
    return files[0] ?? null;
  },
  pickModuleFiles() {
    return pickBrowserFiles(MODULE_EXTENSIONS, true);
  },
  async initialModelFile() {
    const url = new URLSearchParams(location.search).get("model");
    if (!url) return null;
    return readUrl(new URL(url, location.href).href);
  },
};

/** URL のファイルを読む (`?model=`)。 */
async function readUrl(url: string): Promise<ModelFile> {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: ${response.status} ${response.statusText}`);
  return {
    name: baseName(new URL(url).pathname),
    bytes: new Uint8Array(await response.arrayBuffer()),
    location: `url:${url}`,
  };
}

/** アプリに同梱した見本のファイル (`samples/` からのパス。samples.ts・vite.config.ts の `sampleModels`)。 */
export async function readSampleFile(path: string): Promise<ModelFile> {
  const url = new URL(`samples/${path}`, document.baseURI);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: ${response.status} ${response.statusText}`);
  return {
    name: `${baseName(path)} (サンプル)`,
    bytes: new Uint8Array(await response.arrayBuffer()),
    location: `sample:${path}`,
  };
}

/**
 * 場所 (`ModelFile.location`) からファイルを読み直す。`path:` は `ViewerHost.readPath` を使い、無ければ
 * (ブラウザ) 読めないとして例外を投げる。ファイルが消えた・動いたときも例外。
 */
export async function readFileAt(location: FileLocation): Promise<ModelFile> {
  const colon = location.indexOf(":");
  const rest = location.slice(colon + 1);
  switch (location.slice(0, colon)) {
    case "sample":
      return readSampleFile(rest);
    case "url":
      return readUrl(rest);
    default: {
      const host = viewerHost();
      if (host.readPath === undefined)
        throw new Error("このアプリではパスからファイルを読めません");
      return host.readPath(rest);
    }
  }
}

/** 今使う `ViewerHost`。 */
export function viewerHost(): ViewerHost {
  return window.__UGOE_VIEWER_HOST__ ?? (isTauri() ? tauriHost : browserHost);
}
