// 時間の上限 (T0884): 組ごとの上限 (`--variant-timeout`) と、全体の締め切り (`--deadline`)。
// 止まった場面や混んだ機械で、全体が呼び手の時間切れ (エージェントが前面で流す Bash の 600 秒など) で殺されて報告が残らない、
// ということが無いようにする。

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
