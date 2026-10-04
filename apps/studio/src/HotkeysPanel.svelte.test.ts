// @vitest-environment happy-dom
// ホットキーの割り当て (HotkeysPanel.svelte): プロファイルの通知を待たずに続けて変えても、変更が消えないこと。
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import HotkeysPanel from "./HotkeysPanel.svelte";
import { t } from "./lib/i18n";
import type { Hotkey } from "./lib/inputs";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

/** 割り当ての画面を描く。送った割り当ては、`notify` を呼ぶまで props に戻らない (バックエンドの通知を待つ)。 */
function setup(initial: Hotkey[]) {
  const sent: Hotkey[][] = [];
  const finish: (() => void)[] = [];
  const props = $state({
    hotkeys: initial,
    problems: [],
    wayland: false,
    scenes: [{ id: "a", label: "A" }],
    expressions: [],
    motions: [],
    slots: [],
    onchange: (list: Hotkey[]) => {
      sent.push(list);
      return new Promise<void>((resolve) => finish.push(resolve));
    },
    onsuspend: (_suspended: boolean) => {},
    onpickmodule: async () => null,
    moduleSlot: async () => "",
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(HotkeysPanel, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  /** バックエンドが送った割り当てを受け取り、通知で返してから送り終える (送っていないものが無くなるまで)。 */
  const notify = async () => {
    for (let resolve = finish.shift(); resolve !== undefined; resolve = finish.shift()) {
      const last = sent.at(-1);
      if (last !== undefined) props.hotkeys = last;
      resolve();
      await new Promise((r) => setTimeout(r, 0));
    }
    flushSync();
  };
  return { target, sent, notify };
}

function button(root: Element, text: string): HTMLElement {
  const found = [...root.querySelectorAll<HTMLElement>("button")].find(
    (b) => b.textContent?.trim() === text,
  );
  if (found === undefined) throw new Error(`no button ${text}`);
  return found;
}

function press(code: string, init: KeyboardEventInit = {}) {
  // ホットキーの画面は window で捕まえる (押したときの焦点はどこでもよい)。
  document.body.dispatchEvent(new KeyboardEvent("keydown", { code, bubbles: true, ...init }));
  flushSync();
}

describe("HotkeysPanel", () => {
  it("puts the keys pressed right after adding into the new hotkey", async () => {
    const { target, sent, notify } = setup([]);
    button(target, t("hotkeys.add")).click();
    flushSync();
    // 足した行は、通知を待たずに出る。
    expect(target.querySelectorAll(".hotkeys__item")).toHaveLength(1);
    press("KeyA", { ctrlKey: true });
    await notify();
    expect(sent.at(-1)).toEqual([{ keys: "Ctrl+KeyA", action: { kind: "scene", scene: "a" } }]);
    expect(target.querySelectorAll(".hotkeys__item")).toHaveLength(1);
  });

  it("stacks edits made before the profile notification arrives", async () => {
    const { target, sent, notify } = setup([
      { keys: "Ctrl+KeyA", action: { kind: "stopMotions" } },
    ]);
    button(target, t("hotkeys.add")).click();
    flushSync();
    press("KeyB", { ctrlKey: true });
    button(target, t("hotkeys.add")).click();
    flushSync();
    press("KeyC", { ctrlKey: true });
    // 1 つ目を消しても、足した 2 つは残る。
    const first = target.querySelector(".hotkeys__item");
    if (first === null) throw new Error("no item");
    button(first, t("hotkeys.remove")).click();
    flushSync();
    expect(target.querySelectorAll(".hotkeys__item")).toHaveLength(2);
    // 送っている間の変更はまとめて、最後のものを送る。
    await notify();
    expect(sent).toHaveLength(2);
    expect(sent.at(-1)?.map((h) => h.keys)).toEqual(["Ctrl+KeyB", "Ctrl+KeyC"]);
    expect(target.querySelectorAll(".hotkeys__item")).toHaveLength(2);
  });
});
