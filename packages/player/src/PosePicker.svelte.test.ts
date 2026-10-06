// @vitest-environment happy-dom
// ポーズの要素を選ぶ欄 (PosePicker.svelte。T0948): グループごとに要素を選ぶ欄を出し、選んだ要素を `onchange` に渡すこと。
// 欄は `shown` に追従すること (ビューアのモーションによる切り替え。T0736)。`onchange` が Promise を返す間は選んだ要素を先に出し、
// 終わったら届いた `shown` に戻すこと (スタジオ。T0842)。
import { setLocale } from "@ugoe/i18n";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import PosePicker from "./PosePicker.svelte";
import type { PoseGroupInfo } from "./pose";

const cleanups: (() => void)[] = [];

beforeEach(() => setLocale("ja"));

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

/**
 * 欄を描く。`sync` なら選んだ要素をその場で `shown` に書く (ビューアの Player)。そうでなければ Promise を返し、`notify` を呼ぶまで
 * `shown` に戻らない (スタジオのバックエンドの通知を待つ)。
 */
function setup(groups: PoseGroupInfo[], { sync = false } = {}) {
  const sent: string[] = [];
  const finish: (() => void)[] = [];
  const props = $state({
    groups,
    shown: groups.map((g) => g.parts[0] ?? ""),
    disabled: false,
    onchange: (part: string): unknown => {
      sent.push(part);
      if (sync) {
        const i = props.groups.findIndex((g) => g.parts.includes(part));
        props.shown = props.shown.map((p, j) => (j === i ? part : p));
        return true;
      }
      return new Promise<void>((resolve) => finish.push(resolve));
    },
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(PosePicker, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  /** 表示させた要素を通知で返してから、送り終える。 */
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
  return [...target.querySelectorAll(".ugoe-pose-picker__group")].map((g) => [
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

describe("PosePicker", () => {
  it("shows a choice for each group of the model and of attached modules", () => {
    const { target, props } = setup([
      { parts: ["earwear", "no_earwear"], module: null },
      { parts: ["Hat/On", "Hat/Off"], module: "Hat" },
      { parts: ["Hat/Brim", "Hat/NoBrim"], module: "Hat" },
    ]);
    expect(rows(target)).toEqual([
      ["グループ 1", "earwear"],
      ["Hat のグループ 1", "On"],
      ["Hat のグループ 2", "Brim"],
    ]);

    // 着せ替えでモジュールを外すと、呼び手が渡し直したグループに合わせる。
    props.groups = [{ parts: ["earwear", "no_earwear"], module: null }];
    props.shown = ["no_earwear"];
    flushSync();
    expect(rows(target)).toEqual([["グループ 1", "no_earwear"]]);
  });

  it("passes the chosen part to onchange", () => {
    const { target, sent } = setup([{ parts: ["HandsDown", "Wave"], module: null }], {
      sync: true,
    });
    item(target, "Wave").click();
    flushSync();
    expect(sent).toEqual(["Wave"]);
    expect(rows(target)).toEqual([["グループ 1", "Wave"]]);
  });

  it("follows the shown entries the caller passes", () => {
    const { target, props } = setup(
      [
        { parts: ["HandsDown", "Wave"], module: null },
        { parts: ["A", "B", "C", "D", "E"], module: null },
      ],
      { sync: true },
    );
    // モーションが切り替えた (ビューアの Player が表示中の要素に合わせて poseShown を書き換えた)。
    props.shown = ["Wave", "C"];
    flushSync();
    expect(rows(target)[0]).toEqual(["グループ 1", "Wave"]);
    expect(target.querySelector(".ugoe-select__value")?.textContent?.trim()).toBe("C");

    // モーションが終わった後に利用者が選び直せる。
    item(target, "HandsDown").click();
    flushSync();
    expect(rows(target)[0]).toEqual(["グループ 1", "HandsDown"]);
  });

  it("shows the chosen part while onchange is pending", async () => {
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
    // 送り終えたら届いた値を出す (バックエンドが別の要素にしたなら、それに戻る)。
    await notify(["ArmA", "Hat/Off"]);
    expect(rows(target)).toEqual([
      ["グループ 1", "ArmA"],
      ["Hat のグループ 1", "Off"],
    ]);
  });

  it("uses a select for groups with five or more entries", () => {
    const { target } = setup([{ parts: ["A", "B", "C", "D", "E"], module: null }]);
    expect(target.querySelector(".ugoe-segmented")).toBeNull();
    expect(target.querySelector(".ugoe-select")?.textContent).toContain("A");
  });

  it("disables the choices", () => {
    const { target, props, sent } = setup([{ parts: ["HandsDown", "Wave"], module: null }]);
    props.disabled = true;
    flushSync();
    expect(item(target, "Wave").disabled).toBe(true);
    item(target, "Wave").click();
    flushSync();
    expect(sent).toEqual([]);
  });
});
