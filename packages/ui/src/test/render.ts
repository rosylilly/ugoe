// テストで部品を happy-dom に描く小さな道具 (@testing-library は使わない)。

import { type Component, createRawSnippet, flushSync, mount, unmount } from "svelte";
import { afterEach } from "vitest";

const mounted: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of mounted.splice(0)) cleanup();
});

/** 部品を body に描く。テストの後で片付ける。 */
// biome-ignore lint/suspicious/noExplicitAny: 部品ごとに props の型が違う
export function render<P extends Record<string, any>>(component: Component<P>, props: P) {
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(component, { target, props });
  flushSync();
  mounted.push(() => {
    unmount(instance);
    target.remove();
  });
  return target;
}

/** 固定の HTML を描く snippet。 */
export function html(markup: string) {
  return createRawSnippet(() => ({ render: () => markup }));
}

/** キーを押す (keydown を送り、Svelte の更新を流す)。 */
export function press(el: Element, key: string, init: KeyboardEventInit = {}) {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init });
  el.dispatchEvent(event);
  flushSync();
  return event;
}

/** クリックする。 */
export function click(el: Element) {
  (el as HTMLElement).click();
  flushSync();
}

/** `selector` に合う要素 (無ければ例外)。 */
export function get<E extends Element = HTMLElement>(root: ParentNode, selector: string): E {
  const el = root.querySelector<E>(selector);
  if (!el) throw new Error(`not found: ${selector}`);
  return el;
}

/** aria-labelledby が指す要素の文字をつないだもの (読み上げの名前)。 */
export function labelledByText(el: Element): string {
  const ids = el.getAttribute("aria-labelledby")?.split(/\s+/) ?? [];
  return ids.map((id) => document.getElementById(id)?.textContent?.trim() ?? "").join(" ");
}
