// 部品の間で受け渡す Svelte の context。

import { getContext, setContext } from "svelte";

const TOOLBAR = Symbol("ugoe-ui-toolbar");

/** Toolbar の中にあることを、中の Button・IconButton に知らせる (roving tabindex のため Bits UI の Toolbar.Button で出す)。 */
export function setInToolbar(): void {
  setContext(TOOLBAR, true);
}

/** Toolbar の中にあるか。 */
export function isInToolbar(): boolean {
  return getContext<boolean | undefined>(TOOLBAR) === true;
}

const TOOLTIP_PROVIDER = Symbol("ugoe-ui-tooltip-provider");

/** TooltipProvider の中にあることを、中の Tooltip に知らせる (無ければ Tooltip が自分で Bits UI の Provider を置く)。 */
export function setInTooltipProvider(): void {
  setContext(TOOLTIP_PROVIDER, true);
}

/** TooltipProvider の中にあるか。 */
export function isInTooltipProvider(): boolean {
  return getContext<boolean | undefined>(TOOLTIP_PROVIDER) === true;
}

/** ツールチップを出さずにフォーカスを移している要素 (`focusQuietly` の間だけ入る)。 */
const quietFocus = new WeakSet<Element>();

/**
 * ツールチップを出さずにフォーカスを移す (Dialog の `initialFocus`)。開いたとたんに説明が重なり、最初の Esc が説明を
 * 消すだけになる (ダイアログを閉じない) のを防ぐ。focus イベントは `focus()` の中で同期に届くので、その間だけ印を付ける。
 */
export function focusQuietly(el: HTMLElement): void {
  quietFocus.add(el);
  try {
    el.focus();
  } finally {
    quietFocus.delete(el);
  }
}

/** `focusQuietly` でフォーカスを受けているところか (Tooltip がフォーカスで出るのを止める)。 */
export function isQuietlyFocused(el: EventTarget | null): boolean {
  return el instanceof Element && quietFocus.has(el);
}
