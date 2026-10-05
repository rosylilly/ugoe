// Player の作成と破棄 (attach・detach・GPU を失ったときの作り直し。T0914)。wasm と DOM は偽物 (src/testing/)。
// 着せ替え・モーション・ドライバとトラッキング・受け手のモード・光源は player*.test.ts に流れごとに分ける (T0931・T0933)。
import { describe, expect, it, onTestFinished, vi } from "vitest";
import { t } from "./i18n";
import { MAX_RESTARTS } from "./surface";
import { bundle, fakePuppet } from "./testing/fakePuppet";
import { container, pending, player, settle, setupPlayer } from "./testing/player";

// 画面の部品と MediaPipe まで読み込むと遅いので、Player が使う関数だけにする。
vi.mock("@ugoe/face-tracking", () => import("./testing/faceTracking"));

setupPlayer();

describe("Player", () => {
  it("frees a WebPuppet that finishes after detach", async () => {
    const p = player();
    const attached = p.attach(container);
    await settle();
    p.detach();
    const { puppet, calls } = fakePuppet();
    pending[0]?.resolve(puppet);
    await attached;
    expect(calls).toEqual(["free"]);
    expect(pending[0]?.canvas.removed).toBe(true);
    expect(p.status).toBe("starting");
  });

  it("keeps only the WebPuppet of the last attach", async () => {
    const p = player();
    const first = p.attach(container);
    await settle();
    const second = p.attach(container);
    await settle();
    const old = fakePuppet();
    const current = fakePuppet();
    pending[1]?.resolve(current.puppet);
    await settle();
    pending[0]?.resolve(old.puppet);
    await Promise.all([first, second]);
    expect(old.calls).toEqual(["free"]);
    expect(pending[0]?.canvas.removed).toBe(true);
    expect(current.calls).toEqual([]);
    expect(pending[1]?.canvas.removed).toBe(false);
    expect(p.status).toBe("empty");
  });

  it("recreates the canvas and reloads the model when the WebGL context is lost", async () => {
    const p = player();
    const attached = p.attach(container);
    await settle();
    const first = fakePuppet();
    pending[0]?.resolve(first.puppet);
    await attached;
    const model = bundle("a.ugoe", 1);
    await p.load(model);
    expect(first.loaded).toEqual([model.bytes]);

    const lost = pending[0]?.canvas;
    lost?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    await settle();
    // 復元を待つ間は作り直さない。
    expect(pending).toHaveLength(1);
    expect(first.calls).toEqual([]);
    lost?.dispatchEvent(new Event("webglcontextrestored"));
    await settle();
    expect(first.calls).toEqual(["free"]);
    expect(lost?.removed).toBe(true);
    expect(pending).toHaveLength(2);
    const second = fakePuppet();
    pending[1]?.resolve(second.puppet);
    await settle();
    expect(second.loaded).toEqual([model.bytes]);
    expect(p.status).toBe("ready");
    // 作り直した後の古い canvas の知らせは無視する。
    lost?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    lost?.dispatchEvent(new Event("webglcontextrestored"));
    await settle();
    expect(pending).toHaveLength(2);
  });

  it("stops drawing and says so after losing the GPU repeatedly", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    let now = 0;
    const clock = vi.spyOn(performance, "now").mockImplementation(() => now);
    onTestFinished(() => {
      warn.mockRestore();
      clock.mockRestore();
    });
    const p = player();
    const attached = p.attach(container);
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
    expect(p.status).toBe("unavailable");
    expect(p.error).toBe(t("errors.gpuLostRepeatedly"));
  });

  it("reloads the model that is shown, not one that failed to open, after a lost GPU", async () => {
    const p = player();
    const attached = p.attach(container);
    await settle();
    const first = fakePuppet();
    pending[0]?.resolve(first.puppet);
    await attached;
    const good = bundle("good.ugoe", 1);
    await p.load(good);
    // 読めないモデルと、モデルでないファイル。どちらも前のモデルが残る。
    const broken = fakePuppet({ failLoad: true });
    first.puppet.load_bundle = broken.puppet.load_bundle;
    await p.load(bundle("broken.ugoe", 2));
    // 読み込みの失敗はコードから訳す (T0434)。
    expect(p.error).toBe(
      `broken.ugoe: ${t("loadError.invalidFile", { detail: "not a ugoe file" })}`,
    );
    await p.load({ name: "notes.txt", bytes: new Uint8Array([1, 2, 3]) });

    pending[0]?.canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    pending[0]?.canvas.dispatchEvent(new Event("webglcontextrestored"));
    await settle();
    const second = fakePuppet();
    pending[1]?.resolve(second.puppet);
    await settle();
    expect(second.loaded).toEqual([good.bytes]);
    expect(p.modelName).toBe("good.ugoe");
  });

  it("resizes the drawing buffer when only the device pixel ratio changes", async () => {
    const queries: { media: string; target: EventTarget }[] = [];
    globalThis.matchMedia = (media: string) => {
      const target = new EventTarget();
      queries.push({ media, target });
      return target as unknown as MediaQueryList;
    };
    globalThis.devicePixelRatio = 1;
    try {
      const p = player();
      p.resize(200, 100);
      const attached = p.attach(container);
      await settle();
      const sizes: number[][] = [];
      const { puppet } = fakePuppet();
      puppet.resize = (width, height) => void sizes.push([width, height]);
      pending[0]?.resolve(puppet);
      await attached;
      expect(sizes).toEqual([[200, 100]]);
      expect(queries.map((q) => q.media)).toEqual(["(resolution: 1dppx)"]);
      globalThis.devicePixelRatio = 2;
      queries[0]?.target.dispatchEvent(new Event("change"));
      expect(sizes).toEqual([
        [200, 100],
        [400, 200],
      ]);
      expect(queries.map((q) => q.media)).toEqual(["(resolution: 1dppx)", "(resolution: 2dppx)"]);
      // detach したら聞かない。
      p.detach();
      globalThis.devicePixelRatio = 1;
      queries[1]?.target.dispatchEvent(new Event("change"));
      expect(queries).toHaveLength(2);
    } finally {
      globalThis.devicePixelRatio = 1;
      delete (globalThis as { matchMedia?: unknown }).matchMedia;
    }
  });
});
