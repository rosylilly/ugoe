// アプリと部品が localStorage に覚える設定の読み書き (T0937)。言語 (preference.ts)・テーマ (@ugoe/ui)・
// Player が覚えるもの (@ugoe/player の PlayerStorage)・アプリの古い設定の片付けが使う。
// 翻訳とは別の役目だが、3 つのアプリとすべての共有の部品が依存する一番下のパッケージなのでここに置く。
//
// localStorage は引くだけで例外になる環境 (保存を禁止したブラウザ) や、書くと容量で例外になることがある。
// ここの読み書きはすべて try の中で行い、読めなければ「覚えていない」、書けなければ何もしない (例外を出さない)。

/** `storage` を省くと、呼ぶたびに `globalThis.localStorage` を引く (引くだけで例外になる環境があるので try の中で)。 */
function target(storage: Storage | undefined): Storage | undefined {
  return storage ?? globalThis.localStorage;
}

/** 覚えた文字列。覚えていない・読めなければ `null`。 */
export function readStoredItem(key: string, storage?: Storage): string | null {
  try {
    return target(storage)?.getItem(key) ?? null;
  } catch {
    return null;
  }
}

/** 文字列を覚える。覚えられなくても (プライベートモード・容量) 無視する (今回の値は効く)。 */
export function writeStoredItem(key: string, value: string, storage?: Storage): void {
  try {
    target(storage)?.setItem(key, value);
  } catch {
    // 覚えられないだけで、今回の値は効く。
  }
}

/** 忘れる。忘れられなくても無視する。 */
export function removeStoredItem(key: string, storage?: Storage): void {
  try {
    target(storage)?.removeItem(key);
  } catch {
    // 忘れられないだけ。
  }
}
