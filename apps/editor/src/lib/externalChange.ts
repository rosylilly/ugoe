// 外でのプロジェクトの書き換え (docs/editor.md#外での書き換え) の判定と表示の文。DOM に依存しない部分だけを置く
// (externalChange.test.ts で検査する)。検出はバックエンドの `check_external_change` (ファイルの指紋を比べる)。
import { errorCode } from "./errors";
import type { ExternalChange } from "./ipc";

/** 一定の間隔で確かめる間隔 (ミリ秒)。ウィンドウが見えているときだけ確かめる。 */
export const CHECK_INTERVAL_MS = 5000;

/** 確かめた結果にどう応じるか: 何もしない・黙って開き直す・利用者に尋ねる。 */
export type ExternalAction = "none" | "reload" | "ask";

export interface ExternalContext {
  /** 未保存の変更があるか。 */
  modified: boolean;
  /** 未保存の変更がなければ自動で開き直す設定。 */
  autoReload: boolean;
  /** 利用者が「後で」を選んだ外の状態の目印 (同じ変更を二度尋ねない)。 */
  dismissed: string | null;
}

/** ウィンドウに戻ったとき・一定の間隔で確かめた結果への応じ方。保存の前はこれを使わず、変わっていれば必ず尋ねる。 */
export function externalAction(
  change: ExternalChange | null,
  context: ExternalContext,
): ExternalAction {
  if (!change || change.token === context.dismissed) return "none";
  if (!change.missing && !context.modified && context.autoReload) return "reload";
  return "ask";
}

/**
 * モジュールのプロジェクトの本体の変化への応じ方 (docs/editor.md#本体の変化)。本体の読み直しは保存していない
 * モジュールの編集を保つので、自動で開き直す設定なら未保存の変更があっても読み直す。本体が見つからなければ尋ねる。
 */
export function baseAction(
  change: ExternalChange | null,
  context: Omit<ExternalContext, "modified">,
): ExternalAction {
  if (!change || change.token === context.dismissed) return "none";
  if (!change.missing && context.autoReload) return "reload";
  return "ask";
}

/** 変わったファイルの短い一覧 (`max` を超えたら「ほか N 件」)。 */
export function describeFiles(files: string[], max = 3): string {
  if (files.length <= max) return files.join(", ");
  const rest = files.length - max;
  return `${files.slice(0, max).join(", ")} and ${rest} more file${rest === 1 ? "" : "s"}`;
}

/** 保存が外での書き換えで断られたときのエラー (コード `editor.external_change`。apps/editor/src-tauri/src/error.rs)。 */
export function isExternalChangeError(error: unknown): boolean {
  return errorCode(error) === "editor.external_change";
}

const AUTO_RELOAD_KEY = "ugoe.editor.autoReload";

/** 「未保存の変更がなければ自動で開き直す」(既定は切。localStorage に覚える。使えない環境では切)。 */
export function loadAutoReload(): boolean {
  try {
    return globalThis.localStorage?.getItem(AUTO_RELOAD_KEY) === "true";
  } catch {
    return false;
  }
}

export function saveAutoReload(enabled: boolean): void {
  try {
    globalThis.localStorage?.setItem(AUTO_RELOAD_KEY, String(enabled));
  } catch {
    // 保存できなくても、この画面では今の値を使い続ける。
  }
}
