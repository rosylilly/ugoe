// @vitest-environment happy-dom
// `<ugoe-avatar>` の作成・読み込み・作り直しの順序。wasm は使わず、`WebPuppet` の代わりの偽物を作る (runtime.js を差し替える)。

import { afterEach, beforeEach, describe, expect, expectTypeOf, it, vi } from "vitest";
import { isCreateError } from "./create-errors.js";
import type { WebPuppet } from "./wasm/ugoe_wasm.js";

/** `WebPuppet` の偽物。`<ugoe-avatar>` が呼ぶメソッドだけを持つ。 */
class FakePuppet {
  freed = false;
  lost = false;
  model: unknown = null;
  /** 最後の読み込みに渡した上限 (`loadModel` の `options.limits`)。 */
  limits: unknown = undefined;
  /** `load_warnings_json()` が返す警告の構造。 */
  warningDetails: unknown[] = [];
  values = new Map<string, number>();
  readonly canvas: HTMLCanvasElement;
  readonly backendName: string;

  constructor(canvas: HTMLCanvasElement, backend: string) {
    this.canvas = canvas;
    this.backendName = backend === "webgpu" ? "webgpu" : "webgl2";
  }
  free() {
    this.freed = true;
  }
  backend() {
    return this.backendName;
  }
  has_model() {
    return this.model !== null;
  }
  is_lost() {
    return this.lost;
  }
  resize(_width: number, _height: number) {}
  fit() {}
  update() {}
  render() {}
  unload() {
    this.model = null;
  }
  set_motion_over_tracking() {}
  set_tracking_kept() {}
  set_parameter(id: string, value: number) {
    this.values.set(id, value);
    return true;
  }
  parameter(id: string) {
    return this.values.get(id);
  }
  parameter_ids() {
    return [...this.values.keys()];
  }
  parameters_json() {
    return JSON.stringify({ parameters: [] });
  }
  load_warnings_json() {
    return JSON.stringify(this.warningDetails);
  }
  set_expression(_name: string) {
    throw new Error("no expression");
  }
  stop_expressions() {}
  stop_motions() {}
}

/** 作成を待たせる・失敗させるための口。 */
const runtime = vi.hoisted(() => ({
  /** 次の作成で投げる例外 (1 回だけ)。 */
  createError: null as Error | null,
  /**
   * 真の間は作成がいつも失敗する (GPU が戻っていない)。ugoe-wasm と同じく、コンテキストを取れない `CreateError`
   * (`code` が `context_unavailable`。T0475) を投げる。
   */
  failing: false,
  /** 真の間は、失敗する作成の前に新しい canvas の WebGL2 のコンテキストを失わせる (失った GPU で作ろうとしたブラウザと同じ)。 */
  loseOnFailure: false,
  contextUnavailable(): Error {
    const e = new Error("the canvas could not provide a GPU context: the WebGL2 context is lost");
    e.name = "CreateError";
    return Object.assign(e, { code: "context_unavailable" });
  },
  /** 読み込むと例外を投げるモデル。 */
  badModel: "bad",
}));

vi.mock("./runtime.js", () => ({
  createPuppet: async (canvas: HTMLCanvasElement, options: { backend?: string } = {}) => {
    await Promise.resolve();
    if (runtime.createError) {
      const e = runtime.createError;
      runtime.createError = null;
      throw e;
    }
    if (runtime.failing) {
      if (runtime.loseOnFailure)
        canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
      throw runtime.contextUnavailable();
    }
    const puppet = new FakePuppet(canvas, options.backend ?? "auto");
    return puppet;
  },
  loadModel: (puppet: FakePuppet, data: unknown, options: { limits?: unknown } = {}) => {
    if (data === runtime.badModel) throw new Error("invalid model");
    puppet.limits = options.limits;
    puppet.model = typeof data === "string" ? data : new TextDecoder().decode(data as Uint8Array);
    return [];
  },
}));

const { defineUgoeAvatar } = await import("./avatar.js");
defineUgoeAvatar();
type Avatar = import("./avatar.js").UgoeAvatarElement;
type UgoeAvatarLoadDetail = import("./avatar.js").UgoeAvatarLoadDetail;
type UgoeAvatarErrorDetail = import("./avatar.js").UgoeAvatarErrorDetail;

