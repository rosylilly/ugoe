// ViewportController の WebPuppet の作成と破棄 (attach・detach・作り直し)。wasm と canvas は偽物。

import { setLocale } from "@ugoe/i18n";
import { MAX_RESTARTS, RESTORE_TIMEOUT_MS } from "@ugoe/player";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { t } from "../i18n";
import type { ViewportController as ViewportControllerType } from "./controller.svelte";
import { REVEAL_MARGIN } from "./tools/keys";
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
    view: [1, 0, 0],
    zoom: () => puppet.view[0] ?? 1,
    pan_x: () => puppet.view[1] ?? 0,
    pan_y: () => puppet.view[2] ?? 0,
    set_view: (zoom: number, x: number, y: number) => {
      puppet.view = [zoom, x, y];
    },
    zoom_at: (factor: number) => {
      puppet.view[0] = (puppet.view[0] ?? 1) * factor;
    },
    pan_by: (dx: number, dy: number) => {
      puppet.view[1] = (puppet.view[1] ?? 0) + dx;
      puppet.view[2] = (puppet.view[2] ?? 0) + dy;
    },
  };
  return { puppet: puppet as unknown as WebPuppet & { lost: boolean; view: number[] }, calls };
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

  it("pans a point outside the view into it and leaves a point inside alone", async () => {
    const controller = new ViewportController();
    const attached = controller.attach(fakeContainer());
    await settle();
    const { puppet } = fakePuppet();
    pending[0]?.resolve(puppet);
    await attached;
    // 大きさを知らないうちは動かさない
    controller.reveal(-100, 50);
    expect(puppet.view).toEqual([1, 0, 0]);
    controller.resize(400, 300);
    controller.reveal(200, 150);
    expect(puppet.view).toEqual([1, 0, 0]);
    controller.reveal(-100, 50);
    expect(puppet.view).toEqual([1, 100 + REVEAL_MARGIN, 0]);
  });

  it("reveals a point below the strip covering the top edge (T1653)", async () => {
    const controller = new ViewportController();
    const attached = controller.attach(fakeContainer());
    await settle();
    const { puppet } = fakePuppet();
    pending[0]?.resolve(puppet);
    await attached;
    controller.resize(400, 300);
    // ビュー全体では余白の内側の点も、上の 120px を帯が隠すと帯の下の端から余白の所まで下げる
    controller.reveal(200, 60);
    expect(puppet.view).toEqual([1, 0, 0]);
    controller.setCovered("top", 120);
    controller.reveal(200, 60);
    expect(puppet.view).toEqual([1, 0, 120 + REVEAL_MARGIN - 60]);
    // 帯を外せば (ToolOverlay を閉じた) 元の判定に戻る
    controller.setCovered("top", 0);
    controller.reveal(200, 60);
    expect(puppet.view).toEqual([1, 0, 120 + REVEAL_MARGIN - 60]);
  });

  it("keeps the zoom and the pan when the WebPuppet is recreated", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      const controller = new ViewportController();
      const attached = controller.attach(fakeContainer());
      await settle();
      const first = fakePuppet();
      pending[0]?.resolve(first.puppet);
      await attached;
      controller.zoomAt(2, 0, 0);
      controller.panBy(5, -5);
      expect(first.puppet.view).toEqual([2, 5, -5]);
      const canvas = pending[0]?.canvas;
      first.puppet.lost = true;
      canvas?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
      canvas?.dispatchEvent(new Event("webglcontextrestored"));
      await settle();
      const next = fakePuppet();
      pending[1]?.resolve(next.puppet);
      await settle();
      expect(controller.puppet).toBe(next.puppet);
      // 作り直しは利用者の操作ではないので、見ている場所を変えない (PuppetSurface が渡し直す。T0942)。
      expect(next.puppet.view).toEqual([2, 5, -5]);
    } finally {
      warn.mockRestore();
    }
  });

  it("says it is waiting for the GPU until the WebGL context is restored", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      const controller = new ViewportController();
      const attached = controller.attach(fakeContainer());
      await settle();
      pending[0]?.resolve(fakePuppet().puppet);
      await attached;
      const canvas = pending[0]?.canvas;
      canvas?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
      expect(controller.status).toBe("loading");
      expect(controller.message).toBe(t("viewport.gpuWaiting"));
      canvas?.dispatchEvent(new Event("webglcontextrestored"));
      await settle();
      pending[1]?.resolve(fakePuppet().puppet);
      await settle();
      expect(controller.status).toBe("ready");
      expect(controller.message).toBeNull();
      expect(controller.canRestart).toBe(false);
    } finally {
      warn.mockRestore();
    }
  });

  it("translates a CreateError and offers a restart", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      const createError = () =>
        Object.assign(new Error("could not get a webgl2 context"), {
          name: "CreateError",
          code: "context_unavailable",
        });
      const controller = new ViewportController();
      const attached = controller.attach(fakeContainer());
      // WebGPU で作れず、WebGL2 でも作れない。
      await settle();
      pending[0]?.reject(createError());
      await settle();
      pending[1]?.reject(createError());
      await attached;
      expect(controller.status).toBe("error");
      expect(controller.message).toBe(t("viewport.createError.contextUnavailable"));
      expect(controller.canRestart).toBe(true);

      const restarting = controller.restart();
      await settle();
      pending[2]?.resolve(fakePuppet().puppet);
      await restarting;
      expect(controller.status).toBe("ready");
      expect(controller.message).toBeNull();
    } finally {
      warn.mockRestore();
    }
  });

  it("says why it could not draw when restarting keeps failing", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      const createError = (code: string) =>
        Object.assign(new Error(`create failed: ${code}`), { name: "CreateError", code });
      const controller = new ViewportController();
      const attached = controller.attach(fakeContainer());
      await settle();
      pending[0]?.reject(createError("adapter_unavailable"));
      await settle();
      pending[1]?.reject(createError("context_unavailable"));
      await attached;
      expect(controller.message).toBe(t("viewport.createError.contextUnavailable"));

      // 「作り直す」を押しても作れないまま、試し直しの上限を超える。
      const restarting = controller.restart();
      for (let i = 0; i < MAX_RESTARTS; i++) {
        await settle();
        pending.at(-1)?.reject(createError("adapter_unavailable"));
        await settle();
        pending.at(-1)?.reject(createError("device_unavailable"));
        await settle();
        expect(controller.message).toBe(t("viewport.gpuWaiting"));
        await vi.advanceTimersByTimeAsync(RESTORE_TIMEOUT_MS);
      }
      await restarting;
      expect(pending).toHaveLength(2 + MAX_RESTARTS * 2);
      expect(controller.status).toBe("error");
      expect(controller.canRestart).toBe(true);
      // 「GPU を続けて失った」ではなく、作れなかった理由の文に、作り直しを諦めたことを添える。
      expect(controller.message).toBe(
        t("viewport.recreateFailed", { reason: t("viewport.createError.deviceUnavailable") }),
      );
    } finally {
      warn.mockRestore();
      vi.useRealTimers();
    }
  });

  it("keeps the exception text and no restart for a failure without a code", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      const controller = new ViewportController();
      const attached = controller.attach(fakeContainer());
      await settle();
      pending[0]?.reject(new Error("no webgpu"));
      await settle();
      pending[1]?.reject(new Error("unknown backend"));
      await attached;
      expect(controller.message).toBe("unknown backend");
      expect(controller.canRestart).toBe(false);
    } finally {
      warn.mockRestore();
    }
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
      expect(controller.canRestart).toBe(true);

      // 「作り直す」で同じ場所に作り直す (回数は数え直す)。
      const restarting = controller.restart();
      expect(controller.canRestart).toBe(false);
      expect(controller.status).toBe("loading");
      await settle();
      expect(pending).toHaveLength(MAX_RESTARTS + 2);
      const restarted = fakePuppet();
      pending[MAX_RESTARTS + 1]?.resolve(restarted.puppet);
      await restarting;
      expect(controller.puppet).toBe(restarted.puppet);
      expect(controller.status).toBe("ready");
      expect(controller.message).toBeNull();
    } finally {
      warn.mockRestore();
      clock.mockRestore();
    }
  });

  it("tells whether a model is loaded (T1655)", async () => {
    const controller = new ViewportController();
    const attached = controller.attach(fakeContainer());
    await settle();
    const { puppet } = fakePuppet();
    let loaded = false;
    let fail = false;
    Object.assign(puppet, {
      has_model: () => loaded,
      load_json: () => {
        if (fail)
          throw Object.assign(new Error("broken"), { name: "LoadError", code: "invalid_model" });
        loaded = true;
        return [];
      },
      uses_lighting: () => false,
      set_enabled() {},
      reset_parameters() {},
      parameter_ids: () => [],
    });
    pending[0]?.resolve(puppet);
    await attached;
    // 読み込む前
    expect(controller.hasModel).toBe(false);
    controller.setModel({ json: "{}", bundle: null });
    expect(controller.hasModel).toBe(true);
    // 読み込みの失敗 (WebPuppet が前のモデルを持っていても描くものとしない)
    fail = true;
    controller.setModel({ json: '{"broken": true}', bundle: null });
    expect(controller.status).toBe("error");
    expect(controller.hasModel).toBe(false);
  });

  it("translates the load error of the model from its code", async () => {
    const controller = new ViewportController();
    const attached = controller.attach(fakeContainer());
    await settle();
    const { puppet } = fakePuppet();
    let error: Error = new Error("unused");
    Object.assign(puppet, {
      load_json: () => {
        throw error;
      },
    });
    pending[0]?.resolve(puppet);
    await attached;
    setLocale("ja");
    try {
      error = Object.assign(new Error("newer version"), {
        name: "LoadError",
        code: "unsupported_version",
        found: 9,
        supported: 3,
      });
      controller.setModel({ json: "{}", bundle: null });
      expect(controller.status).toBe("error");
      expect(controller.message).toBe(
        "新しい版の ugoe で保存されたファイルです (ファイルの版 9。このアプリが読めるのは版 3 まで)。アプリを更新してから開いてください",
      );
      // 知らないコードは wasm の文 (英語) のまま。
      error = Object.assign(new Error("something new"), { name: "LoadError", code: "future" });
      controller.setModel({ json: "{}", bundle: null });
      expect(controller.message).toBe("something new");
    } finally {
      setLocale("en");
    }
  });

  it("draws the physics preview only while the shape changes (T0982)", async () => {
    let frames: FrameRequestCallback[] = [];
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      frames.push(callback);
      return frames.length;
    });
    vi.stubGlobal("cancelAnimationFrame", () => {});
    try {
      const controller = new ViewportController();
      const attached = controller.attach(fakeContainer());
      await settle();
      const { puppet } = fakePuppet();
      let renders = 0;
      /** `update` のたびに形が動くか (物理が揺れている)。 */
      let moving = true;
      let changed = true;
      Object.assign(puppet, {
        has_model: () => true,
        load_json: () => [],
        uses_lighting: () => false,
        set_enabled() {},
        reset_parameters() {},
        reset_physics() {},
        parameter_ids: () => [],
        update() {
          if (moving) changed = true;
        },
        shape_changed: () => changed,
        render() {
          renders++;
          changed = false;
        },
      });
      pending[0]?.resolve(puppet);
      await attached;
      controller.setModel({ json: "{}", bundle: null });
      let now = 0;
      /** 待っている `requestAnimationFrame` を 1 回分流し、描いた回数を返す。 */
      const tick = (): number => {
        const before = renders;
        const run = frames;
        frames = [];
        for (const callback of run) callback(now);
        now += 16;
        return renders - before;
      };
      tick();
      controller.startPhysicsPreview();
      expect(tick()).toBe(1);
      expect(tick()).toBe(1);
      // 揺れが収まると描かない (フレームは回し続ける)。
      moving = false;
      expect(tick()).toBe(0);
      expect(tick()).toBe(0);
      const frame = controller.animationFrame;
      // 見せ方の変化はプレビューの外と同じく描く。
      controller.requestRender();
      expect(tick()).toBe(1);
      expect(controller.animationFrame).toBeGreaterThan(frame);
      moving = true;
      expect(tick()).toBe(1);
    } finally {
      vi.unstubAllGlobals();
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
  // 読み込みの途中に GPU を失ったら (`device_lost`)、エラーにせず、作り直した後にそのモデルを読む (T0955。Player・`<ugoe-avatar>` と同じ)。
  describe("when the GPU is lost while loading", () => {
    /** モデルを読める偽の WebPuppet。失っていれば wasm と同じく `device_lost` を投げる。読んだ JSON を `loaded` に残す。 */
    function loadingPuppet() {
      const fake = fakePuppet();
      const loaded: string[] = [];
      Object.assign(fake.puppet, {
        load_json: (json: string) => {
          if (fake.puppet.lost)
            throw Object.assign(new Error("the GPU device was lost"), {
              name: "LoadError",
              code: "device_lost",
            });
          loaded.push(json);
          return [];
        },
        uses_lighting: () => false,
        parameter_ids: () => [],
        view_transform: () => new Float32Array([1, 0, 0]),
      });
      return { ...fake, loaded };
    }

    it("loads the model after the WebGL context comes back", async () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      try {
        const controller = new ViewportController();
        const attached = controller.attach(fakeContainer());
        await settle();
        const first = loadingPuppet();
        pending[0]?.resolve(first.puppet);
        await attached;
        controller.setModel({ json: "a", bundle: null });
        first.puppet.lost = true;
        pending[0]?.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
        controller.setModel({ json: "b", bundle: null });
        expect(controller.status).toBe("loading");
        expect(controller.message).toBe(t("viewport.gpuWaiting"));
        pending[0]?.canvas.dispatchEvent(new Event("webglcontextrestored"));
        await settle();
        const second = loadingPuppet();
        pending[1]?.resolve(second.puppet);
        await settle();
        expect(second.loaded).toEqual(["b"]);
        expect(controller.status).toBe("ready");
        expect(controller.message).toBeNull();
      } finally {
        warn.mockRestore();
      }
    });

    it("recreates the WebGPU device and loads the model", async () => {
      let frame: FrameRequestCallback | null = null;
      vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
        frame = callback;
        return 1;
      });
      try {
        const controller = new ViewportController();
        const attached = controller.attach(fakeContainer());
        await settle();
        const first = loadingPuppet();
        pending[0]?.resolve(first.puppet);
        await attached;
        (frame as FrameRequestCallback | null)?.(0);
        frame = null;
        // WebGPU のデバイスの喪失はイベントが無い。読み込みが `device_lost` を投げたら、次のフレームで作り直す。
        first.puppet.lost = true;
        controller.setModel({ json: "b", bundle: null });
        expect(controller.status).toBe("ready");
        expect(controller.message).toBeNull();
        (frame as FrameRequestCallback | null)?.(16);
        expect(first.calls).toEqual(["free"]);
        await settle();
        const second = loadingPuppet();
        pending[1]?.resolve(second.puppet);
        await settle();
        expect(second.loaded).toEqual(["b"]);
        expect(controller.status).toBe("ready");
        expect(controller.message).toBeNull();
      } finally {
        vi.unstubAllGlobals();
      }
    });
  });
});
