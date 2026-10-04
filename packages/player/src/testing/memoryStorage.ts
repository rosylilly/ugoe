// テスト用の部品 (配布物に入らない。src/index.ts からは出さない)。

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
}
