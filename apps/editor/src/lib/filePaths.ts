// ファイルダイアログに渡すフィルタ・既定のパスと、選ばれたパスの整形 (純粋な関数)。
// ダイアログの呼び出しは ./fileDialogs.ts (docs/editor.md#ファイルダイアログ)。

/** ダイアログの拡張子フィルタ。`extensions` は `.` を付けない。 */
export interface FileFilter {
  name: string;
  extensions: string[];
}

export const PSD_FILTER: FileFilter = { name: "Photoshop document (.psd)", extensions: ["psd"] };
export const PROJECT_FILTER: FileFilter = {
  name: "ugoe project (.ugoeproj)",
  extensions: ["ugoeproj"],
};
export const BUNDLE_FILTER: FileFilter = { name: "ugoe bundle (.ugoe)", extensions: ["ugoe"] };

/** 拡張子 (大文字小文字は区別しない) がなければ付け足す。 */
export function withExtension(path: string, extension: string): string {
  return path.toLowerCase().endsWith(`.${extension.toLowerCase()}`) ? path : `${path}.${extension}`;
}

/** パスの区切り文字。`\` だけを含むもの (Windows のパス) は `\`、それ以外は `/`。 */
function separator(path: string): string {
  return path.includes("\\") && !path.includes("/") ? "\\" : "/";
}

/** ディレクトリとファイル名をつなぐ。区切り文字はディレクトリに合わせる。 */
export function joinPath(dir: string, name: string): string {
  const sep = separator(dir);
  return dir.endsWith("/") || dir.endsWith("\\") ? `${dir}${name}` : `${dir}${sep}${name}`;
}

/** 親のディレクトリ。区切り文字がなければ `null`。ルート (`/`・`C:\`) の子はルートを返す。 */
export function parentDir(path: string): string | null {
  const trimmed = path.replace(/[\\/]+$/, "");
  const index = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  if (index < 0) return null;
  const parent = trimmed.slice(0, index);
  if (parent === "" || /^[A-Za-z]:$/.test(parent)) return trimmed.slice(0, index + 1);
  return parent;
}

/** ファイル名に使えない文字 (Windows の禁止文字と制御文字) を `_` にする。空なら `Untitled`。 */
export function safeFileName(name: string): string {
  // biome-ignore lint/suspicious/noControlCharactersInRegex: 制御文字を取り除くため
  const cleaned = name.replace(/[\\/:*?"<>|\u0000-\u001f]/g, "_").trim();
  return cleaned === "" || cleaned === "." || cleaned === ".." ? "Untitled" : cleaned;
}
