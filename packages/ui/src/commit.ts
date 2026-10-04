// 数値の欄の確定の印 (docs/ui.md#数値の部品と-onchange)。NumberField は onchange を呼んでいる間だけ、
// 欄ごとに一意な印を置く。受け手 (エディタの編集の列) は onchange の中で同期的に読み、同じ欄の続けての確定を
// 1 つにまとめるのに使う (docs/editor.md#undo)。

let current: string | null = null;

/**
 * いま `onchange` を呼んでいる `NumberField` の印 (欄ごとに一意)。`onchange` の中で同期的に呼んだときだけ値があり、
 * それ以外 (await の後・別の部品の操作) では null。
 */
export function numberFieldCommit(): string | null {
  return current;
}

/** `field` を印に置いて `run` を呼ぶ (NumberField の中で使う)。 */
export function withNumberFieldCommit(field: string, run: () => void): void {
  const previous = current;
  current = field;
  try {
    run();
  } finally {
    current = previous;
  }
}
