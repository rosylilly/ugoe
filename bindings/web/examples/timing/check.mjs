// 計測の結果をまとめ、上限と比べる (measure.mjs の `--max-us`。T0194、docs/testing.md#遅くなったら気づく検査)。
// ページを開かない部分だけをここに置き、check.test.mjs で確かめる。

/** 上限と比べる計測 (ネイティブの puppet_timing.rs の `LIMITS` と同じ `update` と `follow`)。 */
export const CHECKED = ["update", "follow"];

/**
 * 上限を超えたときにページを開き直して測り直す回数。ほかの処理と重なった一時の揺れで落ちないように、最も速い中央値で判定する
 * (puppet_timing.rs の `RETRIES` と同じ)。
 */
export const RETRIES = 2;

export const median = (list) => [...list].sort((a, b) => a - b)[Math.floor(list.length / 2)];

/**
 * 回ごとの結果 (ページの `window.ugoeTiming.results` の列) を、計測ごとの中央値の中央値にまとめる。
 * @param {{ model: string, measurement: string, best: number, median: number }[][]} runs
 */
export function summarize(runs) {
  return runs[0].map((r, i) => ({
    model: r.model,
    measurement: r.measurement,
    best: median(runs.map((results) => results[i].best)),
    median: median(runs.map((results) => results[i].median)),
  }));
}

/**
 * `summary` (`summarize` の結果) のうち `CHECKED` の計測の中央値を上限 `limitUs` と比べる。超えたものがあれば、
 * `remeasure()` (ページを 1 回開いた結果を返す) を `retries` 回まで呼び、行ごとに最も速い中央値で判定し直す。
 * @returns {Promise<{ model: string, measurement: string, median: number, limit: number, ok: boolean }[]>}
 */
export async function checkLimit(summary, limitUs, remeasure, retries = RETRIES) {
  const rows = summary
    .filter((r) => CHECKED.includes(r.measurement))
    .map((r) => ({ model: r.model, measurement: r.measurement, median: r.median }));
  for (let retry = 0; retry < retries && rows.some((r) => !(r.median <= limitUs)); retry++) {
    const results = await remeasure();
    for (const row of rows) {
      const again = results.find((r) => r.model === row.model && r.measurement === row.measurement);
      if (again && again.median < row.median) row.median = again.median;
    }
  }
  return rows.map((r) => ({ ...r, limit: limitUs, ok: r.median <= limitUs }));
}
