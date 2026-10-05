// @vitest-environment happy-dom
// モデルを描く領域 (Viewport.svelte) の GPU を失ったときの表示: 戻るのを待つ間は「GPU の復元を待っています」を出し、
// 続けて失って諦めたら「作り直す」ボタンを出して Player.restart を呼ぶこと (T0938)。
import { setLocale } from "@ugoe/i18n";
import type { Player } from "@ugoe/player";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Viewport from "./Viewport.svelte";

const cleanups: (() => void)[] = [];

beforeEach(() => {
  setLocale("ja");
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

/** 領域が読む Player の状態と、呼ぶメソッドだけを持つ偽物。 */
function setup() {
  const calls: string[] = [];
  const player = $state({
    status: "ready" as Player["status"],
    error: null as string | null,
    waitingForGpu: false,
    canRestart: false,
    transparent: false,
    modelName: "a.ugoe",
    usesLighting: false,
    resize() {},
    attach: async () => {},
    detach() {},
    restart: async () => {
      calls.push("restart");
    },
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(Viewport, { target, props: { player: player as unknown as Player } });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  return { target, player, calls };
}

describe("Viewport", () => {
  it("says it is waiting for the GPU while the GPU is lost", () => {
    const { target, player } = setup();
    expect(target.textContent).not.toContain("GPU の復元を待っています");
    player.waitingForGpu = true;
    flushSync();
    expect(target.querySelector('[role="status"]')?.textContent).toBe("GPU の復元を待っています…");
    player.waitingForGpu = false;
    flushSync();
    expect(target.querySelector('[role="status"]')).toBeNull();
  });

  it("offers to restart after giving up and calls restart", () => {
    const { target, player, calls } = setup();
    player.status = "unavailable";
    player.error = "GPU を続けて失ったので、描画を止めました";
    flushSync();
    // 描画を始められなかっただけ (canRestart が偽) なら、ボタンは出さない。
    expect(target.querySelector("button")).toBeNull();
    expect(target.textContent).toContain("GPU を続けて失ったので");

    player.canRestart = true;
    flushSync();
    const button = target.querySelector("button");
    expect(button?.textContent?.trim()).toBe("作り直す");
    expect(target.textContent).toContain("GPU を続けて失ったので");
    button?.click();
    expect(calls).toEqual(["restart"]);
  });
});
