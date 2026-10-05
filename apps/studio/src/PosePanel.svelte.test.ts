// @vitest-environment happy-dom
// ポーズのパネル (PosePanel.svelte): グループごとに要素を選ぶ欄を出し、選んだ要素を送ること。送っている間は選んだ要素を
// 先に出し、送り終えたら届いた通知の値に戻すこと (T0842)。
import { setLocale } from "@ugoe/i18n";
import type { PoseGroupInfo } from "@ugoe/player";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import PosePanel from "./PosePanel.svelte";

const cleanups: (() => void)[] = [];

beforeEach(() => setLocale("ja"));

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

/** パネルを描く。送った要素は、`notify` を呼ぶまで props に戻らない (バックエンドの通知を待つ)。 */
function setup(groups: PoseGroupInfo[]) {
  const sent: string[] = [];
  const finish: (() => void)[] = [];
  const props = $state({
    groups,
    shown: groups.map((g) => g.parts[0] ?? ""),
    onchange: (part: string) => {
      sent.push(part);
      return new Promise<void>((resolve) => finish.push(resolve));
    },
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(PosePanel, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  /** バックエンドが表示させた要素を通知で返してから、送り終える。 */
  const notify = async (shown: string[]) => {
    props.shown = shown;
    for (let resolve = finish.shift(); resolve !== undefined; resolve = finish.shift()) resolve();
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
  };
  return { target, props, sent, notify };
}

/** グループの欄の見出しと、選んでいる要素の表示名。 */
function rows(target: HTMLElement): [string, string][] {
  return [...target.querySelectorAll(".poses__group")].map((g) => [
    g.querySelector(".ugoe-segmented__label")?.textContent?.trim() ?? "",
    g.querySelector('[data-state="checked"]')?.textContent?.trim() ?? "",
  ]);
}

function item(target: HTMLElement, text: string): HTMLButtonElement {
  const found = [...target.querySelectorAll<HTMLButtonElement>(".ugoe-segmented__item")].find(
    (b) => b.textContent?.trim() === text,
  );
  if (found === undefined) throw new Error(`no item ${text}`);
  return found;
}

describe("PosePanel", () => {
  it("shows a choice for each group of the model and of attached modules", () => {
    const { target } = setup([
      { parts: ["ArmA", "ArmB"], module: null },
      { parts: ["Hat/On", "Hat/Off"], module: "Hat" },
    ]);
    expect(rows(target)).toEqual([
      ["グループ 1", "ArmA"],
      ["Hat のグループ 1", "On"],
    ]);
  });

  it("sends the chosen part and shows it until the notice arrives", async () => {
    const { target, sent, notify } = setup([
      { parts: ["ArmA", "ArmB"], module: null },
      { parts: ["Hat/On", "Hat/Off"], module: "Hat" },
    ]);
    item(target, "ArmB").click();
    flushSync();
    // 通知の前に別のグループを選んでも、先に選んだものを消さない。
    item(target, "Off").click();
    flushSync();
    expect(sent).toEqual(["ArmB", "Hat/Off"]);
    expect(rows(target)).toEqual([
      ["グループ 1", "ArmB"],
      ["Hat のグループ 1", "Off"],
    ]);
    await notify(["ArmB", "Hat/Off"]);
    expect(rows(target)).toEqual([
      ["グループ 1", "ArmB"],
      ["Hat のグループ 1", "Off"],
    ]);
  });

  it("uses a select for groups with five or more entries", () => {
    const { target } = setup([{ parts: ["A", "B", "C", "D", "E"], module: null }]);
    expect(target.querySelector(".ugoe-segmented")).toBeNull();
    expect(target.querySelector(".ugoe-select")?.textContent).toContain("A");
  });
});
