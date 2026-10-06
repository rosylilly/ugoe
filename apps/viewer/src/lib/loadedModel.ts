// モデルを開き終えたことの読み上げ (docs/ui.md#状態の変化の知らせ)。足元の「読み込みました」は文が出るだけなので、
// 開いたモデルの名前を読み上げる。同じモデルの読み直し (着せ替え・作り直し) は読まない。

/** 開いたモデルを見張る。 */
export class LoadedModelWatch {
  #last: string | null = null;

  /** 今の様子を渡す (`name` は読み込み済みならモデルの名前、まだなら null)。読み上げるべきなら true。 */
  changed(name: string | null): boolean {
    const announce = name !== null && name !== this.#last;
    this.#last = name;
    return announce;
  }
}
