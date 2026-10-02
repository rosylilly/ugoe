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
