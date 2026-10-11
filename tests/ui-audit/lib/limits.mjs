// 時間の上限 (T0884): 組ごとの上限 (`--variant-timeout`) と、全体の締め切り (`--deadline`)。
// 止まった場面や混んだ機械で、全体が呼び手の時間切れ (エージェントが前面で流す Bash の 600 秒など) で殺されて報告が残らない、
// ということが無いようにする。
// 場面の中の条件の待ち (ダイアログ・メニュー・帯などが出る・消えるのを待つ) の上限 (`WAIT_TIMEOUT_MS`・`waitUntil`。T0985) もここに置く。

/** 上限の時間を過ぎた (`withTimeLimit`)。 */
export class TimeLimitError extends Error {
  constructor(ms) {
    super(`timed out after ${Math.round(ms / 1000)}s`);
    this.name = "TimeLimitError";
    this.ms = ms;
  }
}

/**
 * `body()` を `ms` ミリ秒まで待つ。過ぎたら `abort()` (ページのコンテキストを閉じるなど) を呼び、`TimeLimitError` で reject する
 * (`body` が `abort` で止まらなくても待たない。過ぎた後の `body` の失敗は捨てる)。`ms` が有限でなければ上限なしで待つ。
 */
export async function withTimeLimit(body, ms, abort = () => {}) {
  if (!Number.isFinite(ms)) return body();
  let timer;
  const expired = new Promise((_, reject) => {
    timer = setTimeout(
      () => {
        // 先に決める (止めたことで `body` が先に失敗しても、上限を過ぎたことを返す)
        reject(new TimeLimitError(ms));
        abort();
      },
      Math.max(0, ms),
    );
  });
  const run = body();
  run.catch(() => {});
  try {
    return await Promise.race([run, expired]);
  } finally {
    clearTimeout(timer);
  }
}

/**
 * 全体の締め切り。`seconds` が `null` なら締め切りなし。`started` は数え始めた時刻 (`performance.now()` の値。既定は
 * プロセスの起動、つまりビルドの時間も含む)。
 */
export function deadline(seconds, { started = 0, now = () => performance.now() } = {}) {
  const end = seconds === null ? Number.POSITIVE_INFINITY : started + seconds * 1000;
  return {
    /** 残りのミリ秒 (締め切りなしなら無限大、過ぎたら 0 以下)。 */
    remaining: () => end - now(),
    /** 締め切りを過ぎたか。 */
    passed: () => end - now() <= 0,
  };
}

/** 秒の数のオプション (`--deadline 540` など) を読む。省いたら `fallback`、`0` は上限なし (`null`)。負の数や数でないものは例外。 */
export function secondsOption(value, name, fallback = null) {
  if (value === undefined || value === null) return fallback;
  const n = Number(value);
  if (!(Number.isFinite(n) && n >= 0))
    throw new Error(`${name} must be a non-negative number of seconds (got ${value})`);
  return n === 0 ? null : n;
}

/**
 * 場面の中の条件の待ち (ダイアログ・メニュー・Popover・帯・強調などが出る・消えるのを待つ) の上限 (ミリ秒。T0985)。
 * ふつうは 1 秒ほどで出るが、混んだ機械 (ロードアベレージ 30 を超える・ほかの作業と並行・3 つのアプリをまとめる回) では 5 秒を
 * 超えうる。出るまで (消えるまで) の条件の待ちなので、長くしても通る回は遅くならない (遅くなるのは時間切れになる回だけ)。
 */
export const WAIT_TIMEOUT_MS = 20000;

/** 条件の待ちが上限を過ぎた (`waitUntil`)。 */
export class WaitTimeoutError extends Error {
  constructor(what, ms, hint = null) {
    super(
      `timed out after ${(ms / 1000).toFixed(1)}s waiting for ${what}${hint === null ? "" : ` (${hint})`}`,
    );
    this.name = "WaitTimeoutError";
    this.what = what;
    this.ms = ms;
  }
}

/**
 * 条件を待つ。`wait(timeout)` は上限 (ミリ秒) を受け取って条件を待つもの (Playwright の `locator.waitFor`・`page.waitForFunction`
 * など。上限を過ぎたら名前が `TimeoutError` の例外を投げる)。上限を過ぎたら、`what` (何を待っていたか。例: `the File menu to open`)
 * と待った時間と、`hint()` (画面にあった手がかりの文。省ける) を添えた `WaitTimeoutError` にする (場面の失敗の文はその 1 行目
 * だけなので、Playwright の `Timeout 5000ms exceeded.` だけでは何を待っていたかが分からない)。ほかの失敗はそのまま投げる。
 */
export async function waitUntil(
  what,
  wait,
  { timeout = WAIT_TIMEOUT_MS, hint = null, now = () => performance.now() } = {},
) {
  const started = now();
  try {
    return await wait(timeout);
  } catch (e) {
    if (e?.name !== "TimeoutError") throw e;
    const found = hint === null ? null : await Promise.resolve(hint()).catch(() => null);
    throw new WaitTimeoutError(what, now() - started, found);
  }
}
