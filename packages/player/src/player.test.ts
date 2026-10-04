// Player の振る舞い。wasm と DOM は偽物 (WebPuppet は Proxy で、使うメソッドだけを状態付きで真似る)。
// 作成と破棄 (attach・detach・GPU を失ったときの作り直し。T0914) と、着せ替えの組・モーション・ドライバとトラッキング・
// 受け手のモード・光源のドラッグを Player を通して確かめる (T0931)。
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { FollowFrame, FollowRenderStats } from "./follower";
import { t } from "./i18n";
import type { FileLocation, ModelFile } from "./modelFile";
import { modelIdentity } from "./modelMemory";
import type { PlayerHost, Player as PlayerType } from "./player.svelte";
import type { UgoeWasmModule, WebPuppet } from "./wasm";

/** `setPuppetTracking` に渡したトラッキングの入力。 */
const tracked = vi.hoisted(() => [] as unknown[]);

// 画面の部品と MediaPipe まで読み込むと遅いので、Player が使う関数だけにする。
vi.mock("@ugoe/face-tracking", () => ({
  setPuppetTracking: (_puppet: unknown, frame: unknown) => void tracked.push(frame),
}));

/** WebPuppet.create が返す約束 (テストが好きな時に解く)。 */
let pending: { canvas: FakeCanvas; resolve: (p: WebPuppet) => void }[] = [];
/** 次の `requestAnimationFrame` で呼ばれるもの。 */
let nextFrame: FrameRequestCallback | null = null;

class FakeCanvas extends EventTarget {
  className = "";
  removed = false;
  remove(): void {
    this.removed = true;
  }
}

