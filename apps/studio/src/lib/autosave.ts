// プロファイルの保存の決まり (docs/studio.md#書き換えないプロファイル)。保存先があり変えたプロファイルは、少し待ってから
// 自動で保存する。書き換えないプロファイル (見本の印・書けない場所。T0866) は自動で保存せず、保存は保存先を選ばせる (写しを保存)。
// 一時の失敗 (ディスクが一杯など) では、少し待ってもう一度保存する (T0870)。
import type { ProfileView } from "./profile";

type SaveState = Pick<ProfileView, "path" | "dirty" | "readOnly">;

/** 自動で保存するか (保存先があり、書き換えないプロファイルでなく、保存した後に変えた)。 */
export function shouldAutosave(view: SaveState): boolean {
  return view.path !== null && view.readOnly === null && view.dirty;
}

/** 「保存」で保存先を選ばせるか (「名前を付けて保存」・保存先が無い・書き換えないプロファイル)。 */
export function needsSavePath(view: SaveState | null, saveAs: boolean): boolean {
  return saveAs || view === null || view.path === null || view.readOnly !== null;
}

/** 保存していない変更が残ったままか (自動で保存しないプロファイルを変えた。別のプロファイルを開く前に確かめる)。 */
export function hasUnsavedChanges(view: SaveState | null): boolean {
  return view?.dirty === true && (view.path === null || view.readOnly !== null);
}

/** 自動の保存の振る舞い。 */
export interface AutosaveOptions {
  /** 今の保存先へ保存する (失敗したら reject する)。 */
  save: () => Promise<unknown>;
  /** 変えてから保存するまで待つ時間。 */
  delayMs: number;
  /** 保存に失敗している間、次に保存するまで待つ時間 (失敗し続ける保存を繰り返さないように。T0870)。 */
  retryMs: number;
  /** 保存の失敗を知らせる。同じ失敗 (`failureKey` が同じ) が続く間は、最初の 1 度だけ呼ぶ。 */
  onError: (error: unknown) => void;
  /** 失敗の後に保存できた (か、保存した・別のプロファイルにした) ので、知らせを消す。 */
  onRecovered: () => void;
  /** 同じ失敗かを見分ける鍵 (既定は `String(error)`)。 */
  failureKey?: (error: unknown) => string;
}

/**
 * 自動の保存。プロファイルが変わるたびに `schedule` を呼ぶと、`delayMs` 待ってから `save` (今の保存先へ) を呼ぶ。
 *
 * 保存に失敗したら (ディスクが一杯などの一時の失敗。書けない場所ならバックエンドが書き換えないプロファイルにするので、
 * 次の `schedule` で止まる)、知らせて `retryMs` 後にもう一度保存する。失敗している間は変えても待ち直さない
 * (変え続けても、`retryMs` ごとに 1 度は試す)。同じ失敗が続く間は知らせを繰り返さない (T0870)。
 */
export class Autosave {
  #timer: ReturnType<typeof setTimeout> | undefined;
  readonly #options: AutosaveOptions;
  /** 続いている失敗の鍵と保存先 (失敗していなければ `null`)。 */
  #failure: { key: string; path: string | null } | null = null;
  /** 最後に `schedule` に渡したプロファイル。 */
  #view: SaveState | null = null;

  constructor(options: AutosaveOptions) {
    this.#options = options;
  }

  /** 保存に失敗しているか。 */
  get failing(): boolean {
    return this.#failure !== null;
  }

  /** 待っている保存をやめ、`view` を自動で保存するなら待ち直す (失敗している間は、待っている保存を残す)。 */
  schedule(view: SaveState | null) {
    this.#view = view;
    // 保存した・別のプロファイルにした・保存先が変わった (写しを保存した) なら、失敗は終わった。
    if (
      this.#failure !== null &&
      (view === null || view.path !== this.#failure.path || !view.dirty)
    ) {
      this.#recover();
    }
    if (view === null || !shouldAutosave(view)) {
      this.cancel();
      return;
    }
    if (this.#failure !== null && this.#timer !== undefined) return;
    this.#arm(this.#failure === null ? this.#options.delayMs : this.#options.retryMs);
  }

  cancel() {
    clearTimeout(this.#timer);
    this.#timer = undefined;
  }

  #arm(delayMs: number) {
    this.cancel();
    this.#timer = setTimeout(() => {
      this.#timer = undefined;
      void this.#run();
    }, delayMs);
  }

  async #run() {
    const path = this.#view?.path ?? null;
    try {
      await this.#options.save();
    } catch (e) {
      const key = (this.#options.failureKey ?? String)(e);
      const repeated = this.#failure?.key === key && this.#failure.path === path;
      this.#failure = { key, path };
      if (!repeated) this.#options.onError(e);
      // 失敗の後の間隔で待ち直す (保存の通知が先に届いて待ち始めた保存も)。書き換えないプロファイルになったなら待たない。
      if (this.#view !== null && shouldAutosave(this.#view)) this.#arm(this.#options.retryMs);
      return;
    }
    if (this.#failure !== null) this.#recover();
  }

  #recover() {
    this.#failure = null;
    this.#options.onRecovered();
  }
}
