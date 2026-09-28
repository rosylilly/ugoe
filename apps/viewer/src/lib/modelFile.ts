// ビューアが開くモデルのファイル。`.ugoe` バンドルと JSON のモデル (docs/spec/file-formats.md)。

/** 読み込んだファイル。どこから来たか (ダイアログ・ドロップ・URL) は問わない。 */
export interface ModelFile {
  /** 表示用の名前 (パスを除いたファイル名)。 */
  name: string;
  bytes: Uint8Array;
}

export type ModelFileKind = "bundle" | "json";

/** ファイル選択で使う拡張子。 */
export const MODEL_EXTENSIONS = ["ugoe", "json"] as const;

const BUNDLE_MAGIC = [0x55, 0x47, 0x4f, 0x45]; // "UGOE"

/**
 * 形式を判別する。中身 (バンドルのマジック、JSON の `{`) を優先し、決まらなければ拡張子で決める。
 * どちらでもなければ `null`。
 */
export function detectModelKind(file: ModelFile): ModelFileKind | null {
  const { bytes } = file;
  if (BUNDLE_MAGIC.every((b, i) => bytes[i] === b)) return "bundle";
  const first = bytes.find((b) => !isJsonWhitespace(b));
  if (first === 0x7b /* { */) return "json";
  const ext = extensionOf(file.name);
  if (ext === "ugoe") return "bundle";
  if (ext === "json") return "json";
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