/** localStorage の代わり (テストごとに空にする)。 */
class MemoryStorage {
  #items = new Map<string, string>();
  get length(): number {
    return this.#items.size;
  }
  key(index: number): string | null {
    return [...this.#items.keys()][index] ?? null;
  }
  getItem(key: string): string | null {
    return this.#items.get(key) ?? null;
  }
  setItem(key: string, value: string): void {
    this.#items.set(key, String(value));
  }
  removeItem(key: string): void {
    this.#items.delete(key);
  }
  clear(): void {
    this.#items.clear();
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

interface FakeOptions {
  failLoad?: boolean;
  /** モデルのパラメータ (既定値は省くと 0)。 */
  parameters?: { id: string; default?: number }[];
  /** モデル (バンドル) のモーション。 */
  motions?: { name: string; looping?: boolean; parameters?: string[] }[];
  /** トラッキングが書き込むパラメータ。 */
  trackingIds?: string[];
  /** 受け手の口 (`follow`・`reset_physics`) を持つか (省くと持つ)。 */
  follower?: boolean;
}

/**
 * 偽の WebPuppet。`calls` には `free` を、`loaded` には読んだバイト列を、`log` には着せ替え・モーション・トラッキング・
 * 受け手の呼び出しを残す。`state` はテストが書き換えてよい (`playing` を偽にするとモーションが終わったことになる)。
 * 着せ替えは wasm と同じく、モデルを読み直しても読み込んだモジュールと付けていたものを保つ。
 */
function fakePuppet(options: FakeOptions = {}) {
  const calls: string[] = [];
  const loaded: Uint8Array[] = [];
  const log: string[] = [];
  const parameters = (options.parameters ?? []).map((p) => ({
    id: p.id,
    name: p.id,
    min: -1,
    max: 1,
    default: p.default ?? 0,
  }));
  const state = {
    hasModel: false,
    /** モーションを再生しているか。 */
    playing: false,
    /** `start_motion` が始めるか (偽なら優先度が低くて始まらなかったことにする)。 */
    accept: true,
    values: new Map<string, number>(),
    /** 読み込んだモジュール (名前 → スロット。読み込んだ順)。 */
    modules: new Map<string, string>(),
    attached: [] as string[],
    /** `set_lighting_json` に渡されたもの。 */
    lighting: [] as (string | null | undefined)[],
    /** 次にモデルを読んだとき、付けられずに外れるモジュール (モデルが書き換わって合わなくなった)。 */
    unfit: [] as string[],
  };
  const resetValues = () => {
    for (const p of parameters) state.values.set(p.id, p.default);
  };
  const methods: Record<string, ((...args: never[]) => unknown) | undefined> = {
    free: () => calls.push("free"),
    backend: () => "webgl2",
    is_lost: () => false,
    has_model: () => state.hasModel,
    load_bundle: (bytes: Uint8Array) => {
      if (options.failLoad) throw new Error("broken");
      loaded.push(bytes);
      state.hasModel = true;
      state.playing = false;
      resetValues();
      state.attached = state.attached.filter((n) => !state.unfit.includes(n));
      return [];
    },
    load_warnings_json: () => "[]",
    parameters_json: () => JSON.stringify({ parameters, groups: [] }),
    parameter: (id: string) => state.values.get(id),
    set_parameter: (id: string, value: number) => {
      if (!state.values.has(id)) return false;
      state.values.set(id, value);
      return true;
    },
    reset_parameters: resetValues,
    load_module: (bytes: Uint8Array) => {
      const { name, slot } = JSON.parse(new TextDecoder().decode(bytes.subarray(16)));
      state.modules.set(name, slot);
      log.push(`load ${name}`);
      return name;
    },
    attach_module: (name: string) => {
      const slot = state.modules.get(name);
      if (slot === undefined) throw new Error(`${name} is not loaded`);
      state.attached = state.attached.filter((n) => state.modules.get(n) !== slot);
      state.attached.push(name);
    },
    detach_module: (name: string) => {
      if (!state.attached.includes(name)) throw new Error(`${name} is not attached`);
      state.attached = state.attached.filter((n) => n !== name);
    },
    unload_module: (name: string) => {
      if (!state.modules.delete(name)) throw new Error(`${name} is not loaded`);
      state.attached = state.attached.filter((n) => n !== name);
      log.push(`unload ${name}`);
    },
    attached_modules: () => [...state.attached],
    modules_json: () =>
      JSON.stringify(
        [...state.modules].map(([name, slot]) => ({
          name,
          slot,
          attached: state.attached.includes(name),
        })),
      ),
    slots_json: () => "[]",
    avatar_slots_json: () => "null",
    motions_json: () =>
      JSON.stringify(
        (options.motions ?? []).map((m) => ({
          name: m.name,
          duration: 1,
          looping: m.looping ?? false,
          fade_in: 0,
          fade_out: 0,
          parameters: m.parameters ?? [],
        })),
      ),
    start_motion: (name: string, priority: string) => {
      log.push(`start ${name} ${priority}`);
      if (!state.accept) return false;
      state.playing = true;
      return true;
    },
    stop_motions: () => {
      log.push("stop");
      state.playing = false;
    },
    is_motion_playing: () => state.playing,
    expressions_json: () => "[]",
    tracking_parameter_ids: () => options.trackingIds ?? [],
    clear_tracking: () => log.push("clear_tracking"),
    set_motion_over_tracking: (enabled: boolean) => log.push(`over ${enabled}`),
    set_tracking_kept: (ids: string[]) => log.push(`kept ${ids.join(",")}`),
    active_expressions: () => [],
    motion_events: () => [],
    uses_lighting: () => false,
    set_lighting_json: (json?: string | null) => void state.lighting.push(json),
    canvas_size: () => new Float32Array([100, 100]),
    view_transform: () => new Float32Array([1, 0, 0]),
    zoom: () => 1,
    render: () => log.push("render"),
    follow: (values: Float32Array, dt: number) => log.push(`follow ${[...values]} ${dt}`),
    reset_physics: () => log.push("reset_physics"),
  };
  if (options.follower === false) {
    methods.follow = undefined;
    methods.reset_physics = undefined;
  }
  // ほかのメソッド (resize・set_background など) は何もしない。`then` は無い (約束の値として解くため)。
  const puppet = new Proxy(methods, {
    get: (target, key: string) =>
      key in target ? target[key] : key === "then" ? undefined : () => undefined,
  }) as unknown as WebPuppet;
  return { puppet, calls, loaded, log, state };
}

/** 中身が無いバンドル (形式の判別だけ通る)。 */
function bundle(name: string, tag: number, location?: FileLocation): ModelFile {
  const file: ModelFile = { name, bytes: new Uint8Array([0x55, 0x47, 0x4f, 0x45, tag]) };
  if (location !== undefined) file.location = location;
  return file;
}

/** 着せ替えのモジュール (偽の `load_module` が名前とスロットを読む)。 */
function moduleFile(name: string, slot: string, location?: FileLocation): ModelFile {
  const json = new TextEncoder().encode(JSON.stringify({ name, slot }));
  const bytes = new Uint8Array(16 + json.length);
  bytes.set([0x55, 0x47, 0x4f, 0x45]);
  bytes.set(json, 16);
  const file: ModelFile = { name: `${name}.ugoemod`, bytes };
  if (location !== undefined) file.location = location;
  return file;
}

const container = { prepend() {} } as unknown as HTMLElement;

let Player: typeof PlayerType;

beforeAll(async () => {
  globalThis.requestAnimationFrame = (callback) => {
    nextFrame = callback;
    return 1;
  };
  globalThis.cancelAnimationFrame = () => {
    nextFrame = null;
  };
  globalThis.devicePixelRatio ??= 1;
  globalThis.document ??= {
    createElement: () => new FakeCanvas(),
  } as unknown as Document;
  ({ Player } = await import("./player.svelte"));
});

beforeEach(() => {
  pending = [];
  nextFrame = null;
  tracked.length = 0;
  vi.stubGlobal("localStorage", new MemoryStorage());
});

function player(host: Partial<PlayerHost> = {}): PlayerType {
  return new Player({
    loadWasm: async () => wasm,
    readFile: () => Promise.reject(new Error("no files")),
    ...host,
  });
}

/** 待っているマイクロタスクを流す (loadWasm と create の await の後まで)。 */
async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) await Promise.resolve();
}

/** `attach` して偽の WebPuppet を渡し、描画の準備ができた Player。 */
async function ready(options: FakeOptions = {}, host: Partial<PlayerHost> = {}) {
  const p = player(host);
  const attached = p.attach(container);
  await settle();
  const fake = fakePuppet(options);
  pending.at(-1)?.resolve(fake.puppet);
  await attached;
  return { p, ...fake };
}

/** `requestAnimationFrame` の 1 回分を進める (`now` はミリ秒)。 */
function step(now: number): void {
  const callback = nextFrame;
  nextFrame = null;
  callback?.(now);
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

/** 付けているモジュールの名前と、付けているか。 */
function moduleRows(p: PlayerType): [string, boolean][] {
  return p.modules.map((m) => [m.name, m.attached]);
}

describe("Player dress-up sets", () => {
  const model = bundle("a.ugoe", 1, "path:/a.ugoe");
  const other = bundle("b.ugoe", 2, "path:/b.ugoe");
  const hat = moduleFile("hat", "head", "path:/hat.ugoemod");
  const scarf = moduleFile("scarf", "neck", "path:/scarf.ugoemod");

  /** 前のセッションで `model` に hat を付けていた (localStorage に覚えてある)。 */
  async function rememberHat(): Promise<void> {
    const { p } = await ready();
    await p.load(model, [{ file: hat, attach: true }]);
    expect(moduleRows(p)).toEqual([["hat", true]]);
    expect(p.dressUpRemembered).toBe(true);
    p.detach();
  }

  it("offers the remembered set when the model is opened again and restores it from its location", async () => {
    await rememberHat();
    const reads: FileLocation[] = [];
    const { p } = await ready(
      {},
      {
        readFile: async (location) => {
          reads.push(location);
          if (location === hat.location) return hat;
          throw new Error("missing");
        },
      },
    );
    await p.load(model);
    expect(p.restoreOffer).toEqual({
      modules: [{ name: "hat", file: modelIdentity(hat) }],
      attached: ["hat"],
    });
    expect(p.modules).toEqual([]);
    expect(p.dressUpRemembered).toBe(true);

    await p.restoreDressUp();
    expect(reads).toEqual(["path:/hat.ugoemod"]);
    expect(moduleRows(p)).toEqual([["hat", true]]);
    expect(p.restoreOffer).toBeNull();
    expect(p.dressUpError).toBeNull();
  });

  it("restores without asking when autoRestore is on", async () => {
    await rememberHat();
    const { p } = await ready({}, { readFile: async () => hat });
    p.setAutoRestore(true);
    await p.load(model);
    expect(p.restoreOffer).toBeNull();
    expect(moduleRows(p)).toEqual([["hat", true]]);
  });

  it("keeps the remembered set when a module cannot be found again", async () => {
    const { p: first } = await ready();
    // 場所の無いモジュール (ドロップしたファイル) は、次のセッションでは読み直せない。
    const dropped = moduleFile("ribbon", "head");
    await first.load(model, [{ file: dropped, attach: true }]);
    first.detach();

    const { p } = await ready();
    await p.load(model);
    await p.restoreDressUp();
    expect(p.dressUpError).toBe(
      t("dressUp.restore.noLocation", { module: "ribbon", file: "ribbon.ugoemod" }),
    );
    expect(p.modules).toEqual([]);
    // 覚えている組は変えないので、開き直せばまた尋ねる。
    expect(p.dressUpRemembered).toBe(true);
    await p.load(model);
    expect(p.dressUpRemembered).toBe(true);
    await p.load(other);
    await p.load(model);
    expect(p.restoreOffer?.attached).toEqual(["ribbon"]);
  });

  it("keeps the remembered set when the same model is opened again before it is restored", async () => {
    await rememberHat();
    const { p } = await ready();
    await p.load(model);
    await p.load(model);
    expect(p.restoreOffer?.attached).toEqual(["hat"]);
    expect(p.dressUpRemembered).toBe(true);
    // 今回は戻さないと答えても、着せ替えを操作するまでは覚えている組をそのままにする。
    p.dismissRestore();
    await p.load(model);
    expect(p.dressUpRemembered).toBe(true);
    p.detach();

    const { p: next } = await ready();
    await next.load(model);
    expect(next.restoreOffer?.attached).toEqual(["hat"]);
  });

  it("keeps the modules and asks nothing when the same model is opened again", async () => {
    const { p, log } = await ready();
    await p.load(model, [{ file: hat, attach: true }]);
    await p.load(model);
    expect(moduleRows(p)).toEqual([["hat", true]]);
    expect(p.restoreOffer).toBeNull();
    expect(log).not.toContain("unload hat");
  });

  it("remembers the set again when a module no longer fits the reopened model", async () => {
    const { p, state } = await ready();
    await p.load(model, [
      { file: hat, attach: true },
      { file: scarf, attach: true },
    ]);
    state.unfit = ["scarf"];
    await p.load(model);
    expect(moduleRows(p)).toEqual([
      ["hat", true],
      ["scarf", false],
    ]);
    p.detach();

    const { p: next } = await ready();
    await next.load(model);
    expect(next.restoreOffer?.attached).toEqual(["hat"]);
  });

  it("clears the modules for another model and restores them from this session's files", async () => {
    const { p, log } = await ready();
    await p.load(model, [{ file: moduleFile("hat", "head"), attach: true }]);
    await p.load(other);
    expect(log).toContain("unload hat");
    expect(p.modules).toEqual([]);
    expect(p.restoreOffer).toBeNull();
    expect(p.dressUpRemembered).toBe(false);

    // 戻ると尋ねる。場所の無いモジュールでも、このセッションで読んだ中身から戻せる (readFile は失敗する)。
    await p.load(model);
    expect(p.restoreOffer?.attached).toEqual(["hat"]);
    expect(p.dressUpRemembered).toBe(true);
    await p.restoreDressUp();
    expect(p.dressUpError).toBeNull();
    expect(moduleRows(p)).toEqual([["hat", true]]);
    // other には何も覚えていない。
    await p.load(other);
    expect(p.restoreOffer).toBeNull();
    expect(p.dressUpRemembered).toBe(false);
  });

  it("forgets the offer and remembers the new set when the user changes the dress-up", async () => {
    await rememberHat();
    const { p } = await ready();
    await p.load(model);
    expect(p.restoreOffer).not.toBeNull();
    p.loadModule(scarf);
    expect(p.restoreOffer).toBeNull();
    p.detach();

    const { p: next } = await ready();
    await next.load(model);
    expect(next.restoreOffer?.attached).toEqual(["scarf"]);
  });

  it("keeps the shown model's modules and offer when another file fails to open", async () => {
    const { p, puppet } = await ready();
    await p.load(model, [
      { file: hat, attach: true },
      { file: scarf, attach: false },
    ]);
    expect(moduleRows(p)).toEqual([
      ["hat", true],
      ["scarf", false],
    ]);
    const good = puppet.load_bundle;
    puppet.load_bundle = fakePuppet({ failLoad: true }).puppet.load_bundle;
    await p.load(other);
    expect(p.error).toContain("b.ugoe");
    expect(p.modelName).toBe("a.ugoe");
    // 除いたモジュールは戻り、付けていたものも付け直す。どちらのモデルの組も変えない。
    expect(moduleRows(p)).toEqual([
      ["hat", true],
      ["scarf", false],
    ]);
    puppet.load_bundle = good;
    await p.load(other);
    expect(p.restoreOffer).toBeNull();
    await p.load(model);
    expect(p.restoreOffer).toEqual({
      modules: [
        { name: "hat", file: modelIdentity(hat) },
        { name: "scarf", file: modelIdentity(scarf) },
      ],
      attached: ["hat"],
    });

    // 尋ねている間に別のファイルが開けなくても、尋ねている組は残る (表示しているのは model のまま)。
    puppet.load_bundle = fakePuppet({ failLoad: true }).puppet.load_bundle;
    await p.load(other);
    await p.load({ name: "notes.txt", bytes: new Uint8Array([1, 2, 3]) });
    expect(p.restoreOffer?.attached).toEqual(["hat"]);
  });
});

describe("Player motions", () => {
  const model = bundle("a.ugoe", 1, "path:/a.ugoe");
  const motions = [
    { name: "idle", looping: true, parameters: ["Body"] },
    { name: "wave", parameters: ["Arm"] },
  ];
  const parameters = [{ id: "Arm" }, { id: "Body" }];

  it("starts the idle motion on open and goes back to it when another motion ends", async () => {
    const { p, state, log } = await ready({ motions, parameters });
    await p.load(model);
    expect(p.idleMotion).toBe("idle");
    expect(p.currentMotion).toBe("idle");
    expect(log.filter((l) => l.startsWith("start"))).toEqual(["start idle idle"]);

    expect(p.playMotion("wave")).toBe(true);
    step(0);
    expect(p.currentMotion).toBe("wave");
    expect([...p.driven]).toEqual(["Arm"]);

    state.playing = false;
    step(16);
    expect(p.currentMotion).toBe("idle");
    expect(log.filter((l) => l.startsWith("start")).at(-1)).toBe("start idle idle");
    step(32);
    expect([...p.driven]).toEqual(["Body"]);
  });

  it("starts an ended motion again with the same priority when repeating", async () => {
    const { p, state, log } = await ready({ motions, parameters });
    await p.load(model);
    p.setRepeatMotion(true);
    p.playMotion("wave", "force");
    state.playing = false;
    step(0);
    expect(p.currentMotion).toBe("wave");
    expect(log.filter((l) => l.startsWith("start")).slice(-2)).toEqual([
      "start wave force",
      "start wave force",
    ]);

    // 止めたら待機に戻る。待機を止めたら何も流さない。
    p.stopMotions();
    expect(p.currentMotion).toBe("idle");
    p.stopMotions();
    expect(p.currentMotion).toBeNull();
    expect(log.at(-1)).toBe("stop");
  });

  it("tells why a motion did not start and clears it when the playing motion ends", async () => {
    const { p, state } = await ready({ motions, parameters });
    await p.load(model);
    state.accept = false;
    expect(p.playMotion("wave", "idle")).toBe(false);
    expect(p.motionNotice).toBe(
      t("motion.rejected", { name: "wave", priority: t("motion.priority.idle.label") }),
    );
    expect(p.currentMotion).toBe("idle");
    state.playing = false;
    step(0);
    expect(p.motionNotice).toBeNull();
  });

  it("stops the idle motion when auto idle is turned off and remembers the idle choice per model", async () => {
    const { p, log } = await ready({ motions, parameters });
    await p.load(model);
    p.setAutoIdle(false);
    expect(p.currentMotion).toBeNull();
    expect(log.at(-1)).toBe("stop");
    p.setAutoIdle(true);
    expect(p.currentMotion).toBe("idle");

    p.setIdleMotion("wave");
    expect(p.currentMotion).toBe("wave");
    expect(p.idleRemembered).toBe(true);
    p.detach();
    const { p: next } = await ready({ motions, parameters });
    await next.load(model);
    expect(next.idleMotion).toBe("wave");
    expect(next.idleRemembered).toBe(true);
  });
});

describe("Player drivers and tracking", () => {
  const model = bundle("a.ugoe", 1);
  const options: FakeOptions = {
    parameters: [{ id: "Driven" }, { id: "Tracked", default: 0.25 }, { id: "Moved" }],
    trackingIds: ["Tracked"],
    motions: [{ name: "wave", parameters: ["Moved"] }],
  };
  const face = { head: { yaw: 0, pitch: 0, roll: 0 }, blendShapes: {} };

  it("marks the parameters set by drivers, tracking and the playing motion as driven", async () => {
    const { p, state, log } = await ready(options);
    await p.load(model);
    const remove = p.drivers.add({ name: "test", frame: () => [["Driven", 0.5]] });
    step(0);
    expect(state.values.get("Driven")).toBe(0.5);
    expect(p.values.Driven).toBe(0.5);
    expect([...p.driven]).toEqual(["Driven"]);
    // スライダーでは、動かされているパラメータを変えられない。
    p.setParameter("Driven", -1);
    expect(state.values.get("Driven")).toBe(0.5);

    p.setTracking(face);
    step(16);
    step(32);
    // 入力は 1 回だけ渡す (次の入力が来るまで渡し直さない)。
    expect(tracked).toEqual([face]);
    expect([...p.driven].sort()).toEqual(["Driven", "Tracked"]);

    // トラッキング中にモーションを流すと、そのパラメータではモーションを優先する。
    p.playMotion("wave");
    step(48);
    expect([...p.driven].sort()).toEqual(["Driven", "Moved", "Tracked"]);
    expect(log.filter((l) => l.startsWith("over"))).toEqual(["over true"]);

    // トラッキングを止めると、書き込んでいたパラメータを既定値に戻す。
    state.values.set("Tracked", 0.9);
    p.setTracking(null);
    expect(log).toContain("clear_tracking");
    expect(state.values.get("Tracked")).toBe(0.25);
    expect(p.values.Tracked).toBe(0.25);
    step(64);
    expect([...p.driven].sort()).toEqual(["Driven", "Moved"]);
    expect(log.filter((l) => l.startsWith("over"))).toEqual(["over true", "over false"]);

    remove();
    step(80);
    expect([...p.driven]).toEqual(["Moved"]);
    p.setParameter("Driven", -1);
    expect(state.values.get("Driven")).toBe(-1);
  });

  it("passes the kept tracking parameters on the first frame and after a change", async () => {
    const { p, log } = await ready(options);
    await p.load(model);
    step(0);
    step(16);
    p.setTrackingKept(["Tracked", "Tracked"]);
    step(32);
    expect(log.filter((l) => l.startsWith("kept"))).toEqual(["kept ", "kept Tracked"]);
  });
});

/** 受け手のフレーム (値は 1 つ)。 */
function followFrame(seq: number, revision: number, value: number): FollowFrame {
  return {
    seq,
    revision,
    dt: 0.5,
    reset: false,
    values: new Float32Array([value]),
    lipSync: null,
    partOpacityParts: new Uint32Array(),
    partOpacityValues: new Float32Array(),
  };
}

/** 受け手の適用 (と描画) の呼び出しだけ。 */
function followLog(log: string[], render = false): string[] {
  return log.filter(
    (l) => l.startsWith("follow") || l === "reset_physics" || (render && l === "render"),
  );
}

describe("Player follower mode", () => {
  const first = bundle("a.ugoe", 1);
  const second = bundle("b.ugoe", 2);

  it("applies frames that arrived before the model was read in order once the scene is followed", async () => {
    const p = player();
    p.startFollowing();
    expect(p.following).toBe(true);
    // 前のシーンのフレームと、WebPuppet を作っている間に届いたフレーム。
    p.pushFrame(followFrame(1, 0, 9));
    const attached = p.attach(container);
    await settle();
    p.pushFrame(followFrame(5, 1, 1));
    const fake = fakePuppet();
    pending[0]?.resolve(fake.puppet);
    await attached;
    // モデルを読む前に届いたフレーム。
    p.pushFrame(followFrame(6, 1, 2));
    await p.load(first);
    expect(followLog(fake.log)).toEqual([]);

    p.followScene(1);
    expect(followLog(fake.log)).toEqual(["follow 1 0.5", "follow 2 0.5"]);
    // 同じシーンのフレームは届いたその場で適用する。seq が飛んだら物理をリセットしてから。
    p.pushFrame(followFrame(7, 1, 3));
    p.pushFrame(followFrame(9, 1, 4));
    expect(followLog(fake.log).slice(2)).toEqual(["follow 3 0.5", "reset_physics", "follow 4 0.5"]);

    // まだ読んでいない次のシーンのフレームは、そのモデルを読み終えるまで待たせる。
    p.pushFrame(followFrame(1, 2, 5));
    p.pushFrame(followFrame(2, 2, 6));
    expect(followLog(fake.log)).toHaveLength(5);
    await p.load(second);
    expect(followLog(fake.log)).toHaveLength(5);
    p.followScene(2);
    expect(followLog(fake.log).slice(5)).toEqual(["follow 5 0.5", "follow 6 0.5"]);
  });

  it("applies the waiting frames before drawing when the model was read after the scene was followed", async () => {
    const p = player();
    p.startFollowing();
    // WebPuppet ができる前に読んだモデルは、できたときに読む。
    await p.load(first);
    p.followScene(1);
    p.pushFrame(followFrame(1, 1, 1));
    const attached = p.attach(container);
    await settle();
    const fake = fakePuppet();
    pending[0]?.resolve(fake.puppet);
    await attached;
    expect(fake.loaded).toEqual([first.bytes]);
    expect(followLog(fake.log)).toEqual([]);
    step(0);
    expect(followLog(fake.log, true)).toEqual(["follow 1 0.5", "render"]);
  });

  it("continues the frames of the previous scene when the model is kept", async () => {
    const { p, log } = await ready();
    p.startFollowing();
    await p.load(first);
    p.followScene(1);
    p.pushFrame(followFrame(1, 1, 1));
    p.pushFrame(followFrame(2, 2, 2));
    p.pushFrame(followFrame(1, 1, 9));
    p.continueScene(2);
    // 前のシーンのフレームが後から届いても、モデルを読んだシーン以降なので適用する (seq が戻ったので物理はリセット)。
    expect(followLog(log)).toEqual([
      "follow 1 0.5",
      "follow 2 0.5",
      "reset_physics",
      "follow 9 0.5",
    ]);
  });

  it("reports the drawing load and the frames dropped while waiting", async () => {
    const stats: FollowRenderStats[] = [];
    const p = player();
    p.startFollowing((s) => stats.push(s));
    await p.load(first);
    p.followScene(1);
    // WebPuppet ができるまで待たせすぎて溢れたら古いものから捨て、続きは seq が飛んだものとして物理をリセットしてから適用する。
    for (let seq = 1; seq <= 601; seq++) p.pushFrame(followFrame(seq, 1, seq));
    const attached = p.attach(container);
    await settle();
    const fake = fakePuppet();
    pending[0]?.resolve(fake.puppet);
    await attached;
    step(100);
    const applied = followLog(fake.log, true);
    expect(applied.slice(0, 2)).toEqual(["reset_physics", "follow 2 0.5"]);
    expect(applied.slice(-2)).toEqual(["follow 601 0.5", "render"]);
    expect(applied).toHaveLength(602);
    expect(stats).toHaveLength(1);
    expect(stats[0]?.at).toBe(100);
    expect(stats[0]?.dropped).toBe(1);
    expect(stats[0]?.renderMs).toBeGreaterThanOrEqual(0);
  });

  it("neither uses nor remembers the dress-up set and the lights", async () => {
    const hat = moduleFile("hat", "head", "path:/hat.ugoemod");
    const { p: viewer } = await ready();
    await viewer.load(first, [{ file: hat, attach: true }]);
    viewer.detach();
    const saved = localStorage.getItem("ugoe-viewer.dress-up");
    const lighting = localStorage.getItem("ugoe-viewer.lighting");

    const { p } = await ready({}, { readFile: async () => hat });
    p.startFollowing();
    p.setAutoRestore(true);
    await p.load(first, [{ file: moduleFile("scarf", "neck"), attach: true }]);
    expect(p.restoreOffer).toBeNull();
    expect(moduleRows(p)).toEqual([["scarf", true]]);
    p.updateLighting((s) => {
      s.ambient.intensity = 2;
    });
    expect(p.lighting.ambient.intensity).toBe(2);
    expect(p.lightingRemembered).toBe(false);
    expect(localStorage.getItem("ugoe-viewer.dress-up")).toBe(saved);
    expect(localStorage.getItem("ugoe-viewer.lighting")).toBe(lighting);
  });

  it("reports a missing follower API instead of applying frames", async () => {
    const { p, log } = await ready({ follower: false });
    p.startFollowing();
    await p.load(first);
    p.pushFrame(followFrame(1, 1, 1));
    p.followScene(1);
    expect(p.error).toBe(t("errors.noFollowerApi"));
    expect(followLog(log)).toEqual([]);
  });
});

describe("Player lighting", () => {
  const model = bundle("a.ugoe", 1, "path:/a.ugoe");
  const other = bundle("b.ugoe", 2, "path:/b.ugoe");
  const LAST = "ugoe-viewer.lighting";

  it("draws a dragged light without remembering it until it is committed", async () => {
    const { p, state } = await ready();
    await p.load(model);
    const drawn = state.lighting.length;
    p.updateLighting(
      (s) => {
        s.ambient.intensity = 2;
      },
      { preview: true },
    );
    expect(state.lighting).toHaveLength(drawn + 1);
    expect(p.lighting.ambient.intensity).toBe(2);
    expect(localStorage.getItem(LAST)).toBeNull();
    expect(p.lightingRemembered).toBe(false);

    p.commitLighting();
    expect(JSON.parse(localStorage.getItem(LAST) ?? "null")?.settings?.ambient?.intensity).toBe(2);
    expect(p.lightingRemembered).toBe(true);
  });

  it("remembers a light dragged on the previous model when another model is opened", async () => {
    const { p } = await ready();
    await p.load(model);
    p.updateLighting(
      (s) => {
        s.ambient.intensity = 1.5;
      },
      { preview: true },
    );
    await p.load(other);
    // 覚えていないモデルでは、最後に使った設定を保つ。
    expect(p.lightingRemembered).toBe(false);
    expect(p.lighting.ambient.intensity).toBe(1.5);
    p.updateLighting((s) => {
      s.ambient.intensity = 0.5;
    });
    await p.load(model);
    expect(p.lightingRemembered).toBe(true);
    expect(p.lighting.ambient.intensity).toBe(1.5);
    await p.load(other);
    expect(p.lighting.ambient.intensity).toBe(0.5);
  });

  it("does not remember a dragged light again after it is forgotten", async () => {
    const { p } = await ready();
    await p.load(model);
    p.updateLighting(
      (s) => {
        s.ambient.intensity = 1.5;
      },
      { preview: true },
    );
    p.forgetRemembered(modelIdentity(model));
    await p.load(other);
    await p.load(model);
    expect(p.lightingRemembered).toBe(false);
  });
});
