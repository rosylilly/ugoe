// ビューアの外の世界 (ファイルの選択と読み込み・書き出し) とのやりとり。
//
// 実行する場所ごとに実装を切り替える:
// - Tauri の中: OS のダイアログをバックエンド (`open_file_dialog`・`save_file_dialog`) で開き、
//   バックエンドの `read_model_file` で読み、`write_json_file`・`write_png_file` で書く。バックエンドは、
//   利用者がダイアログで選んだ・ドロップした・起動時に渡したパスしか読み書きしない (T0727。README の「ファイルの読み書きの範囲」)。
// - ふつうのブラウザ (`pnpm dev` を直接開いたとき): `<input type="file">` で選ぶ。
//   `?model=<URL>` を付けると、起動時にその URL のモデルを開く。
// - ヘッドレスのブラウザでの確認: `window.__UGOE_VIEWER_HOST__` に `ViewerHost` を置くと、それを使う。
//
// ドラッグ・アンド・ドロップは、Tauri のウィンドウでは OS からのドロップを Tauri が受けてパスを渡す
// (`listenFileDrop`。パスから読むので、起動し直した後も読み直せる。T0713)。ブラウザでは HTML の File API で読むので、
// ここを通らず、場所は分からない。
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { locale } from "@tauri-apps/plugin-os";
import {
  baseName,
  type FileLocation,
  MODEL_EXTENSIONS,
  MODULE_EXTENSIONS,
  type ModelFile,
} from "@ugoe/player";
import { t } from "./i18n";
import { PNG_PATH_HEADER } from "./screenshot";

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
   * PNG の画像を保存する (スクリーンショット、T0420)。`name` は既定のファイル名。保存した場所 (パスかファイル名) を返し、
   * 取り消したら `null`。省くとブラウザのダウンロードで保存する。
   */
  savePngFile?(name: string, bytes: Uint8Array): Promise<string | null>;
  /**
   * パス (`FileLocation` の `path:` の後ろ) のファイルを読み直す (覚えていた着せ替えのモジュールを戻すときなど)。
   * 無ければ `path:` の場所は読み直せない (ブラウザ)。
   */
  readPath?(path: string): Promise<ModelFile>;
  /**
   * OS からウィンドウにドロップしたファイルを、パスで受け取る (Tauri)。返すのは受け取りをやめる関数。
   * 無ければ HTML のドロップ (File API) で受け取る (場所は分からない)。
   */
  listenFileDrop?(listener: (event: FileDropEvent) => void): Promise<() => void>;
  /**
   * OS の言語 (BCP 47 のタグ。Tauri は tauri-plugin-os の `locale()`)。取れなければ `null`。
   * 無ければ WebView の `navigator.languages` を使う (@ugoe/i18n の既定)。
   */
  systemLocale?(): Promise<string | null>;
}

/**
 * OS からのドロップ (`ViewerHost.listenFileDrop`)。`hover` はファイルを持ってウィンドウに入った・出たとき
 * (入ったときは持っているもののパスも。落とす前に開けるかを示す、T0717)、
 * `drop` は落としたファイルのパス (絶対パス。`readPath` で読む)。
 */
export type FileDropEvent =
  | { type: "hover"; active: boolean; paths?: string[] }
  | { type: "drop"; paths: string[] };

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

/** 開くダイアログ (バックエンドの `open_file_dialog`)。選んだファイルのパスを返し、取り消したら空。 */
function openFileDialog(
  title: string,
  filterName: string,
  extensions: readonly string[],
  multiple: boolean,
): Promise<string[]> {
  return invoke<string[]>("open_file_dialog", { title, filterName, extensions, multiple });
}

/**
 * 保存のダイアログ (バックエンドの `save_file_dialog`)。選んだパス (拡張子が無ければバックエンドが付ける) を返し、
 * 取り消したら `null`。返したパスにだけ、1 回書ける。
 */
function saveFileDialog(
  title: string,
  defaultName: string,
  filterName: string,
  extension: "json" | "png",
): Promise<string | null> {
  return invoke<string | null>("save_file_dialog", { title, defaultName, filterName, extension });
}

const tauriHost: ViewerHost = {
  async pickModelFile() {
    const [path] = await openFileDialog(
      t("files.openModel.title"),
      t("files.openModel.filter"),
      [...MODEL_EXTENSIONS, ...MODULE_EXTENSIONS],
      false,
    );
    return path === undefined ? null : readTauriFile(path);
  },
  async pickModuleFiles() {
    const paths = await openFileDialog(
      t("files.openModules.title"),
      t("files.openModules.filter"),
      MODULE_EXTENSIONS,
      true,
    );
    return Promise.all(paths.map(readTauriFile));
  },
  async initialModelFile() {
    const path = await invoke<string | null>("initial_file");
    return path === null ? null : readTauriFile(path);
  },
  async saveJsonFile(name, text) {
    const path = await saveFileDialog(t("files.saveJson.title"), name, "JSON", "json");
    if (path === null) return false;
    await invoke("write_json_file", { path, contents: text });
    return true;
  },
  async savePngFile(name, bytes) {
    const path = await saveFileDialog(
      t("files.savePng.title"),
      name,
      t("files.savePng.filter"),
      "png",
    );
    if (path === null) return null;
    // バイト列は本文でそのまま送り、パスはヘッダーで渡す (ヘッダーは ASCII だけなので URI の形に符号化する)。
    await invoke("write_png_file", bytes, {
      headers: { [PNG_PATH_HEADER]: encodeURIComponent(path) },
    });
    return path;
  },
  readPath: readTauriFile,
  systemLocale: locale,
  listenFileDrop(listener) {
    return getCurrentWebview().onDragDropEvent(({ payload }) => {
      switch (payload.type) {
        case "enter":
          listener({ type: "hover", active: true, paths: payload.paths });
          break;
        case "leave":
          listener({ type: "hover", active: false });
          break;
        case "drop":
          listener({ type: "drop", paths: payload.paths });
          break;
      }
    });
  },
};

/** ブラウザのダウンロードで保存する。 */
function downloadJson(name: string, text: string): void {
  download(name, new Blob([text], { type: "application/json" }));
}

function download(name: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
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

/** PNG の画像を、今の `ViewerHost` で保存する。保存した場所を返し、取り消したら `null`。 */
export async function savePngFile(name: string, bytes: Uint8Array): Promise<string | null> {
  const host = viewerHost();
  if (host.savePngFile) return host.savePngFile(name, bytes);
  download(name, new Blob([bytes as Uint8Array<ArrayBuffer>], { type: "image/png" }));
  return name;
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
    name: t("samples.fileName", { name: baseName(path) }),
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
      if (host.readPath === undefined) throw new Error(t("files.noPaths"));
      return host.readPath(rest);
    }
  }
}

/** OS の言語 (今の `ViewerHost` で取る)。取れなければ `null`。 */
export async function systemLocale(): Promise<string | null> {
  return (await viewerHost().systemLocale?.()) ?? null;
}

/** 今使う `ViewerHost`。 */
export function viewerHost(): ViewerHost {
  return window.__UGOE_VIEWER_HOST__ ?? (isTauri() ? tauriHost : browserHost);
}
