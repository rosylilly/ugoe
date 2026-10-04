// PuppetSurface の作成・作り直し・破棄と、devicePixelRatio の追従。wasm と canvas は偽物。
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { PuppetSurface, type SurfaceHost, type SurfacePuppet } from "./surface";

class FakeCanvas extends EventTarget {
  width = 300;
  height = 150;
  className = "";
  removed = false;
  remove(): void {
    this.removed = true;
  }
}

interface Pending {
  canvas: FakeCanvas;
  backend: string;
  resolve: (p: SurfacePuppet) => void;
  reject: (e: unknown) => void;
}

/** WebPuppet.create が返す約束 (テストが好きな時に解く)。 */
let pending: Pending[] = [];

const wasm = {
  WebPuppet: {
    create: (canvas: HTMLCanvasElement, backend?: string | null) =>
      new Promise<SurfacePuppet>((resolve, reject) =>
        pending.push({
          canvas: canvas as unknown as FakeCanvas,
          backend: backend ?? "auto",
          resolve,
          reject,
        }),
      ),
  },
};

const container = { prepend() {} } as unknown as HTMLElement;

function fakePuppet() {
  const calls: string[] = [];
  const puppet = {
    lost: false,
    free: () => void calls.push("free"),
    is_lost: () => puppet.lost,
    resize: (width: number, height: number) => void calls.push(`resize ${width}x${height}`),
  };
  return { puppet, calls };
}

/** 呼ばれた口を順に残す SurfaceHost。 */
function host(overrides: Partial<SurfaceHost<SurfacePuppet>> = {}) {
  const log: string[] = [];
  const value: SurfaceHost<SurfacePuppet> = {
    loadWasm: async () => wasm,
    created: (_puppet, recovered) => void log.push(recovered ? "recovered" : "created"),
    failed: (e) => void log.push(`failed ${e instanceof Error ? e.message : String(e)}`),
    lost: () => void log.push("lost"),
    released: () => void log.push("released"),
    pixelRatioChanged: () => void log.push("pixelRatio"),
    canvasClass: "viewport-canvas",
    ...overrides,
  };
  return { host: value, log };
}

/** 待っているマイクロタスクを流す (loadWasm と create の await の後まで)。 */
async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) await Promise.resolve();
}

beforeAll(() => {
  globalThis.document ??= {
    createElement: () => new FakeCanvas(),
  } as unknown as Document;
});

