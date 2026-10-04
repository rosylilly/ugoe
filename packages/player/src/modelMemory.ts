// モデルごとに覚える設定 (T0711 の着せ替えの組、T0709 の光源の設定) の共通の部品。
// モデルを見分ける鍵 (`ModelIdentity`) と、localStorage に上限付きで覚える入れ物 (`ModelMemory`)。
//
// 鍵は「読み直せる場所」と「中身の指紋」の組で、どちらかが同じなら同じモデルとみなす (`sameModel`)。
// - 場所 (`ModelFile.location`) が同じなら、中身が変わっていても同じモデル (エディタで書き出し直したファイル)。
// - 場所が分からない (ドロップ・ブラウザの `<input type="file">`) か違うときは、中身の指紋で比べる
//   (同じファイルをダイアログで開いてもドロップしても同じモデルになる)。
// 名前は表示だけに使い、比べない (`model.ugoe` のような同じ名前の別のモデルがよくあるため)。
import { type FileLocation, isFileLocation, type ModelFile } from "./modelFile";
import type { PlayerStorage, StorageName } from "./storage";

/** モデル (やモジュール) のファイルを見分ける鍵。 */
export interface ModelIdentity {
  /** 表示用の名前 (ファイル名)。比べない。 */
  name: string;
  /** 中身の指紋 (`fingerprint`)。 */
  fingerprint: string;
  /** 読み直せる場所。分からなければ無い。 */
  location?: FileLocation;
}

/**
 * バイト列の指紋。長さと、2 通りの 32 ビットのハッシュ (FNV-1a と、乗算で混ぜるもの) を 16 進で並べる。
 * 暗号のハッシュではない (同じファイルかを見分けるだけ)。同期で計算でき、数 MB で数ミリ秒。
 */
export function fingerprint(bytes: Uint8Array): string {
  let a = 0x811c9dc5;
  let b = 0x9e3779b9;
  for (let i = 0; i < bytes.length; i++) {
    const byte = bytes[i] as number;
    a = Math.imul(a ^ byte, 0x01000193);
    b = Math.imul(b ^ byte, 0x85ebca6b);
    b = (b << 13) | (b >>> 19);
  }
  const hex = (n: number) => (n >>> 0).toString(16).padStart(8, "0");
  return `${bytes.length.toString(16)}-${hex(a)}${hex(b)}`;
}

/** ファイルの鍵を作る。 */
export function modelIdentity(file: ModelFile): ModelIdentity {
  const identity: ModelIdentity = { name: file.name, fingerprint: fingerprint(file.bytes) };
  if (file.location !== undefined) identity.location = file.location;
  return identity;
}

/** 同じモデルか: 場所が同じか、中身の指紋が同じ。 */
export function sameModel(a: ModelIdentity, b: ModelIdentity): boolean {
  return (a.location !== undefined && a.location === b.location) || a.fingerprint === b.fingerprint;
}

/** 保存した鍵を読む。形が違えば `null`。 */
export function parseIdentity(v: unknown): ModelIdentity | null {
  if (typeof v !== "object" || v === null) return null;
  const r = v as Record<string, unknown>;
  if (typeof r.fingerprint !== "string" || r.fingerprint === "") return null;
  const identity: ModelIdentity = {
    name: typeof r.name === "string" ? r.name : "",
    fingerprint: r.fingerprint,
  };
  if (isFileLocation(r.location)) identity.location = r.location;
  return identity;
}

export interface ModelMemoryOptions {
  /** 覚えるモデルの数の上限。超えたら最も長く使っていないものから消す。 */
  limit: number;
  /** 覚えた日時に使う今の時刻 (ミリ秒)。省くと `Date.now`。 */
  now?: () => number;
}

/** 保存の形の版。変えたら古いものは読まずに捨てる。 */
const STORAGE_VERSION = 1;

/** 覚えた項目。`at` は最後に覚えた日時 (ミリ秒。T0717)。日時を持たない前の保存値から読んだ項目には無い。 */
export interface MemoryEntry<T> {
  model: ModelIdentity;
  value: T;
  at?: number;
}

