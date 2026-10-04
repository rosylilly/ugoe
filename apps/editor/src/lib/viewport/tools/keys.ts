// ビューポートのツールが window で受けるキー操作 (Esc・Delete・Ctrl (Cmd) + A など) の判定 (keys.test.ts)。
import { isInDialog, isTextEntry } from "../../shortcuts";

/**
 * ツールがキー操作を受けてよいか。入力欄 (その欄自身の操作を優先する) とダイアログの中 (後ろのビューポートの
 * 選択や頂点を見えないところで変えない) では受けない。
 */
export function acceptsToolKey(target: EventTarget | null): boolean {
  return !isTextEntry(target) && !isInDialog(target);
}