const fake = (p: WebPuppet | null) => p as unknown as FakePuppet;
/** マイクロタスクと rAF (happy-dom はタイマーで回す) を流す。 */
const flush = () => new Promise((r) => setTimeout(r, 30));

/** 解決を外から決める `fetch` の偽物。`requests` に呼ばれた順の設定を残す。 */
function controlledFetch(requests: { key: string; init: RequestInit | undefined }[] = []) {
  const pending = new Map<string, (body: string) => void>();
  vi.stubGlobal("fetch", (url: URL, init?: RequestInit) => {
    const key = url.pathname.slice(1);
    requests.push({ key, init });
    return new Promise<Response>((resolve, reject) => {
      pending.set(key, (body) => resolve(new Response(body)));
      init?.signal?.addEventListener("abort", () => reject(init.signal?.reason));
    });
  });
  return (key: string, body: string) => {
    const respond = pending.get(key);
    if (!respond) throw new Error(`no pending fetch for ${key}`);
    pending.delete(key);
    respond(body);
  };
}

function newAvatar(attributes: Record<string, string> = {}): Avatar {
  const avatar = document.createElement("ugoe-avatar");
  for (const [k, v] of Object.entries(attributes)) avatar.setAttribute(k, v);
  return avatar;
}

beforeEach(() => {
  runtime.createError = null;
  runtime.failing = false;
  runtime.loseOnFailure = false;
});

