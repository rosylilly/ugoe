// テスト用の部品 (`@ugoe/i18n/testing`。配布物の画面からは import しない。T0937)。
// パッケージとアプリのテストで localStorage の代わりに使う。

/** localStorage の代わり (Map に覚える)。 */
export class MemoryStorage implements Storage {
  #items = new Map<string, string>();
  get length(): number {
    return this.#items.size;
  }
  clear(): void {
    this.#items.clear();
  }
  getItem(key: string): string | null {
    return this.#items.get(key) ?? null;
  }
  key(index: number): string | null {
    return [...this.#items.keys()][index] ?? null;
  }
  removeItem(key: string): void {
    this.#items.delete(key);
  }
  setItem(key: string, value: string): void {
    this.#items.set(key, String(value));
  }
  /** 覚えているキーの一覧 (テストで比べる)。 */
  keys(): string[] {
    return [...this.#items.keys()];
  }
}

/** 引くと例外になる localStorage (保存を禁止したブラウザ) の代わり。 */
export class BrokenStorage implements Storage {
  get length(): number {
    throw new Error("blocked");
  }
  clear(): void {
    throw new Error("blocked");
  }
  getItem(_key: string): string | null {
    throw new Error("blocked");
  }
  key(_index: number): string | null {
    throw new Error("blocked");
  }
  removeItem(_key: string): void {
    throw new Error("blocked");
  }
  setItem(_key: string, _value: string): void {
    throw new Error("blocked");
  }
}

/** `globalThis.localStorage` を置き換える (`undefined` で無い環境)。 */
export function useLocalStorage(storage: Storage | undefined): void {
  Object.defineProperty(globalThis, "localStorage", { value: storage, configurable: true });
}
