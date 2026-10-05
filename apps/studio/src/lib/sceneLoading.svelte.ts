// 読み込み中のシーン (操作ウィンドウの表示。docs/studio.md#シーンの切り替えと舞台のロック、T0837・T0841)。
// シーンの切り替えはバックエンドがファイルを読んでパペットを組み立てるまで返らないので、その間に「読み込み中」を出す。
// 操作ウィンドウが始めた切り替えはコマンドが返るまで (`track`)、ホットキーと外部 API が始めた切り替えはバックエンドの
// `sceneLoading` の通知 (`notice`) で出す。
// バックエンドと同じく、後から始めた切り替えが勝つ (先の切り替えが後で終わっても、表示は後の切り替えのまま)。

/** 切り替えがこれより早く終われば、読み込み中を出さない (見せ方だけの編集などでちらつかないように)。 */
export const LOADING_DELAY_MS = 150;

/** 始めた切り替え (操作ウィンドウのコマンドかバックエンドの通知) のうち、最後のものが終わるまでを読み込み中とする。 */
export class SceneLoading {
  /** 読み込み中を出しているか。 */
  #active = $state(false);
  /** 読み込んでいるシーンの ID (分からなければ `null`。プロファイルを開くときなど)。 */
  #scene = $state.raw<string | null>(null);
  /** 始めた切り替えの番号。 */
  #generation = 0;
  /** バックエンドの通知で始めた切り替えの番号 (読んでいなければ `null`)。 */
  #noticed: number | null = null;
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
    const generation = this.#start(scene);
    try {
      return await work();
    } finally {
      this.#end(generation);
    }
  }

  /**
   * バックエンドの `sceneLoading` の通知を受ける。`scene` が文字列ならそのシーンの切り替えを読み始めた
   * (ホットキー・外部 API が始めたものも含む)、`null` なら最後に始めた切り替えが終わった (失敗・捨てたものも含む)。
   * 通知の後に操作ウィンドウが始めた切り替えがあれば、`null` ではそちらの表示を消さない。
   */
  notice(scene: string | null): void {
    if (scene !== null) {
      this.#noticed = this.#start(scene);
    } else if (this.#noticed !== null) {
      this.#end(this.#noticed);
      this.#noticed = null;
    }
  }

  /** 切り替えを始め、その番号を返す。 */
  #start(scene: string | null): number {
    const generation = ++this.#generation;
    clearTimeout(this.#timer);
    this.#scene = scene;
    this.#timer = setTimeout(() => {
      if (generation === this.#generation) this.#active = true;
    }, this.#delay);
    return generation;
  }

  /** 切り替え `generation` が終わった。後から始めた切り替えがあれば、そちらが終わるまで出し続ける。 */
  #end(generation: number): void {
    if (generation !== this.#generation) return;
    clearTimeout(this.#timer);
    this.#active = false;
    this.#scene = null;
  }
}
