// グルーのパネルの表示の状態 (Undo の対象ではない)。グルーそのものは作業用のモデルの `glues` にある。

/** ビューポートで頂点の組を編集するモード (T0661)。`off` は重ねて見せるだけ。 */
export type PairMode = "off" | "add" | "remove";

/** 組を足すときに先に押した頂点 (どちらの端のメッシュの何番目か)。 */
export interface PendingVertex {
  side: "a" | "b";
  index: number;
}

/** グルーの頂点の組 (A の頂点の番号と B の頂点の番号。0 から)。 */
export interface PairKey {
  a: number;
  b: number;
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

  /** パネルの組の一覧でポインタを乗せている組 (T1651)。 */
  hoveredPair = $state<PairKey | null>(null);
  /** パネルの組の一覧でフォーカスのある組 (行の「×」。T1651)。 */
  focusedPair = $state<PairKey | null>(null);

  /**
   * ビューポートで強調する組 (T1651)。ポインタを乗せた組を、無ければフォーカスのある組を選ぶ。表示だけの状態で、
   * Command は送らない。組の番号で持つので、一覧の並びが変わっても同じ組を指す。
   */
  get highlightedPair(): PairKey | null {
    return this.hoveredPair ?? this.focusedPair;
  }

  /** 強調を消す (グルーを替えたときなど)。 */
  clearHighlight(): void {
    this.hoveredPair = null;
    this.focusedPair = null;
  }

  /** モードを `mode` にする (先に押した頂点は忘れる)。 */
  setPairMode(mode: PairMode): void {
    this.pairMode = mode;
    this.pending = null;
  }
}