afterEach(() => {
  vi.useRealTimers();
  document.body.replaceChildren();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("<ugoe-avatar>", () => {
  it("types the detail of load and error events", async () => {
    const avatar = newAvatar();
    const loads: UgoeAvatarLoadDetail[] = [];
    const errors: Error[] = [];
    const onLoad = (e: CustomEvent<UgoeAvatarLoadDetail>) => loads.push(e.detail);
    avatar.addEventListener("load", onLoad);
    avatar.addEventListener("error", function (e) {
      expectTypeOf(this).toEqualTypeOf<Avatar>();
      expectTypeOf(e).toEqualTypeOf<CustomEvent<UgoeAvatarErrorDetail>>();
      errors.push(e.detail.error);
    });
    // HTMLElement のほかのイベントの型は変わらない
    avatar.addEventListener("keydown", (e) => expectTypeOf(e).toEqualTypeOf<KeyboardEvent>());
    // @ts-expect-error `load` の detail に `error` は無い
    avatar.addEventListener("load", (e) => e.detail.error);
    avatar.addEventListener("x-custom", (e) => expectTypeOf(e).toEqualTypeOf<Event>());
    document.body.append(avatar);
    const puppet = fake(await avatar.ready);
    puppet.warningDetails = [{ code: "texture_ktx2", warning: "w", texture: 0 }];
    await avatar.load("model-a");
    await expect(avatar.load(runtime.badModel)).rejects.toThrow("invalid model");
    avatar.setAttribute("expression", "missing");
    expect(loads).toEqual([
      {
        warnings: [],
        warningDetails: [{ code: "texture_ktx2", warning: "w", texture: 0 }],
        backend: "webgl2",
      },
    ]);
    expect(avatar.warningDetails).toEqual(loads[0]?.warningDetails);
    expect(errors.map((e) => e.message)).toEqual(["no expression"]);

    avatar.removeEventListener("load", onLoad);
    await avatar.load("model-b");
    expect(loads).toHaveLength(1);
  });

  it("passes the load limits to the next load", async () => {
    const avatar = newAvatar();
    document.body.append(avatar);
    const puppet = fake(await avatar.ready);
    expect(avatar.loadLimits).toBeNull();
    await avatar.load("model-a");
    expect(puppet.limits).toBeUndefined();
    avatar.loadLimits = { maxTotalDecompressed: 1024 };
    // 変えただけでは読み直さない。
    expect(puppet.limits).toBeUndefined();
    await avatar.load("model-b");
    expect(puppet.limits).toEqual({ maxTotalDecompressed: 1024 });
  });

  it("creates a puppet when connected and frees it when disconnected", async () => {
    const avatar = newAvatar();
    document.body.append(avatar);
    const puppet = fake(await avatar.ready);
    expect(avatar.puppet).toBe(puppet);
    avatar.remove();
    expect(puppet.freed).toBe(true);
    expect(avatar.puppet).toBeNull();
  });

  it("recreates the puppet after the WebGL context is restored", async () => {
    const avatar = newAvatar({ backend: "webgl2" });
    document.body.append(avatar);
    await avatar.load("model-a");
    const first = fake(avatar.puppet);
    const lost = new Event("webglcontextlost", { cancelable: true });
    first.canvas.dispatchEvent(lost);
    // 復元を待つ (preventDefault しないと復元されない)
    expect(lost.defaultPrevented).toBe(true);
    first.canvas.dispatchEvent(new Event("webglcontextrestored"));
    const second = fake(await avatar.ready);
    expect(second).not.toBe(first);
    expect(first.freed).toBe(true);
    expect(second.canvas).not.toBe(first.canvas);
    await flush();
    expect(second.model).toBe("model-a");
  });

  it("keeps recreating after context losses that are far apart", async () => {
    let now = 0;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    const errors: unknown[] = [];
    const avatar = newAvatar({ backend: "webgl2" });
    avatar.addEventListener("error", (e) => errors.push(e.detail.error));
    document.body.append(avatar);
    await avatar.ready;
    for (let i = 0; i < 5; i++) {
      now += 10 * 60 * 1000; // 10 分ごと
      const canvas = fake(avatar.puppet).canvas;
      canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
      canvas.dispatchEvent(new Event("webglcontextrestored"));
      await avatar.ready;
      await flush();
    }
    expect(errors).toEqual([]);
    expect(avatar.puppet).not.toBeNull();
  });

  it("gives up when the context is lost repeatedly in a short time", async () => {
    const errors: Error[] = [];
    const avatar = newAvatar({ backend: "webgl2" });
    avatar.addEventListener("error", (e) => errors.push(e.detail.error));
    document.body.append(avatar);
    await avatar.ready;
    for (let i = 0; i < 4 && avatar.puppet; i++) {
      const canvas = fake(avatar.puppet).canvas;
      canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
      canvas.dispatchEvent(new Event("webglcontextrestored"));
      await flush();
    }
    expect(errors.map((e) => e.message)).toEqual(["the GPU device was lost repeatedly"]);
    expect(avatar.puppet).toBeNull();
  });

  /** 復元を待つ時間 (avatar.ts の `RESTORE_TIMEOUT_MS`)。 */
  const RESTORE_TIMEOUT_MS = 5_000;

  it("recreates on a new canvas when the WebGL context is not restored in time", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const avatar = newAvatar({ backend: "webgl2" });
    let waits = 0;
    avatar.addEventListener("gpulost", (e) => {
      expectTypeOf(e).toEqualTypeOf<CustomEvent<null>>();
      waits++;
    });
    document.body.append(avatar);
    await avatar.load("model-a");
    const first = fake(avatar.puppet);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    first.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    // 待ち始めたことはすぐ知らせ、`ready` は作り直した WebPuppet を待つ。
    expect(waits).toBe(1);
    expect(avatar.waitingForGpu).toBe(true);
    const recreated = avatar.ready;
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS - 1);
    expect(avatar.puppet).toBe(first);
    await vi.advanceTimersByTimeAsync(1);
    const second = fake(await recreated);
    expect(second).not.toBe(first);
    expect(second.canvas).not.toBe(first.canvas);
    expect(first.freed).toBe(true);
    expect(second.model).toBe("model-a");
    expect(avatar.waitingForGpu).toBe(false);
    // 遅れて届いた古い canvas の復元は無視する。
    first.canvas.dispatchEvent(new Event("webglcontextrestored"));
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS);
    expect(avatar.puppet).toBe(second);
  });

  it("does not time out once the WebGL context is restored", async () => {
    const avatar = newAvatar({ backend: "webgl2" });
    document.body.append(avatar);
    const first = fake(await avatar.ready);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    first.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    first.canvas.dispatchEvent(new Event("webglcontextrestored"));
    const second = fake(await avatar.ready);
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS * 2);
    expect(avatar.puppet).toBe(second);
  });

  it("retries a recreation that fails until the GPU is back", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const errors: Error[] = [];
    const avatar = newAvatar({ backend: "webgl2" });
    avatar.addEventListener("error", (e) => errors.push(e.detail.error));
    document.body.append(avatar);
    await avatar.load("model-a");
    avatar.setParameter("AngleX", 15);
    const first = fake(avatar.puppet);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    runtime.failing = true;
    first.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    first.canvas.dispatchEvent(new Event("webglcontextrestored"));
    await vi.advanceTimersByTimeAsync(0);
    expect(avatar.puppet).toBeNull();
    expect(avatar.waitingForGpu).toBe(true);
    expect(errors).toEqual([]);
    runtime.failing = false;
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS);
    const puppet = fake(await avatar.ready);
    expect(puppet.model).toBe("model-a");
    expect(puppet.values.get("AngleX")).toBe(15);
    expect(avatar.waitingForGpu).toBe(false);
    expect(errors).toEqual([]);
  });

  it("keeps one restore timer when a recreation fails on a canvas whose context is lost", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const avatar = newAvatar({ backend: "webgl2" });
    document.body.append(avatar);
    const first = fake(await avatar.ready);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    runtime.failing = true;
    runtime.loseOnFailure = true;
    first.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    first.canvas.dispatchEvent(new Event("webglcontextrestored"));
    await vi.advanceTimersByTimeAsync(0);
    expect(avatar.puppet).toBeNull();
    expect(avatar.waitingForGpu).toBe(true);
    // 新しい canvas の `webglcontextlost` と作成の失敗の両方が時間切れを置くが、残るのは 1 つだけ。
    expect(vi.getTimerCount()).toBe(1);
    // 残った時間切れで試し直し、GPU が戻っていれば作り直せる。
    runtime.failing = false;
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS);
    expect(fake(await avatar.ready)).not.toBe(first);
    expect(avatar.waitingForGpu).toBe(false);
  });

  it("gives up when the recreation keeps failing", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const errors: Error[] = [];
    let waits = 0;
    const avatar = newAvatar({ backend: "webgl2" });
    avatar.addEventListener("error", (e) => errors.push(e.detail.error));
    avatar.addEventListener("gpulost", () => waits++);
    document.body.append(avatar);
    const first = fake(await avatar.ready);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    runtime.failing = true;
    first.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    // 時間切れで 1 回、試し直しで 2 回作ろうとして、4 回目で諦める (1 分に 4 回)。
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS * 4);
    expect(errors.map((e) => e.message)).toEqual(["the GPU device was lost repeatedly"]);
    expect(waits).toBe(1);
    expect(avatar.waitingForGpu).toBe(false);
    await expect(avatar.ready).rejects.toThrow("the GPU device was lost repeatedly");
    // 諦めた後は試さない。
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS * 4);
    expect(errors).toHaveLength(1);

    runtime.failing = false;
    avatar.restart();
    expect(fake(await avatar.ready)).not.toBeNull();
  });

  it("stops waiting for the GPU when disconnected", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const avatar = newAvatar({ backend: "webgl2" });
    document.body.append(avatar);
    const first = fake(await avatar.ready);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    first.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    avatar.remove();
    expect(avatar.waitingForGpu).toBe(false);
    await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS * 2);
    expect(avatar.puppet).toBeNull();
  });

  it("loads the new src when it is reconnected while fetching", async () => {
    const respond = controlledFetch();
    const avatar = newAvatar({ src: "a" });
    document.body.append(avatar);
    await flush();
    respond("a", "model-a");
    await flush();
    expect(fake(avatar.puppet).model).toBe("model-a");

    avatar.setAttribute("src", "b");
    // 取得の途中で動かす (切断して繋ぎ直す)
    document.body.append(newAvatar());
    document.body.append(avatar);
    await avatar.ready;
    await flush();
    respond("b", "model-b");
    await flush();
    expect(fake(avatar.puppet).model).toBe("model-b");
  });

  it("does not reload a model that failed to load when the puppet is recreated", async () => {
    const errors: unknown[] = [];
    const avatar = newAvatar();
    avatar.addEventListener("error", (e) => errors.push(e.detail.error));
    document.body.append(avatar);
    await avatar.load("model-a");
    await expect(avatar.load(runtime.badModel)).rejects.toThrow("invalid model");
    avatar.setAttribute("backend", "webgl2");
    await avatar.ready;
    await flush();
    expect(fake(avatar.puppet).model).toBe("model-a");
    expect(errors).toEqual([]);
  });

  it("rejects a load before connecting when the model fails to load", async () => {
    const avatar = newAvatar();
    const loading = avatar.load(runtime.badModel);
    document.body.append(avatar);
    await expect(loading).rejects.toThrow("invalid model");
  });

  it("rejects a load before connecting when the puppet cannot be created", async () => {
    runtime.createError = new Error("no GPU");
    const avatar = newAvatar({ backend: "webgpu" });
    avatar.addEventListener("error", () => {});
    const loading = avatar.load("model-a");
    document.body.append(avatar);
    await expect(loading).rejects.toThrow("no GPU");
  });

  it("rejects ready when the puppet cannot be created", async () => {
    runtime.createError = new Error("no GPU");
    const errors: Error[] = [];
    const avatar = newAvatar({ backend: "webgpu" });
    avatar.addEventListener("error", (e) => errors.push(e.detail.error));
    document.body.append(avatar);
    await expect(avatar.ready).rejects.toThrow("no GPU");
    expect(errors.map((e) => e.message)).toEqual(["no GPU"]);
    // 作成に失敗した後の `load` は待たせずに reject する
    await expect(avatar.load("model-a")).rejects.toThrow("no GPU");
  });

  it("reports a CreateError when the first creation cannot get a context", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    runtime.failing = true;
    const errors: Error[] = [];
    const avatar = newAvatar();
    avatar.addEventListener("error", (e) => errors.push(e.detail.error));
    document.body.append(avatar);
    const error = await avatar.ready.catch((e: unknown) => e);
    expect(isCreateError(error) && error.code).toBe("context_unavailable");
    expect(errors).toEqual([error]);
    expect(avatar.waitingForGpu).toBe(false);
  });

  it("restarts after the puppet could not be created", async () => {
    const respond = controlledFetch();
    runtime.createError = new Error("no GPU");
    const avatar = newAvatar({ backend: "webgpu", src: "a" });
    avatar.addEventListener("error", () => {});
    document.body.append(avatar);
    await expect(avatar.ready).rejects.toThrow("no GPU");
    avatar.restart();
    const puppet = fake(await avatar.ready);
    expect(avatar.puppet).toBe(puppet);
    await flush();
    respond("a", "model-a");
    await flush();
    expect(puppet.model).toBe("model-a");
  });

  /** コンテキストを続けて失わせて、作り直しを諦めさせる。 */
  async function loseRepeatedly(avatar: Avatar) {
    for (let i = 0; i < 4 && avatar.puppet; i++) {
      const canvas = fake(avatar.puppet).canvas;
      canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
      canvas.dispatchEvent(new Event("webglcontextrestored"));
      await flush();
    }
  }

  it("rejects ready after giving up and restarts with the last model", async () => {
    const avatar = newAvatar({ backend: "webgl2" });
    avatar.addEventListener("error", () => {});
    document.body.append(avatar);
    await avatar.load("model-a");
    avatar.setParameter("AngleX", 15);
    await loseRepeatedly(avatar);
    expect(avatar.puppet).toBeNull();
    await expect(avatar.ready).rejects.toThrow("the GPU device was lost repeatedly");

    avatar.restart();
    const puppet = fake(await avatar.ready);
    await flush();
    expect(puppet.model).toBe("model-a");
    // 諦めたときのパラメータの値も引き継ぐ
    expect(puppet.values.get("AngleX")).toBe(15);
    // 作り直しの回数も数え直す (もう一度失っても作り直す)
    const canvas = puppet.canvas;
    canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    canvas.dispatchEvent(new Event("webglcontextrestored"));
    const next = fake(await avatar.ready);
    expect(next).not.toBe(puppet);
  });

  it("recreates the puppet when the backend changes after giving up", async () => {
    const avatar = newAvatar({ backend: "webgl2" });
    avatar.addEventListener("error", () => {});
    document.body.append(avatar);
    await avatar.load("model-a");
    await loseRepeatedly(avatar);
    await expect(avatar.ready).rejects.toThrow();
    avatar.setAttribute("backend", "webgpu");
    const puppet = fake(await avatar.ready);
    expect(puppet.backend()).toBe("webgpu");
    await flush();
    expect(puppet.model).toBe("model-a");
  });

  it("resizes the drawing buffer when only the device pixel ratio changes", async () => {
    const queries: { media: string; target: EventTarget }[] = [];
    vi.stubGlobal("matchMedia", (media: string) => {
      const target = new EventTarget();
      queries.push({ media, target });
      return target;
    });
    let observed: ResizeObserverCallback | null = null;
    vi.stubGlobal(
      "ResizeObserver",
      class {
        constructor(callback: ResizeObserverCallback) {
          observed = callback;
        }
        observe() {}
        disconnect() {}
      },
    );
    vi.stubGlobal("devicePixelRatio", 1);
    const avatar = newAvatar();
    document.body.append(avatar);
    const puppet = fake(await avatar.ready);
    const sizes: number[][] = [];
    puppet.resize = (width: number, height: number) => void sizes.push([width, height]);
    const entry = { contentRect: { width: 200, height: 100 } } as ResizeObserverEntry;
    (observed as ResizeObserverCallback | null)?.([entry], {} as ResizeObserver);
    expect(sizes).toEqual([[200, 100]]);
    expect(queries.map((q) => q.media)).toEqual(["(resolution: 1dppx)"]);

    vi.stubGlobal("devicePixelRatio", 2);
    queries[0]?.target.dispatchEvent(new Event("change"));
    expect(sizes).toEqual([
      [200, 100],
      [400, 200],
    ]);
    expect(queries.map((q) => q.media)).toEqual(["(resolution: 1dppx)", "(resolution: 2dppx)"]);
    // 外したら聞かない。
    avatar.remove();
    queries[1]?.target.dispatchEvent(new Event("change"));
    expect(queries).toHaveLength(2);
  });

  it("does nothing on restart while disconnected", async () => {
    const avatar = newAvatar();
    avatar.restart();
    await flush();
    expect(avatar.puppet).toBeNull();
    document.body.append(avatar);
    await avatar.ready;
    avatar.remove();
    avatar.restart();
    await flush();
    expect(avatar.puppet).toBeNull();
  });

  it("fetches src with credentials chosen by crossorigin", async () => {
    const requests: { key: string; init: RequestInit | undefined }[] = [];
    const respond = controlledFetch(requests);
    const avatar = newAvatar({ src: "a" });
    document.body.append(avatar);
    await flush();
    respond("a", "model-a");
    avatar.crossOrigin = "use-credentials";
    expect(avatar.getAttribute("crossorigin")).toBe("use-credentials");
    avatar.setAttribute("src", "b");
    await flush();
    respond("b", "model-b");
    avatar.crossOrigin = "anonymous";
    avatar.setAttribute("src", "c");
    await flush();
    expect(requests.map((r) => [r.key, r.init?.credentials])).toEqual([
      ["a", "same-origin"],
      ["b", "include"],
      ["c", "same-origin"],
    ]);
    avatar.crossOrigin = null;
    expect(avatar.hasAttribute("crossorigin")).toBe(false);
  });

  it("aborts a fetch that is superseded", async () => {
    const requests: { key: string; init: RequestInit | undefined }[] = [];
    const respond = controlledFetch(requests);
    const errors: unknown[] = [];
    const avatar = newAvatar({ src: "a" });
    avatar.addEventListener("error", (e) => errors.push(e.detail.error));
    document.body.append(avatar);
    await flush();
    avatar.setAttribute("src", "b");
    expect(requests[0]?.init?.signal?.aborted).toBe(true);
    await avatar.load("model-c");
    expect(requests[1]?.init?.signal?.aborted).toBe(true);
    avatar.setAttribute("src", "d");
    avatar.removeAttribute("src");
    expect(requests[2]?.init?.signal?.aborted).toBe(true);
    await flush();
    expect(errors).toEqual([]);
    // 外しても取得は続ける (戻したら結果を読む)
    avatar.setAttribute("src", "e");
    await flush();
    avatar.remove();
    expect(requests[3]?.init?.signal?.aborted).toBe(false);
    document.body.append(avatar);
    await avatar.ready;
    respond("e", "model-e");
    await flush();
    expect(fake(avatar.puppet).model).toBe("model-e");
  });
});
