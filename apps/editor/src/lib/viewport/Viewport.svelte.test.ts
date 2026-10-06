// @vitest-environment happy-dom
// ビューポート (Viewport.svelte) の GPU を失ったときの表示: 戻るのを待つ間は controller の文言を出し、続けて失って
// 諦めたら「作り直す」ボタンを出して ViewportController.restart を呼ぶこと (T0938)。描くものが無い間はスクリーンショットを
// 押せないこと (T1655)。
import { createRawSnippet, flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { t } from "../i18n";
import type { ViewportController, ViewportStatus } from "./controller.svelte";
import Viewport from "./Viewport.svelte";

const cleanups: (() => void)[] = [];

beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
});

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
  vi.unstubAllGlobals();
});

/** ビューポートが読む controller の状態と、呼ぶメソッドだけを持つ偽物。 */
function setup(options: { empty?: boolean } = {}) {
  const calls: string[] = [];
  const controller = $state({
    status: "ready" as ViewportStatus,
    message: null as string | null,
    hasModel: true,
    backend: "webgl2",
    canRestart: false,
    attach: async () => {},
    detach() {},
    resize() {},
    setModel() {},
    setBackground() {},
    fit() {},
    covered: { top: 0, bottom: 0 },
    setCovered(edge: "top" | "bottom", height: number) {
      controller.covered[edge] = height;
    },
    restart: async () => {
      calls.push("restart");
    },
    panBy(dx: number, dy: number) {
      calls.push(`pan ${dx},${dy}`);
    },
  });
  const target = document.createElement("div");
  document.body.append(target);
  const props = $state({
    controller: controller as unknown as ViewportController,
    model: null,
    empty: options.empty ? createRawSnippet(() => ({ render: () => "<p>empty</p>" })) : undefined,
  });
  const instance = mount(Viewport, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  return { target, controller, props, calls };
}

/** HUD の「Screenshot」のボタン。 */
function screenshotButton(target: HTMLElement): HTMLButtonElement {
  const button = [...target.querySelectorAll<HTMLButtonElement>(".hud button")].find(
    (b) => b.textContent?.trim() === t("viewport.screenshot.label"),
  );
  if (!button) throw new Error("no screenshot button");
  return button;
}

/** ボタンの aria-describedby が指す文。 */
function description(button: HTMLElement): string | null {
  const id = button.getAttribute("aria-describedby");
  return id ? (document.getElementById(id)?.textContent ?? null) : null;
}

/** HUD のボタンの文言。 */
function buttons(target: HTMLElement): string[] {
  return [...target.querySelectorAll(".hud button")].map((b) => b.textContent?.trim() ?? "");
}

describe("Viewport", () => {
  it("shows the message while waiting for the GPU", () => {
    const { target, controller } = setup();
    controller.status = "loading";
    controller.message = t("viewport.gpuWaiting");
    flushSync();
    expect(target.querySelector(".hud")?.textContent).toContain(t("viewport.gpuWaiting"));
    expect(buttons(target)).not.toContain(t("viewport.restart"));
  });

  it("offers to restart after giving up and calls restart", () => {
    const { target, controller, calls } = setup();
    controller.status = "error";
    controller.message = t("viewport.gpuLostRepeatedly");
    flushSync();
    // ほかの失敗 (読み込みなど) では出さない。
    expect(buttons(target)).not.toContain(t("viewport.restart"));
    controller.canRestart = true;
    flushSync();
    const restart = [...target.querySelectorAll<HTMLButtonElement>(".hud button")].find(
      (b) => b.textContent?.trim() === t("viewport.restart"),
    );
    expect(restart).toBeDefined();
    restart?.click();
    expect(calls).toEqual(["restart"]);
  });

  it("pans with Alt + arrows while the focus is on the page or in the viewport", () => {
    const { target, calls } = setup();
    const press = (el: Element, key: string, init: KeyboardEventInit = {}) => {
      const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init });
      el.dispatchEvent(event);
      return event;
    };
    expect(press(document.body, "ArrowRight", { altKey: true }).defaultPrevented).toBe(true);
    // ビューポートの中 (表示の道具のボタンなど)
    const zoomIn = target.querySelector(`[aria-label="${t("viewport.zoomIn.label")}"]`);
    if (!zoomIn) throw new Error("no zoom button");
    press(zoomIn, "ArrowUp", { altKey: true, shiftKey: true });
    // 矢印だけ (点を動かす) とビューポートの外 (ツリー・スライダー) では動かさない
    press(document.body, "ArrowLeft");
    const outside = document.createElement("button");
    document.body.append(outside);
    cleanups.push(() => outside.remove());
    expect(press(outside, "ArrowLeft", { altKey: true }).defaultPrevented).toBe(false);
    expect(calls).toEqual(["pan 40,0", "pan 0,-200"]);
  });

  it("tells the controller how much of the bottom edge the view tools cover (T1653)", () => {
    // 右下の表示の道具の高さを 40px にする (happy-dom は大きさを測らない)
    const height = vi
      .spyOn(HTMLElement.prototype, "offsetHeight", "get")
      .mockImplementation(function (this: HTMLElement) {
        return this.classList.contains("view-tools") ? 40 : 0;
      });
    try {
      const { controller } = setup();
      // 道具の高さと、道具とビューの下の端の間 (8px)
      expect(controller.covered.bottom).toBe(48);
      // 道具を出さない間 (描画できていない) は隠さない
      controller.status = "loading";
      flushSync();
      expect(controller.covered.bottom).toBe(0);
    } finally {
      height.mockRestore();
    }
  });

  it("disables the screenshot until a model is drawn and says why (T1655)", () => {
    const { target, controller } = setup();
    controller.hasModel = false;
    flushSync();
    expect(screenshotButton(target).disabled).toBe(true);
    expect(description(screenshotButton(target))).toBe(t("viewport.screenshot.unavailable"));
    // モデルを読み込んだら押せる
    controller.hasModel = true;
    flushSync();
    expect(screenshotButton(target).disabled).toBe(false);
    expect(description(screenshotButton(target))).toBeNull();
    // 読み込みの失敗 (と描画できない間) は押せない
    controller.status = "error";
    flushSync();
    expect(screenshotButton(target).disabled).toBe(true);
    expect(description(screenshotButton(target))).toBe(t("viewport.screenshot.unavailable"));
  });

  it("disables the screenshot in an empty project (T1655)", () => {
    const { target, props } = setup({ empty: true });
    expect(screenshotButton(target).disabled).toBe(true);
    expect(description(screenshotButton(target))).toBe(t("viewport.screenshot.unavailable"));
    // 中身を足して空でなくなったら押せる
    props.empty = undefined;
    flushSync();
    expect(screenshotButton(target).disabled).toBe(false);
  });
});
