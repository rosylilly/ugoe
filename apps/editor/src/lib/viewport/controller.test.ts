// ViewportController の WebPuppet の作成と破棄 (attach・detach・作り直し)。wasm と canvas は偽物。
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { ViewportController as ViewportControllerType } from "./controller.svelte";
import type { WebPuppet } from "./wasm";

/** WebPuppet.create が返す約束 (テストが好きな時に解く)。 */
let pending: { resolve: (p: WebPuppet) => void; reject: (e: unknown) => void }[] = [];

vi.mock("./wasm", () => ({
  loadUgoeWasm: async () => ({
    WebPuppet: {
      create: () => new Promise<WebPuppet>((resolve, reject) => pending.push({ resolve, reject })),
    },
  }),
}));

let ViewportController: typeof ViewportControllerType;

beforeAll(async () => {
  globalThis.requestAnimationFrame ??= () => 1;
  globalThis.cancelAnimationFrame ??= () => {};
  ({ ViewportController } = await import("./controller.svelte"));
});

beforeEach(() => {
  pending = [];
});

function fakePuppet() {
  const calls: string[] = [];
  const puppet = {
    free: () => calls.push("free"),
    backend: () => "webgl2",
    set_background() {},
    set_lighting_json() {},
    resize() {},
    has_model: () => false,
  };
  return { puppet: puppet as unknown as WebPuppet, calls };
}

function fakeCanvas(): HTMLCanvasElement {
  return new EventTarget() as unknown as HTMLCanvasElement;
}

/** 待っているマイクロタスクを流す (loadUgoeWasm と create の await の後まで)。 */
async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) await Promise.resolve();
}

describe("ViewportController", () => {
  it("makes a WebPuppet for the canvas", async () => {
    const controller = new ViewportController();
    const attached = controller.attach(fakeCanvas());
    await settle();
    const { puppet } = fakePuppet();
    pending[0]?.resolve(puppet);
    await attached;
    expect(controller.puppet).toBe(puppet);
    expect(controller.status).toBe("ready");
  });

  it("frees a WebPuppet that finishes after detach", async () => {
    const controller = new ViewportController();
    const attached = controller.attach(fakeCanvas());
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
    const first = controller.attach(fakeCanvas());
    await settle();
    const second = controller.attach(fakeCanvas());
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
    const canvas = fakeCanvas();
    const attached = controller.attach(canvas);
    await settle();
    const { puppet, calls } = fakePuppet();
    pending[0]?.resolve(puppet);
    await attached;
    const generation = controller.generation;
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    canvas.dispatchEvent(new Event("webglcontextlost"));
    warn.mockRestore();
    expect(controller.generation).toBe(generation + 1);
    expect(controller.puppet).toBeNull();
    expect(calls).toEqual(["free"]);
    // 作り直した後の古い canvas の知らせは無視する。
    canvas.dispatchEvent(new Event("webglcontextlost"));
    expect(controller.generation).toBe(generation + 1);
  });
});
