// Tauri の Channel の受け取り (T0812)。
//
// `@tauri-apps/api` の `Channel` は、`onmessage` が一度でも例外を投げると、次に配る番号を進めないまま止まり、
// 以後のメッセージをすべて溜めて配らなくなる。受け取る処理は、例外を外へ出さない形にして渡す。

/** `handler` を、投げた例外を console に出して外へ出さない形にする (Channel の `onmessage` に渡す)。 */
export function safeChannelHandler<T>(handler: (message: T) => void): (message: T) => void {
  return (message) => {
    try {
      handler(message);
    } catch (e) {
      console.error(e);
    }
  };
}
