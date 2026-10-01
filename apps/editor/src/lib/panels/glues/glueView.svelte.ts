// グルーのパネルの表示の状態 (Undo の対象ではない)。グルーそのものは作業用のモデルの `glues` にある。

export class GlueView {
  /** 「Glue」のタブを開いているか。 */
  active = $state(false);
  /** パネルで選んでいるグルーの ID。 */
  glue = $state<string | null>(null);
}
