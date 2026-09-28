// エディタ全体のキーボードショートカットの判定 (Undo / Redo)。DOM のイベントの形だけに依存する
// 純粋な関数を置く (shortcuts.test.ts で検査する)。

/** 判定に使うキーイベントの値 (KeyboardEvent の一部)。 */
export interface KeyInput {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

export type HistoryAction = "undo" | "redo";

/**
 * Undo / Redo のショートカットか。macOS (`mac`) は Cmd、それ以外は Ctrl を使う。
 * Undo は Z、Redo は Shift + Z と (macOS 以外の) Y。Alt との組み合わせは別の操作とみなす。
 */
export function historyShortcut(e: KeyInput, mac: boolean): HistoryAction | null {
  const primary = mac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
  if (!primary || e.altKey) return null;
  const key = e.key.toLowerCase();
  if (key === "z") return e.shiftKey ? "redo" : "undo";
  if (key === "y" && !mac && !e.shiftKey) return "redo";
  return null;
}

/** 文字を入力している要素 (入力欄・選択欄・編集可能な要素) か。そこでは入力欄自身の Undo を優先する。 */
export function isTextEntry(target: EventTarget | null): boolean {
  const el = target as { closest?: (selector: string) => unknown } | null;
  return (
    typeof el?.closest === "function" && !!el.closest("input, textarea, select, [contenteditable]")
  );
}

/** 実行環境が macOS か (ショートカットの修飾キーの判定に使う)。 */
export function isMac(platform: string): boolean {
  return /mac|iphone|ipad/i.test(platform);
}
