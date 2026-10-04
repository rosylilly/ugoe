// @vitest-environment happy-dom
// `<ugoe-avatar>` の作成・読み込み・作り直しの順序。wasm は使わず、`WebPuppet` の代わりの偽物を作る (runtime.js を差し替える)。

import { afterEach, beforeEach, describe, expect, expectTypeOf, it, vi } from "vitest";
import type { WebPuppet } from "./wasm/ugoe_wasm.js";

/** `WebPuppet` の偽物。`<ugoe-avatar>` が呼ぶメソッドだけを持つ。 */
class FakePuppet {
  freed = false;
  lost = false;
  model: unknown = null;
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
  resize() {}
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
    const puppet = new FakePuppet(canvas, options.backend ?? "auto");
    return puppet;
  },
  loadModel: (puppet: FakePuppet, data: unknown) => {
    if (data === runtime.badModel) throw new Error("invalid model");
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

/** 解決を外から決める `fetch` の偽物。 */
function controlledFetch() {
  const pending = new Map<string, (body: string) => void>();
  vi.stubGlobal("fetch", (url: URL) => {
    const key = url.pathname.slice(1);
    return new Promise<Response>((resolve) => {
      pending.set(key, (body) => resolve(new Response(body)));
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
});

afterEach(() => {
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
});