beforeEach(() => {
  pending = [];
  vi.stubGlobal("devicePixelRatio", 1);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("PuppetSurface", () => {
  it("creates a WebPuppet on a canvas sized to the drawing buffer", async () => {
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    vi.stubGlobal("devicePixelRatio", 2);
    surface.resize(200, 100);
    const attached = surface.attach(container);
    await settle();
    const first = pending[0];
    expect(first?.backend).toBe("auto");
    expect([first?.canvas.width, first?.canvas.height]).toEqual([400, 200]);
    expect(first?.canvas.className).toBe("viewport-canvas");
    const { puppet, calls } = fakePuppet();
    first?.resolve(puppet);
    expect(await attached).toBe("ready");
    expect(surface.puppet).toBe(puppet);
    expect(surface.pixelRatio).toBe(2);
    expect(calls).toEqual(["resize 400x200"]);
    expect(log).toEqual(["created"]);
  });

  it("reports a missing wasm", async () => {
    const { host: h, log } = host({ loadWasm: async () => null });
    const surface = new PuppetSurface(h);
    expect(await surface.attach(container)).toBe("missing");
    expect(pending).toHaveLength(0);
    expect(log).toEqual([]);
  });

  it("falls back to WebGL2 on a new canvas and keeps using it", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    pending[0]?.reject(new Error("no WebGPU"));
    await settle();
    expect(pending[0]?.canvas.removed).toBe(true);
    expect(pending[1]?.backend).toBe("webgl2");
    const first = fakePuppet();
    pending[1]?.resolve(first.puppet);
    expect(await attached).toBe("ready");
    expect(log).toEqual(["created"]);

    // 作り直しは WebGL2 から始める。
    first.puppet.lost = true;
    expect(surface.checkLost()).toBe(true);
    await settle();
    expect(pending[2]?.backend).toBe("webgl2");
  });

  it("reports when neither WebGPU nor WebGL2 works", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    pending[0]?.reject(new Error("no WebGPU"));
    await settle();
    pending[1]?.reject(new Error("no WebGL2"));
    expect(await attached).toBe("failed");
    expect(log).toEqual(["failed no WebGL2"]);
    expect(surface.puppet).toBeNull();
  });

  it("frees a WebPuppet that finishes after detach", async () => {
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    surface.detach();
    const { puppet, calls } = fakePuppet();
    pending[0]?.resolve(puppet);
    expect(await attached).toBe("superseded");
    expect(calls).toEqual(["free"]);
    expect(pending[0]?.canvas.removed).toBe(true);
    expect(surface.puppet).toBeNull();
    expect(log).toEqual([]);
  });

  it("keeps only the WebPuppet of the last attach", async () => {
    const { host: h } = host();
    const surface = new PuppetSurface(h);
    const first = surface.attach(container);
    await settle();
    const second = surface.attach(container);
    await settle();
    const old = fakePuppet();
    const current = fakePuppet();
    pending[1]?.resolve(current.puppet);
    await settle();
    pending[0]?.resolve(old.puppet);
    expect(await Promise.all([first, second])).toEqual(["superseded", "ready"]);
    expect(surface.puppet).toBe(current.puppet);
    expect(old.calls).toEqual(["free"]);
    expect(pending[0]?.canvas.removed).toBe(true);
    expect(pending[1]?.canvas.removed).toBe(false);
  });

  it("recreates once when both is_lost and webglcontextlost tell of a lost context", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    const first = fakePuppet();
    pending[0]?.resolve(first.puppet);
    await attached;

    // wasm は canvas の webglcontextlost で is_lost を立てる。
    const lost = pending[0]?.canvas;
    first.puppet.lost = true;
    lost?.dispatchEvent(new Event("webglcontextlost"));
    expect(warn).toHaveBeenCalled();
    expect(first.calls).toEqual(["resize 1x1", "free"]);
    expect(lost?.removed).toBe(true);
    expect(surface.puppet).toBeNull();
    expect(surface.recovering).toBe(true);
    // 描く前の確かめは、作り直している間は何もしない。
    expect(surface.checkLost()).toBe(false);
    await settle();
    expect(pending).toHaveLength(2);
    const second = fakePuppet();
    pending[1]?.resolve(second.puppet);
    await settle();
    expect(surface.puppet).toBe(second.puppet);
    expect(surface.recovering).toBe(false);
    expect(log).toEqual(["created", "lost", "released", "recovered"]);
    // 作り直した後の古い canvas の知らせは無視する。
    lost?.dispatchEvent(new Event("webglcontextlost"));
    await settle();
    expect(pending).toHaveLength(2);
  });

  it("recreates when is_lost turns true without an event (WebGPU)", async () => {
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    const first = fakePuppet();
    pending[0]?.resolve(first.puppet);
    await attached;
    expect(surface.checkLost()).toBe(false);
    first.puppet.lost = true;
    expect(surface.checkLost()).toBe(true);
    await settle();
    pending[1]?.resolve(fakePuppet().puppet);
    await settle();
    expect(log).toEqual(["created", "lost", "released", "recovered"]);
  });

  it("frees the recreated WebPuppet when detached while recreating", async () => {
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    const first = fakePuppet();
    pending[0]?.resolve(first.puppet);
    await attached;
    first.puppet.lost = true;
    const recovering = surface.recover();
    await settle();
    surface.detach();
    const second = fakePuppet();
    pending[1]?.resolve(second.puppet);
    await recovering;
    expect(second.calls).toEqual(["free"]);
    expect(surface.puppet).toBeNull();
    expect(surface.recovering).toBe(false);
    expect(log).toEqual(["created", "lost", "released"]);
  });

  it("resizes the drawing buffer when only the device pixel ratio changes", async () => {
    const queries: { media: string; target: EventTarget }[] = [];
    vi.stubGlobal("matchMedia", (media: string) => {
      const target = new EventTarget();
      queries.push({ media, target });
      return target;
    });
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    surface.resize(200, 100);
    const attached = surface.attach(container);
    await settle();
    const { puppet, calls } = fakePuppet();
    pending[0]?.resolve(puppet);
    await attached;
    expect(calls).toEqual(["resize 200x100"]);
    expect(queries.map((q) => q.media)).toEqual(["(resolution: 1dppx)"]);

    vi.stubGlobal("devicePixelRatio", 2);
    queries[0]?.target.dispatchEvent(new Event("change"));
    expect(calls).toEqual(["resize 200x100", "resize 400x200"]);
    expect(surface.pixelRatio).toBe(2);
    expect(log).toEqual(["created", "pixelRatio"]);
    expect(queries.map((q) => q.media)).toEqual(["(resolution: 1dppx)", "(resolution: 2dppx)"]);
    // detach したら聞かない。
    surface.detach();
    queries[1]?.target.dispatchEvent(new Event("change"));
    expect(queries).toHaveLength(2);
  });
});
