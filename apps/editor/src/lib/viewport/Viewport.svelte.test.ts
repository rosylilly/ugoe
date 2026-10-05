// @vitest-environment happy-dom
// ビューポート (Viewport.svelte) の GPU を失ったときの表示: 戻るのを待つ間は controller の文言を出し、続けて失って
// 諦めたら「作り直す」ボタンを出して ViewportController.restart を呼ぶこと (T0938)。
import { flushSync, mount, unmount } from "svelte";
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
function setup() {
  const calls: string[] = [];
  const controller = $state({
    status: "ready" as ViewportStatus,
    message: null as string | null,
    backend: "webgl2",
    canRestart: false,
    attach: async () => {},
    detach() {},
    resize() {},
    setModel() {},
    setBackground() {},
    fit() {},
    restart: async () => {
      calls.push("restart");
    },
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(Viewport, {
    target,
    props: { controller: controller as unknown as ViewportController, model: null },
  });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  return { target, controller, calls };
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
});
