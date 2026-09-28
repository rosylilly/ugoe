// 「Verify」のパネルとビューポートのはみ出しの表示が共有する表示の状態 (Undo の対象ではない)。
// docs/editor.md#はみ出しの検証
import type { ElementOverflow } from "../../ipc";

export class VerifyView {
  /** 「Verify」のタブを開いているか。 */
  active = $state(false);
  /** ビューポートに、今のパラメータ値で親のワープの格子の外にある点を重ねるか。 */
  showOverflows = $state(false);
  /** 今のパラメータ値でのはみ出し (表示中だけビューポートの表示が読み直す)。 */
  current = $state.raw<ElementOverflow[]>([]);
  /** はみ出しを求められなかった理由 (なければ null)。 */
  error = $state<string | null>(null);
}
