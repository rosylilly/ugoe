// プロファイルの保存の決まり (docs/studio.md#書き換えないプロファイル)。保存先があり変えたプロファイルは、少し待ってから
// 自動で保存する。書き換えないプロファイル (見本の印・書けない場所。T0866) は自動で保存せず、保存は保存先を選ばせる (写しを保存)。
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

/** 自動の保存。プロファイルが変わるたびに `schedule` を呼ぶと、`delayMs` 待ってから `save` (今の保存先へ) を呼ぶ。 */
export class Autosave {
  #timer: ReturnType<typeof setTimeout> | undefined;
  readonly #save: () => unknown;
  readonly #delayMs: number;

  constructor(save: () => unknown, delayMs: number) {
    this.#save = save;
    this.#delayMs = delayMs;
  }

  /** 待っている保存をやめ、`view` を自動で保存するなら待ち直す。 */
  schedule(view: SaveState | null) {
    this.cancel();
    if (view === null || !shouldAutosave(view)) return;
    this.#timer = setTimeout(() => {
      this.#timer = undefined;
      this.#save();
    }, this.#delayMs);
  }

  cancel() {
    clearTimeout(this.#timer);
    this.#timer = undefined;
  }
}
