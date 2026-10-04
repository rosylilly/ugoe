// ウィンドウにドロップしたファイルを開く (App.svelte)。
//
// Tauri のウィンドウでは OS からのドロップを Tauri が受けてパスを渡す (`ViewerHost.listenFileDrop`) ので、
// パスから読み、`path:` の場所を付ける (起動し直した後も覚えた組や光源を戻せる。T0713)。
// ブラウザでは HTML の File API で読むので、場所は分からない。
//
// フォルダや読めない拡張子のファイルが混ざっていても、読めるものだけを開き、残りは飛ばしたと知らせる (T0717)。
// Tauri ではドラッグ中 (`enter`) にもパスが分かるので、落とす前に開けるかを示す (`describeDropPaths`)。
// ブラウザではドラッグ中は名前が分からず、数と種類 (MIME) だけが分かるので、種類から分かる範囲で示す (`describeDropTypes`。T0718)。
import { formatList } from "@ugoe/i18n";
import {
  baseName,
  detectModelKind,
  MODEL_EXTENSIONS,
  MODULE_EXTENSIONS,
  type ModelFile,
} from "@ugoe/player";
import { errorText } from "./errors";
import { t } from "./i18n";
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
  /** 読めなかった理由 (今の言語の文。バックエンドのエラーはカタログで訳したもの。errors.ts)。 */
  detail?: string;
}

/**
 * ドロップしたものを読んで開く。開ける拡張子のものだけを読み、読めたものを `openDroppedFiles` で開く。
 * 飛ばしたものがあれば、知らせる今の言語の文 (`skippedMessage`) を返す。無ければ `null`。
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
        detail: errorText(result.reason),
      });
  });
  if (files.length > 0) await openDroppedFiles(files, target);
  return skippedMessage(skipped, files.length);
}

/** 表示する名前の数の上限 (多ければ「ほか N 個」)。 */
const MAX_NAMES = 3;

function listNames(names: readonly string[]): string {
  const shown = formatList(names.slice(0, MAX_NAMES));
  return names.length > MAX_NAMES
    ? t("drop.names.more", { names: shown, count: names.length - MAX_NAMES })
    : shown;
}

/** 飛ばしたものを知らせる文。飛ばしたものが無ければ `null`。`opened` は開いたファイルの数。 */
export function skippedMessage(skipped: readonly SkippedItem[], opened: number): string | null {
  if (skipped.length === 0) return null;
  const head =
    opened === 0 ? t("drop.skipped.none") : t("drop.skipped.some", { count: skipped.length });
  const parts: string[] = [];
  const ext = skipped.filter((s) => s.reason === "extension").map((s) => s.name);
  const unreadable = skipped.filter((s) => s.reason === "unreadable");
  if (ext.length > 0) parts.push(t("drop.skipped.extension", { names: listNames(ext) }));
  // 読めなかったものが 1 つなら理由も出す (いくつもあると長くなるので名前だけ)。
  const [only] = unreadable;
  if (unreadable.length === 1 && only?.detail)
    parts.push(t("drop.skipped.unreadableReason", { name: only.name, reason: only.detail }));
  else if (unreadable.length > 0)
    parts.push(t("drop.skipped.unreadable", { names: listNames(unreadable.map((s) => s.name)) }));
  return t("drop.skipped.message", {
    head,
    details: parts.join(t("drop.skipped.separator")),
  });
}

/**
 * ドラッグ中のパスの見立て (枠の色と文)。`all` はすべて開ける、`some` は一部を飛ばす、`none` は開けるものが無い、
 * `unknown` は開けるかが落とすまで分からない (ブラウザで名前が分からないとき。枠は `all` と同じ色)。
 */
export interface DropPreview {
  kind: "all" | "some" | "none" | "unknown";
  message: string;
}

/** ドラッグ中 (Tauri の `enter`) のパスから、開けるかを見立てる (拡張子だけで見る。中身は読まない)。 */
export function describeDropPaths(paths: readonly string[]): DropPreview {
  const openable = paths.filter(isOpenableName).length;
  const skipped = paths.length - openable;
  if (openable === 0) return { kind: "none", message: t("drop.preview.none") };
  if (skipped > 0)
    return { kind: "some", message: t("drop.preview.some", { count: openable, skipped }) };
  return { kind: "all", message: t("drop.preview.all", { count: openable }) };
}

/** ドラッグ中のもの 1 つ (ブラウザの `DataTransferItem` の `kind` と `type`)。 */
export interface DragItemInfo {
  kind: string;
  type: string;
}

/**
 * 種類 (MIME) から、開けるかを見立てる。`open` は開ける (JSON)、`skip` は開けない (画像・文書など、開けない拡張子と分かる種類)、
 * `unknown` は分からない。`.ugoe`・`.ugoemod`・フォルダはブラウザでは種類が空 (または `application/octet-stream`) になるので、
 * 開けるかは落としたときに名前で確かめる。
 */
export function classifyDragType(type: string): "open" | "skip" | "unknown" {
  const t = type.trim().toLowerCase();
  if (t === "" || t === "application/octet-stream") return "unknown";
  // OS に登録した種類 (いまは登録していない) でも開けるものとみなす。
  if (t === "application/json" || t === "text/json" || t.endsWith("+json") || t.includes("ugoe"))
    return "open";
  return "skip";
}

/**
 * ブラウザのドラッグ中 (`dragover`) のものから、開けるかを見立てる (T0718)。ブラウザはドラッグ中にファイルの名前を渡さないが、
 * 数と種類は分かる (`DataTransfer.items`)。ファイルでないもの (一緒に来る URL の文字列など) は数えない。
 * ファイルが 1 つも分からない (種類を渡さないブラウザ) なら `null` (見立てない)。
 */
export function describeDropTypes(items: readonly DragItemInfo[]): DropPreview | null {
  const files = items
    .filter((item) => item.kind === "file")
    .map((item) => classifyDragType(item.type));
  if (files.length === 0) return null;
  const skipped = files.filter((c) => c === "skip").length;
  const unknown = files.filter((c) => c === "unknown").length;
  const open = files.length - skipped - unknown;
  if (skipped === files.length) return describeDropPaths([]);
  if (unknown === 0)
    return skipped > 0
      ? { kind: "some", message: t("drop.preview.some", { count: open, skipped }) }
      : { kind: "all", message: t("drop.preview.all", { count: open }) };
  const later = t("drop.preview.later", { count: files.length - skipped });
  return skipped > 0
    ? { kind: "some", message: t("drop.preview.skipTypes", { count: skipped, later }) }
    : { kind: "unknown", message: t("drop.preview.unknown", { count: files.length, later }) };
}
