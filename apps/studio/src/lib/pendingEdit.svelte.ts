// 送っている途中のプロファイルの設定 (操作ウィンドウのパネル。docs/studio.md#操作ウィンドウ)。
// 画面の値はプロファイルの通知で変わるので、通知が届く前に続けて変えると、次の変更が古い値を基にして前の変更を消す。
// 送っている途中の値を先に画面に出し、次の変更はその値を基にする (シーンの編集の Control.svelte の `editing` と同じ考え方)。

/**
 * 設定の変更を 1 つずつ送り、送っている途中の値を持つ。送っている間の変更はまとめて、最後のものだけを次に送る。
 * 送り終えたら途中の値を捨て、届いた値 (`current` に渡す値) に戻す (バックエンドは、コマンドを返す前にプロファイルの通知を配る)。
 */
export class PendingEdit<T> {
  /** 送っている途中の値 (送っていなければ `null`)。 */
  #value = $state.raw<T | null>(null);
  /** 送っていない値 (送っている間に来た最後の 1 つ)。 */
  #queued: T | null = null;
  #sending = false;
  readonly #send: (value: T) => Promise<unknown>;

  constructor(send: (value: T) => Promise<unknown>) {
    this.#send = send;
  }

  /** 画面に出す値 (送っている途中の値、無ければ届いた値 `confirmed`)。 */
  current(confirmed: T): T {
    return this.#value ?? confirmed;
  }

  /** `next` を送る。すぐに `current` の値になる。 */
  async submit(next: T): Promise<void> {
    this.#value = next;
    this.#queued = next;
    if (this.#sending) return;
    this.#sending = true;
    try {
      while (this.#queued !== null) {
        const value = this.#queued;
        this.#queued = null;
        await this.#send(value);
      }
    } finally {
      this.#queued = null;
      this.#sending = false;
      this.#value = null;
    }
  }
}
