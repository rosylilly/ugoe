// @vitest-environment happy-dom
// モデルを描く領域 (Viewport.svelte) の GPU を失ったときの表示: 戻るのを待つ間は「GPU の復元を待っています」を出し、
// 続けて失って諦めたら「作り直す」ボタンを出して Player.restart を呼ぶこと (T0938)。
// 空の状態に呼び手の中身 (開く・サンプル) を出し、拡大・縮小の道具を重ね、操作できる部品を role="img" の中に入れないこと (T0725)。
import { setLocale } from "@ugoe/i18n";
import type { Player } from "@ugoe/player";
import { createRawSnippet, flushSync, mount, unmount } from "svelte";
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
    zoom: 1,
    resize() {},
    zoomBy(factor: number) {
      calls.push(`zoomBy ${factor}`);
    },
    fit() {
      calls.push("fit");
    },
    attach: async () => {},
    detach() {},
    restart: async () => {
      calls.push("restart");
    },
  });
  const target = document.createElement("div");
  document.body.append(target);
  const empty = createRawSnippet(() => ({
    render: () => '<div class="empty-content"><button type="button">開く</button></div>',
  }));
  const instance = mount(Viewport, {
    target,
    props: { player: player as unknown as Player, empty },
  });
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

  it("shows the empty state from the caller and no view tools without a model", () => {
    const { target, player } = setup();
    player.status = "empty";
    flushSync();
    expect(target.querySelector(".empty-content button")?.textContent).toBe("開く");
    expect(target.querySelector('[role="toolbar"]')).toBeNull();
  });

  it("overlays the view tools and keeps controls out of the picture", () => {
    const { target, calls } = setup();
    const toolbar = target.querySelector('[role="toolbar"]');
    expect(toolbar).not.toBeNull();
    const fit = toolbar?.querySelector<HTMLButtonElement>('button[aria-label="全体を表示"]');
    fit?.click();
    expect(calls).toContain("fit");
    const picture = target.querySelector('[role="img"]');
    expect(picture?.getAttribute("aria-label")).toBe("a.ugoe");
    // 絵の名前を持つ要素は、操作できる部品を中に持たない (axe の nested-interactive)
    expect(picture?.querySelector("button, [tabindex], input")).toBeNull();
    expect(picture?.contains(toolbar ?? null)).toBe(false);
  });
});
