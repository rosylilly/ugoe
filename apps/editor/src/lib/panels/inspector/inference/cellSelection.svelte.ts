// インスペクタのキーの格子で選んだセル (推定の操作の対象)。キーの格子 (keyforms/KeyformInspector.svelte) が書き、
// 推定の操作 (InferenceSection.svelte) が読む。表示の状態なので Undo の対象にしない。
import type { KeyformTarget } from "../../../generated/types";
import type { Cell } from "../keyforms/keyforms";
import { cellKey, targetKey, toggleCell } from "./inference";

export class CellSelection {
  /** 選んだセルの格子 (`targetKey`)。別の格子を選ぶと選択は消える。 */
  target = $state<string | null>(null);
  cells = $state<Cell[]>([]);

  /** `target` の格子で選んだセル (別の格子なら空)。 */
  of(target: KeyformTarget): Cell[] {
    return this.target === targetKey(target) ? this.cells : [];
  }

  has(target: KeyformTarget, cell: Cell): boolean {
    const key = cellKey(cell);
    return this.of(target).some((c) => cellKey(c) === key);
  }

  /** 選択を 1 つのセルにする。 */
  select(target: KeyformTarget, cell: Cell): void {
    this.target = targetKey(target);
    this.cells = [cell];
  }

  /** 追加選択 (Shift / Ctrl クリック)。すでに選んでいれば外す。 */
  toggle(target: KeyformTarget, cell: Cell): void {
    this.cells = toggleCell(this.of(target), cell);
    this.target = targetKey(target);
  }

  clear(): void {
    this.target = null;
    this.cells = [];
  }
}

/** エディタ全体で 1 つのセルの選択。 */
export const cellSelection = new CellSelection();