type Entry<T> = MemoryEntry<T>;

/**
 * モデルごとの値を localStorage の 1 つのキー (`store` の `name`) に覚える。値の形は `parse` が確かめる (形の違う項目は読み飛ばす)。
 * 保存の形は `{"version": 1, "entries": [{"model": ModelIdentity, "value": T, "at": number}]}` で、最近使った順。
 * `at` (覚えた日時、T0717) は後から足したので無くてもよい (版は変えない。無い項目は日時なしとして読む)。
 * localStorage が使えない・壊れた値・版の違いでは、何も覚えていないものとして動き、例外を出さない。
 */
export class ModelMemory<T> {
  readonly #store: PlayerStorage;
  readonly #name: StorageName;
  readonly #parse: (value: unknown) => T | null;
  readonly #limit: number;
  readonly #now: () => number;

  constructor(
    store: PlayerStorage,
    name: StorageName,
    parse: (value: unknown) => T | null,
    options: ModelMemoryOptions,
  ) {
    this.#store = store;
    this.#name = name;
    this.#parse = parse;
    this.#limit = Math.max(1, Math.floor(options.limit));
    this.#now = options.now ?? Date.now;
  }

  /** `model` の値。場所が同じ項目を優先し、無ければ指紋が同じ項目。覚えていなければ `null`。 */
  get(model: ModelIdentity): T | null {
    const entries = this.#read();
    const found =
      (model.location !== undefined
        ? entries.find((e) => e.model.location === model.location)
        : undefined) ?? entries.find((e) => e.model.fingerprint === model.fingerprint);
    return found?.value ?? null;
  }

  /** `model` の値を覚える (最近使ったものにする)。同じモデルの古い項目は、鍵も新しいものに置き換える。 */
  set(model: ModelIdentity, value: T): void {
    const rest = this.#read().filter((e) => !sameModel(model, e.model));
    this.#write([{ model, value, at: this.#now() }, ...rest].slice(0, this.#limit));
  }

  /** `model` の値を忘れる (覚えたものの一覧・「覚えた組を忘れる」)。 */
  delete(model: ModelIdentity): void {
    const entries = this.#read();
    const rest = entries.filter((e) => !sameModel(model, e.model));
    if (rest.length !== entries.length) this.#write(rest);
  }

  /** 覚えているモデル (最近使った順)。 */
  models(): ModelIdentity[] {
    return this.#read().map((e) => e.model);
  }

  /** 覚えているモデルと値と覚えた日時 (最近使った順。覚えたものの一覧で使う、T0714・T0717)。 */
  entries(): MemoryEntry<T>[] {
    return this.#read();
  }

  /** すべて忘れる。 */
  clear(): void {
    this.#store.remove(this.#name);
  }

  #read(): Entry<T>[] {
    const saved = this.#store.getJson(this.#name);
    if (typeof saved !== "object" || saved === null) return [];
    const { version, entries } = saved as Record<string, unknown>;
    if (version !== STORAGE_VERSION || !Array.isArray(entries)) return [];
    return entries
      .flatMap((item): Entry<T>[] => {
        if (typeof item !== "object" || item === null) return [];
        const r = item as Record<string, unknown>;
        const model = parseIdentity(r.model);
        if (model === null) return [];
        const value = this.#parseValue(r.value);
        if (value === null) return [];
        const entry: Entry<T> = { model, value };
        if (typeof r.at === "number" && Number.isFinite(r.at)) entry.at = r.at;
        return [entry];
      })
      .slice(0, this.#limit);
  }

  /** 値を読む。`parse` が例外を出しても、その項目だけを読み飛ばす。 */
  #parseValue(value: unknown): T | null {
    try {
      return this.#parse(value);
    } catch {
      return null;
    }
  }

  #write(entries: Entry<T>[]): void {
    this.#store.setJson(this.#name, { version: STORAGE_VERSION, entries });
  }
}
