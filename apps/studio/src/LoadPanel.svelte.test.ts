// @vitest-environment happy-dom
// 負荷のパネル (LoadPanel.svelte) の GPU の優先度 (T0861): Windows でだけ選べ、当てられなかった理由をコードから訳して出すこと。
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import LoadPanel from "./LoadPanel.svelte";
import type { GpuPriority, GpuView } from "./lib/host";
import { t } from "./lib/i18n";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

const WINDOWS: GpuView = {
  preference: "auto",
  launched: "auto",
  graphicsSettings: true,
  priority: "normal",
  prioritySupported: true,
  priorityErrors: [],
};

function setup(gpu: GpuView) {
  const chosen: GpuPriority[] = [];
  const props = $state({
    tickRate: 60 as const,
    renderScale: 1 as const,
    status: null,
    gpu,
    onTickRate: () => {},
    onRenderScale: () => {},
    onGpuPreference: () => {},
    onGpuPriority: (priority: GpuPriority) => chosen.push(priority),
    onGraphicsSettings: () => {},
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(LoadPanel, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  return { target, props, chosen };
}

/** 文字が `text` の選択肢。 */
function option(target: HTMLElement, text: string): HTMLElement | undefined {
  return [...target.querySelectorAll<HTMLElement>("[role=radio]")].find(
    (el) => el.textContent?.trim() === text,
  );
}

describe("GPU の優先度", () => {
  it("Windows の外では出さない", () => {
    const { target } = setup({ ...WINDOWS, graphicsSettings: false, prioritySupported: false });
    expect(target.textContent).not.toContain(t("load.priority"));
    expect(option(target, t("load.priorityLow"))).toBeUndefined();
  });

  it("Windows では選べ、選んだ優先度を渡す", () => {
    const { target, chosen } = setup(WINDOWS);
    expect(target.textContent).toContain(t("load.priority"));
    const low = option(target, t("load.priorityLow"));
    expect(low).toBeDefined();
    low?.click();
    flushSync();
    expect(chosen).toEqual(["low"]);
  });

  it("当てられなかった理由をコードから訳して出す", () => {
    const { target, props } = setup({
      ...WINDOWS,
      priority: "low",
      priorityErrors: [
        {
          code: "gpu_priority.access_denied",
          args: { process: "msedgewebview2 (GPU, 42)" },
          message: "access to msedgewebview2 (GPU, 42) was denied when changing its GPU priority",
        },
      ],
    });
    expect(target.textContent).toContain(
      t("errors.gpu_priority.access_denied", { process: "msedgewebview2 (GPU, 42)" }),
    );
    // 当て直せたら (通知で様子が変わったら) 消える
    props.gpu = { ...props.gpu, priorityErrors: [] };
    flushSync();
    expect(target.textContent).not.toContain("msedgewebview2");
  });
});
