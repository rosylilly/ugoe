// 出しているシーンの変化の読み上げ (docs/ui.md#状態の変化の知らせ)。クリック・ホットキー・API のどれで変わっても、
// 操作ウィンドウはプロファイルの `live` が変わったことで知る。最初に受け取った値 (開いたとき) と、名前だけの変化は読まない。

/** 出しているシーンの番号を見張る。 */
export class LiveSceneWatch {
  /** 前に見た `live` (まだ見ていなければ undefined)。 */
  #last: string | null | undefined = undefined;

  /** 今の `live` を渡す。読み上げるべき変化 (別のシーンに替わった) なら true。 */
  changed(live: string | null): boolean {
    const announce = this.#last !== undefined && live !== null && live !== this.#last;
    this.#last = live;
    return announce;
  }
}
