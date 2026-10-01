// グルーのパネルの表示の状態 (Undo の対象ではない)。グルーそのものは作業用のモデルの `glues` にある。

/** ビューポートで頂点の組を編集するモード (T0661)。`off` は重ねて見せるだけ。 */
export type PairMode = "off" | "add" | "remove";

/** 組を足すときに先に押した頂点 (どちらの端のメッシュの何番目か)。 */
export interface PendingVertex {
  side: "a" | "b";
  index: number;
}

export class GlueView {
  /** 「Glue」のタブを開いているか。 */
  active = $state(false);
  /** パネルで選んでいるグルーの ID。 */
  glue = $state<string | null>(null);
  /** ビューポートで組を足す・外すモード。 */
  pairMode = $state<PairMode>("off");
  /** 組を足すときに先に押した頂点。 */
  pending = $state<PendingVertex | null>(null);

  /** モードを `mode` にする (先に押した頂点は忘れる)。 */
  setPairMode(mode: PairMode): void {
    this.pairMode = mode;
    this.pending = null;
  }
}
