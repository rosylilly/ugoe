// スロットのパネルの表示の状態 (Undo の対象ではない)。スロットそのものはモデル (Model.slots) にある。

export class SlotsView {
  /** スロットのパネルを開いているか。 */
  active = $state(false);
  /** パネルで選んでいるスロットの ID。 */
  slot = $state<string | null>(null);
}
