// Player の WebPuppet の作成と破棄 (attach・detach・GPU を失ったときの作り直し)。wasm と DOM は偽物。
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { ModelFile } from "./modelFile";
import type { Player as PlayerType } from "./player.svelte";
import type { UgoeWasmModule, WebPuppet } from "./wasm";

// 画面の部品と MediaPipe まで読み込むと遅いので、Player が使う関数だけにする。
vi.mock("@ugoe/face-tracking", () => ({ setPuppetTracking: () => {} }));

/** WebPuppet.create が返す約束 (テストが好きな時に解く)。 */
let pending: { canvas: FakeCanvas; resolve: (p: WebPuppet) => void }[] = [];

class FakeCanvas extends EventTarget {
  className = "";
  removed = false;
  remove(): void {
    this.removed = true;
  }
}

const wasm: UgoeWasmModule = {
  WebPuppet: {
    create: (canvas) =>
      new Promise<WebPuppet>((resolve) =>
        pending.push({ canvas: canvas as unknown as FakeCanvas, resolve }),
      ),
  },
};

/** 偽の WebPuppet。呼ばれたメソッドを `calls` に、読んだバイト列を `loaded` に残す。 */
function fakePuppet(options: { failLoad?: boolean } = {}) {
  const calls: string[] = [];
  const loaded: Uint8Array[] = [];
  let hasModel = false;
  const methods: Record<string, (...args: never[]) => unknown> = {
    free: () => calls.push("free"),
    backend: () => "webgl2",
    is_lost: () => false,
    has_model: () => hasModel,
    load_bundle: (bytes: Uint8Array) => {
      if (options.failLoad) throw new Error("broken");
      loaded.push(bytes);
      hasModel = true;
      return [];
    },
    load_warnings_json: () => "[]",
    parameters_json: () => "{}",
    modules_json: () => "[]",
    slots_json: () => "[]",
    avatar_slots_json: () => "null",
    motions_json: () => "[]",
    expressions_json: () => "[]",
    tracking_parameter_ids: () => [],
    attached_modules: () => [],
    active_expressions: () => [],
    motion_events: () => [],
    uses_lighting: () => false,
    canvas_size: () => new Float32Array([100, 100]),
    view_transform: () => new Float32Array([1, 0, 0]),
    zoom: () => 1,
  };
  // ほかのメソッド (resize・set_background など) は何もしない。`then` は無い (約束の値として解くため)。
  const puppet = new Proxy(methods, {
    get: (target, key: string) => target[key] ?? (key === "then" ? undefined : () => undefined),
  }) as unknown as WebPuppet;
  return { puppet, calls, loaded };
}

/** 中身が無いバンドル (形式の判別だけ通る)。 */
function bundle(name: string, tag: number): ModelFile {
  return { name, bytes: new Uint8Array([0x55, 0x47, 0x4f, 0x45, tag]) };
}

const container = { prepend() {} } as unknown as HTMLElement;

let Player: typeof PlayerType;

beforeAll(async () => {
  globalThis.requestAnimationFrame ??= () => 1;
  globalThis.cancelAnimationFrame ??= () => {};
  globalThis.devicePixelRatio ??= 1;
  globalThis.document ??= {
    createElement: () => new FakeCanvas(),
  } as unknown as Document;
  ({ Player } = await import("./player.svelte"));
});

beforeEach(() => {
  pending = [];
});

function player(): PlayerType {
  return new Player({
    loadWasm: async () => wasm,
    readFile: () => Promise.reject(new Error("no files")),
  });
}

/** 待っているマイクロタスクを流す (loadWasm と create の await の後まで)。 */
async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) await Promise.resolve();
}

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
    lost?.dispatchEvent(new Event("webglcontextlost"));
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
    lost?.dispatchEvent(new Event("webglcontextlost"));
    await settle();
    expect(pending).toHaveLength(2);
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
    expect(p.error).toContain("broken.ugoe");
    await p.load({ name: "notes.txt", bytes: new Uint8Array([1, 2, 3]) });

    pending[0]?.canvas.dispatchEvent(new Event("webglcontextlost"));
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
