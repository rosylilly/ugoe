// ViewportController の WebPuppet の作成と破棄 (attach・detach・作り直し)。wasm と canvas は偽物。

import { MAX_RESTARTS } from "@ugoe/player";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { t } from "../i18n";
import type { ViewportController as ViewportControllerType } from "./controller.svelte";
import type { WebPuppet } from "./wasm";

class FakeCanvas extends EventTarget {
  width = 300;
  height = 150;
  className = "";
  removed = false;
  remove(): void {
    this.removed = true;
  }
}

/** WebPuppet.create が返す約束 (テストが好きな時に解く)。 */
let pending: {
  canvas: FakeCanvas;
  resolve: (p: WebPuppet) => void;
  reject: (e: unknown) => void;
}[] = [];

// @ugoe/player の Player が読むトラッキング (と画面の部品・MediaPipe) は使わないので空にする (読むと遅く、
// `document` を置いた後に読むと bits-ui が `window` を引く)。
vi.mock("@ugoe/face-tracking", () => ({ setPuppetTracking: () => {} }));

vi.mock("./wasm", () => ({
  loadUgoeWasm: async () => ({
    WebPuppet: {
      create: (canvas: FakeCanvas) =>
        new Promise<WebPuppet>((resolve, reject) => pending.push({ canvas, resolve, reject })),
    },
  }),
}));

let ViewportController: typeof ViewportControllerType;

beforeAll(async () => {
  globalThis.requestAnimationFrame ??= () => 1;
  globalThis.cancelAnimationFrame ??= () => {};
  globalThis.document ??= { createElement: () => new FakeCanvas() } as unknown as Document;
  ({ ViewportController } = await import("./controller.svelte"));
});

beforeEach(() => {
  pending = [];
});

function fakePuppet() {
  const calls: string[] = [];
  const puppet = {
    lost: false,
    is_lost: () => puppet.lost,
    render() {},
    free: () => calls.push("free"),
    backend: () => "webgl2",
    set_background() {},
    set_lighting_json() {},
    resize() {},
    has_model: () => false,
  };
  return { puppet: puppet as unknown as WebPuppet & { lost: boolean }, calls };
}

/** canvas を置く要素 (canvas は ViewportController が作る)。 */
function fakeContainer(): HTMLElement {
  return { prepend() {} } as unknown as HTMLElement;
}

/** 待っているマイクロタスクを流す (loadUgoeWasm と create の await の後まで)。 */
async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) await Promise.resolve();
}

describe("ViewportController", () => {
  it("makes a WebPuppet for the canvas", async () => {
    const controller = new ViewportController();
    const attached = controller.attach(fakeContainer());
    await settle();
    const { puppet } = fakePuppet();
    pending[0]?.resolve(puppet);
    await attached;
    expect(controller.puppet).toBe(puppet);
    expect(controller.status).toBe("ready");
  });

  it("frees a WebPuppet that finishes after detach", async () => {
    const controller = new ViewportController();
    const attached = controller.attach(fakeContainer());
    await settle();
    controller.detach();
    const { puppet, calls } = fakePuppet();
    pending[0]?.resolve(puppet);
    await attached;
    expect(controller.puppet).toBeNull();
    expect(calls).toEqual(["free"]);
  });

  it("keeps only the WebPuppet of the last attach", async () => {
    const controller = new ViewportController();
    const first = controller.attach(fakeContainer());
    await settle();
    const second = controller.attach(fakeContainer());
    await settle();
    const old = fakePuppet();
    const current = fakePuppet();
    pending[1]?.resolve(current.puppet);
    pending[0]?.resolve(old.puppet);
    await Promise.all([first, second]);
    expect(controller.puppet).toBe(current.puppet);
    expect(old.calls).toEqual(["free"]);
    expect(current.calls).toEqual([]);
  });

  it("recreates the canvas when the WebGL context is lost", async () => {
    const controller = new ViewportController();
    const attached = controller.attach(fakeContainer());
    await settle();
    const { puppet, calls } = fakePuppet();
    pending[0]?.resolve(puppet);
    await attached;
    const canvas = pending[0]?.canvas;
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    // wasm は canvas の webglcontextlost で is_lost を立てる。復元を待ってから、描くのを待たずに作り直す。
    puppet.lost = true;
    canvas?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    warn.mockRestore();
    controller.requestRender();
    await settle();
    expect(pending).toHaveLength(1);
    expect(calls).toEqual([]);
    canvas?.dispatchEvent(new Event("webglcontextrestored"));
    expect(controller.puppet).toBeNull();
    expect(calls).toEqual(["free"]);
    expect(canvas?.removed).toBe(true);
    await settle();
    expect(pending).toHaveLength(2);
    const next = fakePuppet();
    pending[1]?.resolve(next.puppet);
    await settle();
    expect(controller.puppet).toBe(next.puppet);
    expect(controller.status).toBe("ready");
    // 作り直した後の古い canvas の知らせは無視する。
    canvas?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    canvas?.dispatchEvent(new Event("webglcontextrestored"));
    await settle();
    expect(pending).toHaveLength(2);
  });

  it("stops drawing and says so after losing the WebGL context repeatedly", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    let now = 0;
    const clock = vi.spyOn(performance, "now").mockImplementation(() => now);
    try {
      const controller = new ViewportController();
      const attached = controller.attach(fakeContainer());
      await settle();
      pending[0]?.resolve(fakePuppet().puppet);
      await attached;
      for (let i = 0; i <= MAX_RESTARTS; i++) {
        now += 1000;
        const canvas = pending[i]?.canvas;
        canvas?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
        canvas?.dispatchEvent(new Event("webglcontextrestored"));
        await settle();
        pending[i + 1]?.resolve(fakePuppet().puppet);
        await settle();
      }
      expect(pending).toHaveLength(MAX_RESTARTS + 1);
      expect(controller.puppet).toBeNull();
      expect(controller.status).toBe("error");
      expect(controller.message).toBe(t("viewport.gpuLostRepeatedly"));
    } finally {
      warn.mockRestore();
      clock.mockRestore();
    }
  });

  it("recreates the canvas when the GPU device is lost while rendering", async () => {
    let frame: FrameRequestCallback | null = null;
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      frame = callback;
      return 1;
    });
    try {
      const controller = new ViewportController();
      const attached = controller.attach(fakeContainer());
      await settle();
      const { puppet, calls } = fakePuppet();
      pending[0]?.resolve(puppet);
      await attached;
      (frame as FrameRequestCallback | null)?.(0);
      frame = null;
      // WebGPU のデバイスの喪失はイベントが無いので、次に描くときに気づく。
      puppet.lost = true;
      controller.requestRender();
      (frame as FrameRequestCallback | null)?.(16);
      expect(calls).toEqual(["free"]);
      await settle();
      expect(pending).toHaveLength(2);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
