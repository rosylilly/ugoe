// @vitest-environment happy-dom
// シーンの一覧 (ScenesPanel.svelte): 読み込んでいるシーンに印を付け、読み込み中を知らせる (T0837)。
// 切り替えのホットキーを並べて示し、削除は確かめてから行う (T0817)。
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { t } from "./lib/i18n";
import type { SceneSettings } from "./lib/profile";
import ScenesPanel from "./ScenesPanel.svelte";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

/** モデルを選んだシーン。 */
function scene(id: string, name: string, model: string): SceneSettings {
  return {
    id,
    name,
    model,
    modules: [],
    lighting: null,
    transform: { x: 0, y: 0, scale: 1, rotation: 0 },
    background: { kind: "transparent" },
    pose: [],
  };
}

function setup(
  loading: { scene: string | null } | null,
  hotkeys: ReadonlyMap<string, string> = new Map(),
) {
  const scenes: SceneSettings[] = [
    scene("a", "Chat", "/m/avatar.ugoe"),
    scene("b", "Game", "/m/other.ugoe"),
  ];
  const props = $state({
    scenes,
    live: "a" as string | null,
    selected: "b" as string | null,
    loading,
    hotkeys,
    onactivate: (_id: string) => {},
    onadd: () => {},
    onduplicate: (_id: string) => {},
    onmove: (_id: string, _index: number) => {},
    onremove: (_id: string) => {},
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(ScenesPanel, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  const item = (name: string) =>
    [...target.querySelectorAll<HTMLButtonElement>(".scenes__item")].find((b) =>
      b.textContent?.includes(name),
    );
  const status = () => target.querySelector('[role="status"]')?.textContent?.trim();
  const button = (label: string) =>
    target.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);
  return { props, item, status, button };
}

describe("ScenesPanel", () => {
  it("marks the scene being loaded and announces it", () => {
    const { props, item, status } = setup(null);
    expect(status()).toBe(t("scenes.switchHint"));
    expect(item("Game")?.getAttribute("aria-busy")).toBeNull();

    props.loading = { scene: "b" };
    flushSync();
    expect(status()).toBe(t("scenes.loadingStatus"));
    expect(item("Game")?.getAttribute("aria-busy")).toBe("true");
    expect(item("Game")?.textContent).toContain(t("scenes.loading"));
    // 配信中のシーンは、読み込み終えるまで配信中のまま。
    expect(item("Chat")?.textContent).toContain(t("scenes.live"));

    props.loading = null;
    flushSync();
    expect(status()).toBe(t("scenes.switchHint"));
    expect(item("Game")?.textContent).not.toContain(t("scenes.loading"));
  });

  it("announces loading when the scene is not known (opening a profile)", () => {
    const { item, status } = setup({ scene: null });
    expect(status()).toBe(t("scenes.loadingStatus"));
    expect(item("Chat")?.getAttribute("aria-busy")).toBeNull();
    expect(item("Game")?.getAttribute("aria-busy")).toBeNull();
  });

  it("shows the hotkey that switches to each scene", () => {
    const { item } = setup(null, new Map([["b", "Ctrl+Shift+Digit2"]]));
    expect(item("Game")?.querySelector(".ugoe-kbd")?.textContent).toContain("2");
    expect(item("Chat")?.querySelector(".ugoe-kbd")).toBeNull();
  });

  it("asks before deleting the selected scene", async () => {
    const { props, button } = setup(null);
    const onremove = vi.fn();
    props.onremove = onremove;
    flushSync();
    button(t("scenes.removeLabel"))?.click();
    const dialog = await vi.waitFor(() => {
      const found = document.querySelector<HTMLElement>('[role="alertdialog"]');
      if (found === null) throw new Error("no dialog");
      return found;
    });
    expect(onremove).not.toHaveBeenCalled();
    const confirm = [...dialog.querySelectorAll("button")].find(
      (b) => b.textContent?.trim() === t("scenes.remove"),
    );
    confirm?.click();
    await vi.waitFor(() => expect(onremove).toHaveBeenCalledWith("b"));
  });
});
