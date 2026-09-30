// ビューアが開くファイル。`.ugoe` バンドルと JSON のモデル、着せ替えのモジュール `.ugoemod`
// (docs/spec/file-formats.md)。

/** 読み込んだファイル。どこから来たか (ダイアログ・ドロップ・URL) は問わない。 */
export interface ModelFile {
  /** 表示用の名前 (パスを除いたファイル名)。 */
  name: string;
  bytes: Uint8Array;
  /**
   * 後から読み直せる場所 (host.ts の `readFileAt`)。ブラウザでドロップしたファイルと `<input type="file">` で選んだファイルには無い
   * (File API は場所を教えない)。
   */
  location?: FileLocation;
}

/** モデルと一緒に開く着せ替えのモジュール (見本・ドロップ)。 */
export interface ModuleToOpen {
  file: ModelFile;
  /** 読み込んだら付けるか。 */
  attach: boolean;
}

/**
 * ファイルを読み直せる場所。`path:` は Tauri で選んだ・ドロップしたファイルの絶対パス、`sample:` は同梱の見本 (`samples/` からのパス)、
 * `url:` はブラウザで `?model=` から開いた URL (絶対 URL)。
 */
export type FileLocation = `path:${string}` | `sample:${string}` | `url:${string}`;

/** 文字列が `FileLocation` の形か (保存した値を読むとき)。 */
export function isFileLocation(v: unknown): v is FileLocation {
  return typeof v === "string" && /^(path|sample|url):./.test(v);
}

/** `bundle` と `json` はモデル、`module` は着せ替えのモジュール (`.ugoemod`)。 */
export type ModelFileKind = "bundle" | "json" | "module";

/** ファイル選択で使う拡張子 (モデル)。 */
export const MODEL_EXTENSIONS = ["ugoe", "json"] as const;
/** ファイル選択で使う拡張子 (着せ替えのモジュール)。 */
export const MODULE_EXTENSIONS = ["ugoemod"] as const;

const BUNDLE_MAGIC = [0x55, 0x47, 0x4f, 0x45]; // "UGOE"
/** コンテナのヘッダとチャンク表の 1 行の大きさ (docs/spec/file-formats.md#バンドル-ugoe)。 */
const HEADER_BYTES = 16;
const CHUNK_ENTRY_BYTES = 24;

/**
 * 形式を判別する。中身 (コンテナのマジック、JSON の `{`) を優先し、決まらなければ拡張子で決める。
 * どれでもなければ `null`。
 *
 * `.ugoe` と `.ugoemod` はコンテナが同じなので、チャンク表に `MODU` (モジュール) があるか `MODL` (モデル) が
 * あるかで分ける。表が読めなければ拡張子で決め、それも無ければバンドルとみなす (読み込みがエラーを出す)。
 */
export function detectModelKind(file: ModelFile): ModelFileKind | null {
  const { bytes } = file;
  const ext = extensionOf(file.name);
  if (BUNDLE_MAGIC.every((b, i) => bytes[i] === b)) {
    return containerKind(bytes) ?? (ext === "ugoemod" ? "module" : "bundle");
  }
  const first = bytes.find((b) => !isJsonWhitespace(b));
  if (first === 0x7b /* { */) return "json";
  if (ext === "ugoe") return "bundle";
  if (ext === "json") return "json";
  if (ext === "ugoemod") return "module";
  return null;
}

/** チャンク表のタグから、モデル (`MODL`) かモジュール (`MODU`) かを決める。分からなければ `null`。 */
function containerKind(bytes: Uint8Array): "bundle" | "module" | null {
  if (bytes.length < HEADER_BYTES) return null;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const count = view.getUint32(12, true);
  for (let i = 0; i < count; i++) {
    const at = HEADER_BYTES + i * CHUNK_ENTRY_BYTES;
    if (at + 4 > bytes.length) break;
    const tag = String.fromCharCode(...bytes.subarray(at, at + 4));
    if (tag === "MODL") return "bundle";
    if (tag === "MODU") return "module";
  }
  return null;
}

/** パスからファイル名を取り出す (`/` と `\` の両方を区切りとみなす)。 */
export function baseName(path: string): string {
  return path.split(/[/\\]/).pop() ?? path;
}

function extensionOf(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot < 0 ? "" : name.slice(dot + 1).toLowerCase();
}

function isJsonWhitespace(b: number): boolean {
  // BOM (EF BB BF) も読み飛ばす
  return (
    b === 0x20 || b === 0x09 || b === 0x0a || b === 0x0d || b === 0xef || b === 0xbb || b === 0xbf
  );
}
