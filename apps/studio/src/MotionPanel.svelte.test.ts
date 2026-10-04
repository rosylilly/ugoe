// @vitest-environment happy-dom
// モーションの設定 (MotionPanel.svelte): プロファイルの通知を待たずに続けて変えても、変更が消えないこと。
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import { DEFAULT_MOTION, EMPTY_ANIMATION, type MotionSettings } from "./lib/inputs";
import MotionPanel from "./MotionPanel.svelte";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

function setup(initial: MotionSettings, trackingParameters: string[]) {
  const sent: MotionSettings[] = [];
  const finish: (() => void)[] = [];
  const props = $state({
    animation: { ...EMPTY_ANIMATION, trackingParameters },
    settings: initial,
    onplay: () => {},
    onstop: () => {},
    onexpression: () => {},
    onsettings: (settings: MotionSettings) => {
      sent.push(settings);
      return new Promise<void>((resolve) => finish.push(resolve));
    },
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(MotionPanel, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  /** バックエンドが送った設定を受け取り、通知で返してから送り終える (送っていないものが無くなるまで)。 */
  const notify = async () => {
    for (let resolve = finish.shift(); resolve !== undefined; resolve = finish.shift()) {
      const last = sent.at(-1);
      if (last !== undefined) props.settings = last;
      resolve();
      await new Promise((r) => setTimeout(r, 0));
    }
    flushSync();
  };
  return { target, props, sent, notify };
}

/** まとまりのチェックボックス (ラベルの文言で探す)。 */
function checkbox(root: Element, label: string): HTMLElement {
  const box = [...root.querySelectorAll(".ugoe-checkbox")]
    .find((c) => c.textContent?.trim() === label)
    ?.querySelector<HTMLElement>('[role="checkbox"]');
  if (box == null) throw new Error(`no checkbox ${label}`);
  return box;
}

describe("MotionPanel", () => {
  it("keeps every group toggled before the profile notification arrives", async () => {
    const { target, props, sent, notify } = setup(
      { ...DEFAULT_MOTION, overTracking: true, trackingKept: [] },
      ["AngleX", "EyeLOpen"],
    );
    const labels = [...target.querySelectorAll(".ugoe-checkbox")].map(
      (c) => c.textContent?.trim() ?? "",
    );
    expect(labels).toHaveLength(2);
    checkbox(target, labels[0] ?? "").click();
    flushSync();
    checkbox(target, labels[1] ?? "").click();
    flushSync();
    // 通知の前でも、両方が入って見える。
    expect(checkbox(target, labels[0] ?? "").getAttribute("aria-checked")).toBe("true");
    expect(checkbox(target, labels[1] ?? "").getAttribute("aria-checked")).toBe("true");
    await notify();
    expect(sent.at(-1)?.trackingKept).toEqual(["AngleX", "EyeLOpen"]);
    expect(props.settings.trackingKept).toEqual(["AngleX", "EyeLOpen"]);
    expect(checkbox(target, labels[1] ?? "").getAttribute("aria-checked")).toBe("true");
  });

  it("shows the notified settings once the send has finished", async () => {
    const { target, props, notify } = setup(
      { ...DEFAULT_MOTION, overTracking: true, trackingKept: [] },
      ["AngleX"],
    );
    const label = target.querySelector(".ugoe-checkbox")?.textContent?.trim() ?? "";
    checkbox(target, label).click();
    flushSync();
    await notify();
    // ほかの操作 (別のプロファイルを開くなど) で届いた設定に従う。
    props.settings = { ...props.settings, trackingKept: [] };
    flushSync();
    expect(checkbox(target, label).getAttribute("aria-checked")).toBe("false");
  });
});
