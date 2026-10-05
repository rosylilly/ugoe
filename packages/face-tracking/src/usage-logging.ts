// MediaPipe の利用状況の送信を止める (T0943)。
//
// @mediapipe/tasks-vision (1.0.1) は、Face Landmarker などを作ると利用状況の記録 (タスクの種類・実行モード・OS・版・
// 処理時間の統計。画像やランドマークは含まない) を溜め、60 秒ごとと閉じたときに `https://odml.pa.googleapis.com/v1/log`
// へ `fetch` で POST する。送らせない設定は無い (作るときに必ず記録を始める)。送り先は JS の側で決め打ちで、wasm の
// 3 つの版 (SIMD・SIMD 無し・module) のどれを選んでも同じ。
//
// 送るときは呼ぶたびにグローバルの `fetch` を引くので、その `fetch` を包み、この送り先への要求だけをネットワークの失敗
// (`TypeError`) として断る。MediaPipe は送れなかった記録を捨て、以後は送らない (60 秒ごとの送信も止める)。
// 要求はネットワークに出ないので、アプリの CSP (`connect-src`) の違反にもならない。CSP は二重の守りとして残す。
// 詳しくは README.md の「利用状況の送信」。

/** MediaPipe が利用状況を送る先 (オリジン)。 */
export const MEDIAPIPE_USAGE_LOG_ORIGIN = "https://odml.pa.googleapis.com";

const GUARD = Symbol.for("ugoe.face-tracking.usage-logging-guard");

type FetchHost = { fetch: typeof fetch };

function requestUrl(input: Parameters<typeof fetch>[0]): string {
  if (typeof input === "string") return input;
  if (input instanceof URL) return input.href;
  return input.url;
}

/** `input` が MediaPipe の利用状況の送り先への要求か。 */
export function isMediaPipeUsageLog(input: Parameters<typeof fetch>[0]): boolean {
  try {
    return new URL(requestUrl(input)).origin === MEDIAPIPE_USAGE_LOG_ORIGIN;
  } catch {
    return false;
  }
}

/**
 * `host.fetch` (既定はグローバル) を包み、MediaPipe の利用状況の送り先への要求を断る。ほかの要求はそのまま通す。
 * 何度呼んでも包むのは 1 回だけ。MediaPipe の Landmarker を作る前に呼ぶ。
 */
export function blockMediaPipeUsageLogging(host: FetchHost = globalThis): void {
  const original = host.fetch;
  if (typeof original !== "function" || GUARD in original) return;
  const guarded = (input: Parameters<typeof fetch>[0], init?: RequestInit) =>
    isMediaPipeUsageLog(input)
      ? // i18n-ignore: MediaPipe が受け取って捨てる (画面には出ない)
        Promise.reject(new TypeError("MediaPipe usage logging is disabled by @ugoe/face-tracking"))
      : original.call(host, input, init);
  Object.defineProperty(guarded, GUARD, { value: true });
  host.fetch = guarded as typeof fetch;
}
