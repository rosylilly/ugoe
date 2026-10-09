// 重い検出の下でトラッキングを流す長さ (スタジオの e2e の T0812 の手順。tests/e2e/README.md#長く流す)。
//
// 普段は 13 秒で、「トラッキングの待ち」の値は約 17 個しか集まらず、遅れた値の割合が回ごとに 0〜28% と揺れる (docs/tasks/T0873)。
// pacing.rs を変えた前と後を比べるときだけ、環境変数 `UGOE_E2E_TRACKING_MS` で長く流す (T0876)。
// run.mjs (アプリをビルドする前に確かめる) と specs/studio.test.mjs の両方から読むので、ほかのモジュールを読み込まない。

/** 環境変数の名前。 */
export const TRACKING_MS_ENV = "UGOE_E2E_TRACKING_MS";
/** 既定の長さ (ミリ秒)。 */
export const DEFAULT_TRACKING_MS = 13_000;
/**
 * 長さの上限 (ミリ秒)。120 秒で値は約 280 個 (割合の 1 個分が約 0.4%) 集まり、前と後を比べるのに足りる。
 * これより長くすると 1 回の e2e が数分になり、その間は計算機の重さ (ほかのビルド) の変化も混ざるので、回を重ねる方がよい。
 */
export const MAX_TRACKING_MS = 120_000;
/** 届いた回数の下限 (既定の長さのとき)。流す長さに比例させる (`minTrackingDone`)。 */
const MIN_DONE = 10;

/**
 * 環境変数から流す長さ (ミリ秒) を読む。無い・空なら既定の長さ。整数のミリ秒で、既定の長さ (13 秒) から上限 (120 秒) まで。
 * 既定より短くすると「トラッキングの待ち」を読む時間 (6 秒から) がほとんど無くなるので許さない。
 * @param {Record<string, string | undefined>} env
 * @returns {number}
 */
export function trackingMs(env = process.env) {
  const raw = env[TRACKING_MS_ENV];
  if (raw === undefined || raw.trim() === "") return DEFAULT_TRACKING_MS;
  const text = raw.trim();
  const ms = /^\d+$/.test(text) ? Number(text) : Number.NaN;
  if (!Number.isSafeInteger(ms) || ms < DEFAULT_TRACKING_MS || ms > MAX_TRACKING_MS) {
    throw new Error(
      `${TRACKING_MS_ENV} must be whole milliseconds from ${DEFAULT_TRACKING_MS} to ${MAX_TRACKING_MS}: ${JSON.stringify(raw)}`,
    );
  }
  return ms;
}

/**
 * 流す長さで、`set_tracking` がバックエンドに届くべき回数の下限。既定の 13 秒で 10 回 (約 0.77 回/秒) の割合を、長さに比例させる。
 * 下限を今の 10 回のままにすると、長く流したときに「直す前は 50 秒で 3 回」(T0812) に近い遅さでも通ってしまう。
 * @param {number} ms
 */
export function minTrackingDone(ms) {
  return Math.ceil((MIN_DONE * ms) / DEFAULT_TRACKING_MS);
}
