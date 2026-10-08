// PuppetSurface の作成・作り直し・破棄と、devicePixelRatio の追従。wasm と canvas は偽物。
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import {
  GpuLostRepeatedlyError,
  MAX_RESTARTS,
  PuppetSurface,
  RESTART_WINDOW_MS,
  RESTORE_TIMEOUT_MS,
  type SurfaceHost,
  type SurfacePuppet,
} from "./surface";

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
  powerPreference: string | null | undefined;
  resolve: (p: SurfacePuppet) => void;
  reject: (e: unknown) => void;
}

/** WebPuppet.create が返す約束 (テストが好きな時に解く)。 */
let pending: Pending[] = [];

const wasm = {
  WebPuppet: {
    create: (canvas: HTMLCanvasElement, backend?: string | null, powerPreference?: string | null) =>
      new Promise<SurfacePuppet>((resolve, reject) =>
        pending.push({
          canvas: canvas as unknown as FakeCanvas,
          backend: backend ?? "auto",
          powerPreference,
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
    view: [1, 0, 0],
    zoom: () => puppet.view[0] ?? 1,
    pan_x: () => puppet.view[1] ?? 0,
    pan_y: () => puppet.view[2] ?? 0,
    set_view: (zoom: number, x: number, y: number) => {
      puppet.view = [zoom, x, y];
    },
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
    waiting: () => void log.push("waiting"),
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
  vi.useRealTimers();
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
    // 既定は希望なし
    expect(first?.powerPreference).toBe("default");
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

  it("waits for the WebGL context to be restored before recreating once", async () => {
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
    const event = new Event("webglcontextlost", { cancelable: true });
    lost?.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(warn).toHaveBeenCalled();
    expect(surface.awaitingRestore).toBe(true);
    // 復元を待つ間は作り直さず、描かない (checkLost が真)。
    expect(surface.checkLost()).toBe(true);
    await settle();
    expect(pending).toHaveLength(1);
    expect(first.calls).toEqual(["resize 1x1"]);
    expect(surface.puppet).toBe(first.puppet);
    // 待ち始めたことはすぐ知らせる (「GPU の復元を待っています」を出すため)。
    expect(log).toEqual(["created", "waiting"]);

    lost?.dispatchEvent(new Event("webglcontextrestored"));
    expect(first.calls).toEqual(["resize 1x1", "free"]);
    expect(lost?.removed).toBe(true);
    expect(surface.puppet).toBeNull();
    expect(surface.awaitingRestore).toBe(false);
    expect(surface.recovering).toBe(true);
    // 描く前の確かめは、作り直している間は何もしない。
    expect(surface.checkLost()).toBe(false);
    await settle();
    expect(pending).toHaveLength(2);
    expect(pending[1]?.backend).toBe("auto");
    const second = fakePuppet();
    pending[1]?.resolve(second.puppet);
    await settle();
    expect(surface.puppet).toBe(second.puppet);
    expect(surface.recovering).toBe(false);
    expect(log).toEqual(["created", "waiting", "lost", "released", "recovered"]);
    // 作り直した後の古い canvas の知らせは無視する。
    lost?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    lost?.dispatchEvent(new Event("webglcontextrestored"));
    await settle();
    expect(pending).toHaveLength(2);
    expect(surface.awaitingRestore).toBe(false);
  });

  it("ignores webglcontextrestored without a lost context", async () => {
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    pending[0]?.resolve(fakePuppet().puppet);
    await attached;
    pending[0]?.canvas.dispatchEvent(new Event("webglcontextrestored"));
    await settle();
    expect(pending).toHaveLength(1);
    expect(log).toEqual(["created"]);
  });

  it("gives up after losing the GPU repeatedly and starts over on the next attach", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    let now = 1000;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    pending[0]?.resolve(fakePuppet().puppet);
    await attached;

    /** 今の canvas の WebGL2 のコンテキストを失い、戻す。 */
    const loseAndRestore = async () => {
      const canvas = pending[pending.length - 1]?.canvas;
      canvas?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
      canvas?.dispatchEvent(new Event("webglcontextrestored"));
      await settle();
    };
    for (let i = 1; i <= MAX_RESTARTS; i++) {
      now += 10_000;
      await loseAndRestore();
      expect(pending).toHaveLength(i + 1);
      pending[i]?.resolve(fakePuppet().puppet);
      await settle();
    }
    now += 10_000;
    await loseAndRestore();
    // 4 回目は作り直さずに諦める。
    expect(pending).toHaveLength(MAX_RESTARTS + 1);
    expect(surface.puppet).toBeNull();
    expect(surface.recovering).toBe(false);
    expect(pending[MAX_RESTARTS]?.canvas.removed).toBe(true);
    expect(log.slice(-4)).toEqual([
      "waiting",
      "lost",
      "released",
      "failed the GPU device was lost repeatedly",
    ]);
    expect(surface.checkLost()).toBe(false);

    // 次の attach では数え直す。
    const again = surface.attach(container);
    await settle();
    pending[MAX_RESTARTS + 1]?.resolve(fakePuppet().puppet);
    expect(await again).toBe("ready");
    await loseAndRestore();
    expect(pending).toHaveLength(MAX_RESTARTS + 3);
  });

  it("passes a GpuLostRepeatedlyError to failed", async () => {
    let now = 0;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    const errors: unknown[] = [];
    const { host: h } = host({ failed: (e) => void errors.push(e) });
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    let current = fakePuppet();
    pending[0]?.resolve(current.puppet);
    await attached;
    for (let i = 0; i <= MAX_RESTARTS; i++) {
      now += 1;
      current.puppet.lost = true;
      expect(surface.checkLost()).toBe(true);
      await settle();
      current = fakePuppet();
      pending[i + 1]?.resolve(current.puppet);
      await settle();
    }
    expect(errors).toHaveLength(1);
    expect(errors[0]).toBeInstanceOf(GpuLostRepeatedlyError);
    // どの作り直しも作れていた (作っては失った) ので、作れなかった理由は無い。
    expect((errors[0] as Error).cause).toBeUndefined();
  });

  it("does not count losses that are further apart than the window", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    let now = 0;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    let current = fakePuppet();
    pending[0]?.resolve(current.puppet);
    await attached;
    for (let i = 0; i < MAX_RESTARTS * 2; i++) {
      now += RESTART_WINDOW_MS + 1;
      current.puppet.lost = true;
      expect(surface.checkLost()).toBe(true);
      await settle();
      expect(pending).toHaveLength(i + 2);
      current = fakePuppet();
      pending[i + 1]?.resolve(current.puppet);
      await settle();
    }
    expect(log.filter((entry) => entry.startsWith("failed"))).toEqual([]);
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

  it("gives the recreated WebPuppet the view of the lost one", async () => {
    vi.stubGlobal("devicePixelRatio", 2);
    const views: number[][] = [];
    const { host: h } = host({
      created: (puppet) =>
        void views.push([...(puppet as ReturnType<typeof fakePuppet>["puppet"]).view]),
    });
    const surface = new PuppetSurface(h);
    surface.resize(200, 100);
    const attached = surface.attach(container);
    await settle();
    const first = fakePuppet();
    pending[0]?.resolve(first.puppet);
    await attached;
    // 利用者がズームとパンを動かした (パンは描画バッファの画素)。
    first.puppet.view = [3, 40, -20];
    first.puppet.lost = true;
    surface.checkLost();
    // 作り直す前に倍率が変わっても、CSS ピクセルで同じ場所を見せる。
    vi.stubGlobal("devicePixelRatio", 1);
    surface.resize(200, 100);
    await settle();
    const second = fakePuppet();
    pending[1]?.resolve(second.puppet);
    await settle();
    // `created` の前に渡す (Player はモデルを読み直す前にビューを読む)。
    expect(views).toEqual([
      [1, 0, 0],
      [3, 20, -10],
    ]);

    // 作り直しでない作成 (次の attach) は既定のビューから始める。
    const reattached = surface.attach(container);
    await settle();
    const third = fakePuppet();
    pending[2]?.resolve(third.puppet);
    await reattached;
    expect(third.puppet.view).toEqual([1, 0, 0]);
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

  it("recreates on a new canvas when the WebGL context is not restored in time", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    pending[0]?.resolve(fakePuppet().puppet);
    await attached;

    const lost = pending[0]?.canvas;
    lost?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    expect(log).toEqual(["created", "waiting"]);
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS - 1);
    expect(pending).toHaveLength(1);
    expect(surface.awaitingRestore).toBe(true);
    await vi.advanceTimersByTimeAsync(1);
    expect(surface.awaitingRestore).toBe(false);
    expect(lost?.removed).toBe(true);
    expect(pending).toHaveLength(2);
    pending[1]?.resolve(fakePuppet().puppet);
    await settle();
    expect(log).toEqual(["created", "waiting", "lost", "released", "recovered"]);
    // 遅れて届いた古い canvas の復元は無視する。
    lost?.dispatchEvent(new Event("webglcontextrestored"));
    await settle();
    expect(pending).toHaveLength(2);
  });

  it("does not time out once the WebGL context is restored", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const { host: h } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    pending[0]?.resolve(fakePuppet().puppet);
    await attached;
    const lost = pending[0]?.canvas;
    lost?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    lost?.dispatchEvent(new Event("webglcontextrestored"));
    await settle();
    pending[1]?.resolve(fakePuppet().puppet);
    await settle();
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS * 2);
    expect(pending).toHaveLength(2);
  });

  it("retries a recreation that fails and gives up after the limit", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    // WebGL2 に落ちておく (作り直しは 1 回に 1 つの canvas)。
    pending[0]?.reject(new Error("no WebGPU"));
    await settle();
    pending[1]?.resolve(fakePuppet().puppet);
    await attached;

    pending[1]?.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS);
    for (let i = 0; i < MAX_RESTARTS; i++) {
      expect(pending).toHaveLength(3 + i);
      expect(pending[2 + i]?.backend).toBe("webgl2");
      pending[2 + i]?.reject(new Error("context lost"));
      await settle();
      expect(pending[2 + i]?.canvas.removed).toBe(true);
      // 次に試すまで待つ間も「待っている」。
      expect(log[log.length - 1]).toBe("waiting");
      await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS);
    }
    expect(pending).toHaveLength(2 + MAX_RESTARTS);
    expect(log.slice(-1)).toEqual(["failed the GPU device was lost repeatedly"]);
    expect(log.filter((entry) => entry.startsWith("failed"))).toHaveLength(1);
    expect(surface.recovering).toBe(false);
  });

  it("keeps the last creation failure as the cause when giving up", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const errors: unknown[] = [];
    const { host: h } = host({ failed: (e) => void errors.push(e) });
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    // 最初の作成で WebGPU も WebGL2 も作れない。
    const first = new Error("no context");
    pending[0]?.reject(new Error("no adapter"));
    await settle();
    pending[1]?.reject(first);
    expect(await attached).toBe("failed");
    expect(errors).toEqual([first]);

    // 「作り直す」で試し直しても作れない (WebGL2 に落ちていないので、毎回 2 つの canvas)。
    const restarting = surface.restart();
    const last = Object.assign(new Error("context_unavailable"), { name: "CreateError" });
    for (let i = 0; i < MAX_RESTARTS; i++) {
      await settle();
      pending[2 + i * 2]?.reject(new Error("no adapter"));
      await settle();
      pending[3 + i * 2]?.reject(i === MAX_RESTARTS - 1 ? last : new Error("no context"));
      await settle();
      await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS);
    }
    await restarting;
    expect(pending).toHaveLength(2 + MAX_RESTARTS * 2);
    expect(errors).toHaveLength(2);
    expect(errors[1]).toBeInstanceOf(GpuLostRepeatedlyError);
    expect((errors[1] as Error).cause).toBe(last);
  });

  it("sets one restore timer however many times the context is lost while waiting", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const { host: h } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    pending[0]?.resolve(fakePuppet().puppet);
    await attached;
    const canvas = pending[0]?.canvas;
    canvas?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    canvas?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    expect(vi.getTimerCount()).toBe(1);
    // 時間切れで作り直し始めた後の知らせ (古い canvas) は、タイマーを置き直さない。
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS);
    canvas?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    expect(vi.getTimerCount()).toBe(0);
    pending[1]?.resolve(fakePuppet().puppet);
    await settle();
    // 作り直した canvas で失えば、新しく 1 つ置く。復元で消える。
    const next = pending[1]?.canvas;
    next?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    expect(vi.getTimerCount()).toBe(1);
    next?.dispatchEvent(new Event("webglcontextrestored"));
    expect(vi.getTimerCount()).toBe(0);
    // detach でも消える。
    await settle();
    pending[2]?.resolve(fakePuppet().puppet);
    await settle();
    pending[2]?.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    expect(vi.getTimerCount()).toBe(1);
    surface.detach();
    expect(vi.getTimerCount()).toBe(0);
  });

  it("recovers when a retried recreation succeeds", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    const first = fakePuppet();
    pending[0]?.resolve(first.puppet);
    await attached;
    first.puppet.lost = true;
    expect(surface.checkLost()).toBe(true);
    await settle();
    // WebGPU も WebGL2 も、まだ作れない。
    pending[1]?.reject(new Error("no device"));
    await settle();
    pending[2]?.reject(new Error("no context"));
    await settle();
    expect(log.slice(-1)).toEqual(["waiting"]);
    expect(surface.recovering).toBe(true);
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS);
    pending[3]?.resolve(fakePuppet().puppet);
    await settle();
    expect(surface.puppet).not.toBeNull();
    expect(log).toEqual(["created", "lost", "released", "waiting", "recovered"]);
  });

  it("stops retrying when detached while waiting", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    const first = fakePuppet();
    pending[0]?.resolve(first.puppet);
    await attached;
    pending[0]?.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    surface.detach();
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS * 2);
    expect(pending).toHaveLength(1);
    expect(log).toEqual(["created", "waiting", "released"]);
  });

  it("restarts on the same container after giving up and counts again", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    let now = 0;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    // attach の前は何もしない。
    await surface.restart();
    expect(pending).toHaveLength(0);
    const attached = surface.attach(container);
    await settle();
    let current = fakePuppet();
    pending[0]?.resolve(current.puppet);
    await attached;
    current.puppet.view = [2, 10, 5];
    for (let i = 0; i <= MAX_RESTARTS; i++) {
      now += 1;
      current.puppet.lost = true;
      surface.checkLost();
      await settle();
      current = fakePuppet();
      pending[i + 1]?.resolve(current.puppet);
      await settle();
    }
    expect(surface.puppet).toBeNull();
    expect(log[log.length - 1]).toBe("failed the GPU device was lost repeatedly");

    const restarting = surface.restart();
    await settle();
    expect(pending).toHaveLength(MAX_RESTARTS + 2);
    const restarted = fakePuppet();
    pending[MAX_RESTARTS + 1]?.resolve(restarted.puppet);
    await restarting;
    expect(surface.puppet).toBe(restarted.puppet);
    expect(log[log.length - 1]).toBe("recovered");
    // 諦めた後の作り直しでも、失う前のビューを渡す。
    expect(restarted.puppet.view).toEqual([2, 10, 5]);
    // 回数は数え直す (もう一度失っても作り直す)。
    restarted.puppet.lost = true;
    surface.checkLost();
    await settle();
    expect(pending).toHaveLength(MAX_RESTARTS + 3);
  });

  it("frees the current WebPuppet when restarting", async () => {
    const { host: h, log } = host();
    const surface = new PuppetSurface(h);
    const attached = surface.attach(container);
    await settle();
    const first = fakePuppet();
    pending[0]?.resolve(first.puppet);
    await attached;
    const restarting = surface.restart();
    expect(first.calls).toEqual(["resize 1x1", "free"]);
    expect(pending[0]?.canvas.removed).toBe(true);
    await settle();
    pending[1]?.resolve(fakePuppet().puppet);
    await restarting;
    expect(log).toEqual(["created", "lost", "released", "recovered"]);
  });

  it("passes the same power preference when creating and recreating", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const { host: h } = host();
    const surface = new PuppetSurface(h);
    expect(surface.powerPreference).toBe("default");
    surface.powerPreference = "high-performance";
    const attached = surface.attach(container);
    await settle();
    // WebGPU で作れずに WebGL2 で作るときも同じ値
    pending[0]?.reject(new Error("no WebGPU"));
    await settle();
    const first = fakePuppet();
    pending[1]?.resolve(first.puppet);
    await attached;
    // GPU を失ったときの作り直しも同じ値
    first.puppet.lost = true;
    surface.checkLost();
    await settle();
    pending[2]?.resolve(fakePuppet().puppet);
    await settle();
    expect(pending.map((p) => p.powerPreference)).toEqual([
      "high-performance",
      "high-performance",
      "high-performance",
    ]);

    // 変えても今の WebPuppet はそのままで、restart から効く
    surface.powerPreference = "low-power";
    expect(pending).toHaveLength(3);
    const restarting = surface.restart();
    await settle();
    pending[3]?.resolve(fakePuppet().puppet);
    await restarting;
    expect(pending[3]?.powerPreference).toBe("low-power");
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
