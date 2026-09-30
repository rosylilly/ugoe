// ウィンドウにドロップしたファイルを開く (App.svelte)。
//
// Tauri のウィンドウでは OS からのドロップを Tauri が受けてパスを渡す (`ViewerHost.listenFileDrop`) ので、
// パスから読み、`path:` の場所を付ける (起動し直した後も覚えた組や光源を戻せる。T0713)。
// ブラウザでは HTML の File API で読むので、場所は分からない。
//
// フォルダや読めない拡張子のファイルが混ざっていても、読めるものだけを開き、残りは飛ばしたと知らせる (T0717)。
// Tauri ではドラッグ中 (`enter`) にもパスが分かるので、落とす前に開けるかを示す (`describeDropPaths`)。
import {
  baseName,
  detectModelKind,
  MODEL_EXTENSIONS,
  MODULE_EXTENSIONS,
  type ModelFile,
} from "./modelFile";
import type { SampleTarget } from "./samples";

/**
 * ドロップしたファイルを開く。モデルとモジュール (.ugoemod) を一緒に落としたら、モデルをモジュールと一緒に開く
 * (別のモデルなら前のモジュールは一覧から除かれ、落としたモジュールが付く)。モデルは最後の 1 つだけを開く。
 * モジュールだけなら、1 つずつ読み込む。
 */
export async function openDroppedFiles(
  files: readonly ModelFile[],
  target: SampleTarget,
): Promise<void> {
  const isModule = (f: ModelFile) => detectModelKind(f) === "module";
  const model = files.filter((f) => !isModule(f)).at(-1);
  const modules = files.filter(isModule);
  if (model !== undefined)
    await target.load(
      model,
      modules.map((file) => ({ file, attach: true })),
    );
  else for (const file of modules) await target.load(file);
}

const OPENABLE_EXTENSIONS: readonly string[] = [...MODEL_EXTENSIONS, ...MODULE_EXTENSIONS];

/**
 * ドロップで開ける名前か (拡張子が `.ugoe`・`.json`・`.ugoemod`)。フォルダは拡張子が無いので開けないとみなす
 * (拡張子の付いたフォルダは、読むときに失敗して飛ばす)。バックエンドの `read_model_file` と同じ拡張子。
 */
export function isOpenableName(name: string): boolean {
  const base = baseName(name);
  const dot = base.lastIndexOf(".");
  return dot > 0 && OPENABLE_EXTENSIONS.includes(base.slice(dot + 1).toLowerCase());
}

/** ドロップしたもの 1 つ (名前か パスと、読み方)。 */
export interface DroppedItem {
  /** 名前かパス (表示と拡張子の判定に使う)。 */
  name: string;
  read(): Promise<ModelFile>;
}

/** 飛ばしたもの。`extension` は開けない拡張子 (フォルダを含む)、`unreadable` は読めなかったもの。 */
export interface SkippedItem {
  name: string;
  reason: "extension" | "unreadable";
  /** 読めなかった理由 (バックエンドのエラーなど。英語のことがある)。 */
  detail?: string;
}

/**
 * ドロップしたものを読んで開く。開ける拡張子のものだけを読み、読めたものを `openDroppedFiles` で開く。
 * 飛ばしたものがあれば、知らせる日本語の文 (`skippedMessage`) を返す。無ければ `null`。
 * 開くとき (モデルの読み込み) の失敗は例外のまま投げる。
 */
export async function openDroppedItems(
  items: readonly DroppedItem[],
  target: SampleTarget,
): Promise<string | null> {
  const skipped: SkippedItem[] = [];
  const openable = items.filter((item) => {
    if (isOpenableName(item.name)) return true;
    skipped.push({ name: baseName(item.name), reason: "extension" });
    return false;
  });
  const results = await Promise.allSettled(openable.map((item) => item.read()));
  const files: ModelFile[] = [];
  results.forEach((result, i) => {
    if (result.status === "fulfilled") files.push(result.value);
    else
      skipped.push({
        name: baseName(openable[i]?.name ?? ""),
        reason: "unreadable",
        detail: result.reason instanceof Error ? result.reason.message : String(result.reason),
      });
  });
  if (files.length > 0) await openDroppedFiles(files, target);
  return skippedMessage(skipped, files.length);
}

/** 表示する名前の数の上限 (多ければ「ほか N 個」)。 */
const MAX_NAMES = 3;

function listNames(names: readonly string[]): string {
  const shown = names.slice(0, MAX_NAMES).join("、");
  return names.length > MAX_NAMES ? `${shown} ほか ${names.length - MAX_NAMES} 個` : shown;
}

/** 飛ばしたものを知らせる文。飛ばしたものが無ければ `null`。`opened` は開いたファイルの数。 */
export function skippedMessage(skipped: readonly SkippedItem[], opened: number): string | null {
  if (skipped.length === 0) return null;
  const head =
    opened === 0
      ? "開けるファイルがありませんでした"
      : `開けないものを ${skipped.length} 個飛ばしました`;
  const parts: string[] = [];
  const ext = skipped.filter((s) => s.reason === "extension").map((s) => s.name);
  const unreadable = skipped.filter((s) => s.reason === "unreadable").map((s) => s.name);
  if (ext.length > 0)
    parts.push(
      `${listNames(ext)} (開けるのはモデル .ugoe・.json と着せ替えのモジュール .ugoemod。フォルダの中は開きません)`,
    );
  if (unreadable.length > 0) parts.push(`${listNames(unreadable)} (読めませんでした)`);
  return `${head}: ${parts.join("。")}`;
}

/** ドラッグ中のパスの見立て (枠の色と文)。`all` はすべて開ける、`some` は一部を飛ばす、`none` は開けるものが無い。 */
export interface DropPreview {
  kind: "all" | "some" | "none";
  message: string;
}

/** ドラッグ中 (Tauri の `enter`) のパスから、開けるかを見立てる (拡張子だけで見る。中身は読まない)。 */
export function describeDropPaths(paths: readonly string[]): DropPreview {
  const openable = paths.filter(isOpenableName).length;
  const skipped = paths.length - openable;
  if (openable === 0)
    return {
      kind: "none",
      message:
        "開けるファイルがありません (モデル .ugoe・.json か着せ替えのモジュール .ugoemod を落とす)",
    };
  if (skipped > 0)
    return {
      kind: "some",
      message: `${openable} 個を開きます (${skipped} 個は開けないので飛ばします)`,
    };
  return { kind: "all", message: `${openable} 個を開きます` };
}
