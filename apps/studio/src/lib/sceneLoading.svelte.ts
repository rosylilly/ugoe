// 読み込み中のシーン (操作ウィンドウの表示。docs/studio.md#シーンの切り替えと舞台のロック、T0837)。
// シーンの切り替えはバックエンドがファイルを読んでパペットを組み立てるまで返らないので、その間に「読み込み中」を出す。
// バックエンドと同じく、後から始めた切り替えが勝つ (先の切り替えが後で終わっても、表示は後の切り替えのまま)。

/** 切り替えがこれより早く終われば、読み込み中を出さない (見せ方だけの編集などでちらつかないように)。 */
export const LOADING_DELAY_MS = 150;

/** 操作ウィンドウが始めた切り替えのうち、最後のものが終わるまでを読み込み中とする。 */
export class SceneLoading {
  /** 読み込み中を出しているか。 */
  #active = $state(false);
  /** 読み込んでいるシーンの ID (分からなければ `null`。プロファイルを開くときなど)。 */
  #scene = $state.raw<string | null>(null);
  /** 始めた切り替えの番号。 */
  #generation = 0;
  #timer: ReturnType<typeof setTimeout> | undefined;
  readonly #delay: number;

  constructor(delay = LOADING_DELAY_MS) {
    this.#delay = delay;
  }

  /** 読み込み中を出しているか。 */
  get active(): boolean {
    return this.#active;
  }

  /** 読み込み中を出しているシーンの ID (出していない・分からなければ `null`)。 */
  get scene(): string | null {
    return this.#active ? this.#scene : null;
  }

  /** シーン `scene` への切り替え `work` を行い、終わるまで (`delay` を過ぎたら) 読み込み中を出す。 */
  async track<T>(scene: string | null, work: () => Promise<T>): Promise<T> {
    const generation = ++this.#generation;
    clearTimeout(this.#timer);
    this.#scene = scene;
    this.#timer = setTimeout(() => {
      if (generation === this.#generation) this.#active = true;
    }, this.#delay);
    try {
      return await work();
    } finally {
      // 後から始めた切り替えがあれば、そちらが終わるまで出し続ける。
      if (generation === this.#generation) {
        clearTimeout(this.#timer);
        this.#active = false;
        this.#scene = null;
      }
    }
  }
}
