// BaseLock (本体のものの読み取り専用) の状態を中の部品に伝える。fieldset の disabled は <input>・<button> にしか効かず、
// @ugoe/ui の Slider のつまみ (Bits UI の span) は動いてしまうので、Slider などには `disabled={locked()}` で渡す。
import { getContext, setContext } from "svelte";

const KEY = Symbol("base-lock");

/** BaseLock が中の部品に状態を渡す (BaseLock だけが呼ぶ)。 */
export function provideBaseLock(locked: () => boolean): void {
  setContext(KEY, locked);
}

/** 囲む BaseLock が無効にしているかを返す関数 (部品の初期化の中で呼ぶ)。BaseLock の外では常に false。 */
export function useBaseLock(): () => boolean {
  const outer = getContext<(() => boolean) | undefined>(KEY);
  return () => outer?.() ?? false;
}
