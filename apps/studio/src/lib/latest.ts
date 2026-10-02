// 送り先が受け取り終えるまで、次の値を 1 つだけ待たせる (トラッキングと口パクの値を IPC で送るとき。T0803)。
// 送っている間に届いた値は最後のものだけを次に送り、古い値を溜めない。

/** `send` を包む。返した関数は値を受け取り、送っている途中なら最後の値だけを覚えて後で送る。 */
export function latestOnly<T>(send: (value: T) => Promise<unknown>): (value: T) => void {
  let pending: { value: T } | null = null;
  let sending = false;
  return (value: T) => {
    pending = { value };
    if (sending) return;
    sending = true;
    void (async () => {
      while (pending !== null) {
        const next: T = pending.value;
        pending = null;
        try {
          await send(next);
        } catch {
          // 送れなかった値は捨てる (次の値で追いつく)。
        }
      }
      sending = false;
    })();
  };
}
