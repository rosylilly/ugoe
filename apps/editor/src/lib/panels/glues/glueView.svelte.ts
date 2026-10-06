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
   * ビューポートの重ねで組の線か頂点にポインタを乗せている組 (T1664)。パネルの行の強調と一覧の送りに使う。
   * パネルの行のポインタ (`hoveredPair`) とは別に持ち、片方が離れたときにもう片方を消さない。
   */
  viewportPair = $state<PairKey | null>(null);

  /**
   * ビューポートとパネルの行で強調する組 (T1651・T1664)。パネルの行にポインタを乗せた組、ビューポートで乗せた組、
   * フォーカスのある組の順に選ぶ。表示だけの状態で、Command は送らない。組の番号で持つので、一覧の並びが変わっても
   * 同じ組を指す。
   */
  get highlightedPair(): PairKey | null {
    return this.hoveredPair ?? this.viewportPair ?? this.focusedPair;
  }

  /** 強調を消す (グルーを替えたときなど)。 */
  clearHighlight(): void {
    this.hoveredPair = null;
    this.focusedPair = null;
    this.viewportPair = null;
  }

  /** モードを `mode` にする (先に押した頂点は忘れる)。 */
  setPairMode(mode: PairMode): void {
    this.pairMode = mode;
    this.pending = null;
  }
}
