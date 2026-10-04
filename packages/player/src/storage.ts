// Player が覚えるもの (光源・着せ替えの組・待機モーションなど) の localStorage の読み書き (T0930)。
// キーは「アプリが渡す接頭辞」+ "." + 名前 (`STORAGE_NAMES`)。接頭辞を省くとビューアの `ugoe-viewer`
// (前からのキーのまま。ビューアが覚えたものを失わない)。
//
// localStorage は引くだけで例外になる環境 (保存を禁止したブラウザ) や、書くと容量で例外になることがある。
// ここの読み書きはすべて try の中で行い、読めなければ「覚えていない」、書けなければ何もしない (例外を出さない)。

/** 接頭辞を省いたときの接頭辞 (ビューア)。 */
export const DEFAULT_STORAGE_PREFIX = "ugoe-viewer";

/** Player が覚えるものの名前 (キーの接頭辞の後ろ)。 */
export const STORAGE_NAMES = {
  /** 最後に使った光源の設定 (lighting.ts)。 */
  lighting: "lighting",
  /** モデルごとの光源の設定 (T0709)。 */
  modelLighting: "lighting.models",
  /** ビューポートに光源の印を出すか (T0708)。 */
  lightMarkers: "lighting.markers",
  /** モデルごとの着せ替えの組 (T0711)。 */
  dressUp: "dress-up",
  /** 着せ替えの組を尋ねずに戻すか。 */
  autoRestore: "dress-up.auto-restore",
  /** モデルごとの待機モーションの選択 (T0722)。 */
  idleMotion: "idle-motion.models",
  /** 待機モーションを自動で流すか (T0721)。 */
  autoIdle: "auto-idle",
  /** トラッキング中もモーションを優先するか (T0721)。 */
  motionOverTracking: "motion-over-tracking",
  /** モーションを優先しても、トラッキングのままにするパラメータ (T0722)。 */
  trackingKept: "motion-over-tracking.kept",
  /** 覚えたものの一覧の並び (T0718)。 */
  rememberedOrder: "remembered-order",
} as const;

export type StorageName = (typeof STORAGE_NAMES)[keyof typeof STORAGE_NAMES];

/** 接頭辞と `Storage` の組。Player が覚えるものの読み書きはすべてこれを通す。 */
export class PlayerStorage {
  readonly prefix: string;
  readonly #storage: Storage | undefined;

  /** `storage` を省くと、読み書きのたびに `globalThis.localStorage` を引く。 */
  constructor(prefix: string = DEFAULT_STORAGE_PREFIX, storage?: Storage) {
    this.prefix = prefix;
    this.#storage = storage;
  }

  /** `name` の localStorage のキー。 */
  key(name: StorageName): string {
    return `${this.prefix}.${name}`;
  }

  /** 覚えた文字列。覚えていない・読めなければ `null`。 */
  get(name: StorageName): string | null {
    try {
      return this.#target()?.getItem(this.key(name)) ?? null;
    } catch {
      return null;
    }
  }

  /** 文字列を覚える。覚えられなくても無視する (今回の値は効く)。 */
  set(name: StorageName, value: string): void {
    try {
      this.#target()?.setItem(this.key(name), value);
    } catch {
      // 覚えられないだけで、今回の値は効く。
    }
  }

  /** 忘れる。忘れられなくても無視する。 */
  remove(name: StorageName): void {
    try {
      this.#target()?.removeItem(this.key(name));
    } catch {
      // 忘れられないだけで、表示には影響しない。
    }
  }

  /** 覚えた JSON の値。覚えていない・読めない・壊れた JSON なら `undefined`。 */
  getJson(name: StorageName): unknown {
    const text = this.get(name);
    if (text === null) return undefined;
    try {
      return JSON.parse(text) as unknown;
    } catch {
      return undefined;
    }
  }

  /** 値を JSON で覚える。 */
  setJson(name: StorageName, value: unknown): void {
    this.set(name, JSON.stringify(value));
  }

  /** 覚えたオン・オフ (`"on"`・`"off"`)。覚えていない・読めない・知らない値なら `fallback`。 */
  getToggle(name: StorageName, fallback: boolean): boolean {
    const value = this.get(name);
    return value === "on" ? true : value === "off" ? false : fallback;
  }

  /** オン・オフを覚える。 */
  setToggle(name: StorageName, value: boolean): void {
    this.set(name, value ? "on" : "off");
  }

  /** この接頭辞で Player が覚えたものをすべて忘れる (アプリの古い値を捨てるとき)。 */
  clear(): void {
    for (const name of Object.values(STORAGE_NAMES)) this.remove(name);
  }

  #target(): Storage | undefined {
    // 引くだけで例外になる環境があるので、呼ぶ側の try の中で引く。
    return this.#storage ?? globalThis.localStorage;
  }
}
