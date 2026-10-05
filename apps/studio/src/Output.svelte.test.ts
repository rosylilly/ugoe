// @vitest-environment happy-dom
// 受け手 (Output.svelte) が今のシーンのファイルを読めなかったことを返すか (T0825、docs/studio.md#受け手が読めなかったとき) と、
// シーンの通知を順に処理して最後のものだけを当てるか (T0826、docs/studio.md#シーンの切り替えと受け手)。
// Player は偽物に置き換え、読めるかどうかはバイト列の先頭 (0 なら読めない) で決める。
import type { ModelFile } from "@ugoe/player";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { decodeImage } from "./lib/decodeImage";
import type { OutputFeed } from "./lib/feed";
import type { LoadFailure, StreamMessage, StudioScene } from "./lib/host";
import Output from "./Output.svelte";

const players = vi.hoisted(() => [] as unknown[]);

vi.mock("@ugoe/player", async (importOriginal) => {
  const original = await importOriginal<typeof import("@ugoe/player")>();
  class FakePlayer {
    status = "empty";
    error: string | null = null;
    modules: { name: string }[] = [];
    canvasSize = null;
    loaded: string[] = [];
    followed: number[] = [];
    continued: number[] = [];
    lightings = 0;
    /** 適用した順 (`follow:<版>`・`continue:<版>`・`pose:<パーツ>`)。 */
    calls: string[] = [];
    constructor() {
      players.push(this);
    }
    startFollowing() {}
    async attach() {}
    setBackground() {}
    pushFrame() {}
    async load(file: ModelFile) {
      if (file.bytes[0] === 0) {
        this.error = `${file.name}: broken`;
        return;
      }
      this.error = null;
      this.status = "ready";
      this.loaded.push(file.name);
    }
    loadModule(file: ModelFile) {
      if (file.bytes[0] === 0) return null;
      this.modules.push({ name: file.name });
      return file.name;
    }
    unloadModule(name: string) {
      this.modules = this.modules.filter((m) => m.name !== name);
      return true;
    }
    followScene(revision: number) {
      this.followed.push(revision);
      this.calls.push(`follow:${revision}`);
    }
    continueScene(revision: number) {
      this.continued.push(revision);
      this.calls.push(`continue:${revision}`);
    }
    showPose(part: string) {
      this.calls.push(`pose:${part}`);
      return true;
    }
    updateLighting() {
      this.lightings++;
    }
    resize() {}
    fit() {}
    zoomBy() {}
    panBy() {}
    detach() {}
  }
  return { ...original, Player: FakePlayer };
});

vi.mock("./lib/wasm/load", () => ({ loadWasm: async () => null }));
// 背景の画像をデコードできるか (happy-dom はデコードしない)。既定はできる。
vi.mock("./lib/decodeImage", () => ({ decodeImage: vi.fn(async () => true) }));

interface Fake {
  status: string;
  loaded: string[];
  followed: number[];
  continued: number[];
  lightings: number;
  modules: { name: string }[];
  calls: string[];
}

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
  players.length = 0;
});

function scene(revision: number, patch: Partial<StudioScene> = {}): StudioScene {
  return {
    kind: "scene",
    revision,
    modelRevision: revision,
    modulesRevision: revision,
    modelName: "avatar.ugoe",
    parameterIds: [],
    modules: [],
    lighting: null,
    transform: { x: 0, y: 0, scale: 1, rotation: 0 },
    background: { kind: "transparent" },
    pose: [],
    ...patch,
  } as StudioScene;
}

const GOOD = new Uint8Array([1]);
const BROKEN = new Uint8Array([0]);

/** 偽の配信の口。ファイルは `files` から返し (無ければ取れない)、報告を `failures` に溜める。 */
function receiver(files: {
  model?: (revision: number) => Promise<Uint8Array>;
  module?: (revision: number, index: number) => Promise<Uint8Array>;
  background?: (revision: number) => Promise<Uint8Array>;
}) {
  const failures: LoadFailure[] = [];
  let send: ((message: StreamMessage) => void) | null = null;
  const missing = () => Promise.reject(new Error("not the current scene"));
  const host: OutputFeed = {
    async subscribeFrames(onMessage) {
      send = onMessage;
    },
    sceneModel: files.model ?? missing,
    sceneModule: files.module ?? missing,
    sceneBackground: files.background ?? missing,
    async reportRenderStats() {},
    async reportLoadFailure(failure) {
      failures.push(failure);
    },
  };
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(Output, { target, props: { feed: async () => host } });
  let mounted = true;
  const close = () => {
    if (!mounted) return;
    mounted = false;
    unmount(instance);
    target.remove();
  };
  cleanups.push(close);
  flushSync();
  const player = players.at(-1) as Fake;
  return {
    failures,
    player,
    /** 出力の要素 (背景を描く)。 */
    output: () => target.querySelector<HTMLDivElement>(".output"),
    /** 購読が始まるのを待ってからシーンを送る。 */
    async send(message: StudioScene) {
      await vi.waitFor(() => expect(send).not.toBeNull());
      await settle();
      send?.(message);
    },
    /** 前のシーンの処理を待たずに、すぐ送る (購読が始まってから使う)。 */
    post(message: StudioScene) {
      send?.(message);
    },
    close,
  };
}

/** 受け手のシーンの処理 (Promise の連なり) を進める。 */
async function settle() {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
}

describe("Output load failures", () => {
  it("reports a model that the current scene cannot get or read", async () => {
    const r = receiver({ model: async (revision) => (revision === 1 ? GOOD : BROKEN) });
    await r.send(scene(1, { modelRevision: 1 }));
    await vi.waitFor(() => expect(r.player.loaded).toEqual(["avatar.ugoe"]));
    await r.send(scene(2, { modelRevision: 2 }));
    await vi.waitFor(() =>
      expect(r.failures).toEqual([
        { revision: 2, asset: "model", index: null, code: "unreadable" },
      ]),
    );
    // 前のモデルにこのシーンのフレームは当てない。
    expect(r.player.followed).toEqual([1]);
    // 見せ方だけが変わった次のシーンでも、読めなかったモデルを読み直す。
    await r.send(scene(3, { modelRevision: 2 }));
    await vi.waitFor(() => expect(r.failures).toHaveLength(2));
    expect(r.failures[1]).toEqual({ revision: 3, asset: "model", index: null, code: "unreadable" });
  });

  it("reports files it cannot get while the scene is still current", async () => {
    const r = receiver({
      model: async () => GOOD,
      module: async (_revision, index) => (index === 1 ? Promise.reject(new Error("io")) : GOOD),
    });
    await r.send(scene(1, { modules: ["CatEars", "Hat"] }));
    await vi.waitFor(() =>
      expect(r.failures).toEqual([{ revision: 1, asset: "module", index: 1, code: "unavailable" }]),
    );
    // 受け取れないモジュールは飛ばして、モデルと受け取れたモジュールで出す (T0840)。
    expect(r.player.loaded).toEqual(["avatar.ugoe"]);
    expect(r.player.modules.map((m) => m.name)).toEqual(["CatEars.ugoemod"]);
  });

  // 1 つのシーンで読めなかったものは、最初の 1 つでなくすべて知らせる (T0840)。
  it("reports every module and the background it cannot get or read", async () => {
    vi.mocked(decodeImage).mockResolvedValueOnce(false);
    const r = receiver({
      model: async () => GOOD,
      module: async (_revision, index) =>
        index === 0 ? BROKEN : index === 1 ? Promise.reject(new Error("io")) : GOOD,
      background: async () => GOOD,
    });
    await r.send(
      scene(1, {
        modules: ["CatEars", "Hat", "Ribbon"],
        background: { kind: "image", path: "/bg.png" },
      }),
    );
    await vi.waitFor(() => expect(r.failures).toHaveLength(3));
    expect(r.failures).toEqual([
      { revision: 1, asset: "module", index: 0, code: "unreadable" },
      { revision: 1, asset: "module", index: 1, code: "unavailable" },
      { revision: 1, asset: "background", index: null, code: "unreadable" },
    ]);
    expect(r.player.followed).toEqual([1]);
    expect(r.player.modules.map((m) => m.name)).toEqual(["Ribbon.ugoemod"]);
    // 見せ方だけが変わった次のシーンで付け直し、まだ読めなければもう一度すべて知らせる。
    await r.send(
      scene(2, {
        modelRevision: 1,
        modulesRevision: 1,
        modules: ["CatEars", "Hat", "Ribbon"],
      }),
    );
    await vi.waitFor(() => expect(r.failures).toHaveLength(5));
    expect(r.failures.slice(3)).toEqual([
      { revision: 2, asset: "module", index: 0, code: "unreadable" },
      { revision: 2, asset: "module", index: 1, code: "unavailable" },
    ]);
  });

  it("reports only the model when it cannot read the model", async () => {
    const r = receiver({
      model: async () => BROKEN,
      module: async () => Promise.reject(new Error("io")),
      background: async () => Promise.reject(new Error("io")),
    });
    await r.send(
      scene(1, { modules: ["CatEars"], background: { kind: "image", path: "/bg.png" } }),
    );
    await vi.waitFor(() => expect(r.failures).toHaveLength(1));
    await settle();
    expect(r.failures).toEqual([{ revision: 1, asset: "model", index: null, code: "unreadable" }]);
  });

  it("does not report a failure caused by moving to the next scene", async () => {
    let reject: (e: Error) => void = () => {};
    const r = receiver({
      model: (revision) =>
        revision === 1
          ? new Promise<Uint8Array>((_, no) => {
              reject = no;
            })
          : Promise.resolve(GOOD),
      background: () => Promise.reject(new Error("not the current scene")),
    });
    await r.send(scene(1));
    await settle();
    // 取っている間に次のシーンが届き、前のシーンのファイルは取れなくなる。
    await r.send(scene(2));
    reject(new Error("scene 1 is not the current scene"));
    await vi.waitFor(() => expect(r.player.followed).toEqual([2]));
    await settle();
    expect(r.failures).toEqual([]);
  });

  it("reports modules it cannot read and attaches them again on the next scene", async () => {
    const r = receiver({
      model: async () => GOOD,
      module: async (_revision, index) => (index === 0 ? BROKEN : GOOD),
    });
    await r.send(scene(1, { modules: ["CatEars", "Hat"] }));
    await vi.waitFor(() =>
      expect(r.failures).toEqual([{ revision: 1, asset: "module", index: 0, code: "unreadable" }]),
    );
    // モデルと読めたモジュールは出す。
    expect(r.player.followed).toEqual([1]);
    expect(r.player.modules.map((m) => m.name)).toEqual(["Hat.ugoemod"]);
    await r.send(scene(2, { modelRevision: 1, modulesRevision: 1, modules: ["CatEars", "Hat"] }));
    await vi.waitFor(() => expect(r.failures).toHaveLength(2));
    expect(r.failures[1]).toEqual({ revision: 2, asset: "module", index: 0, code: "unreadable" });
  });

  it("reports a background image it cannot get", async () => {
    const r = receiver({});
    await r.send(scene(1, { modelRevision: 0, background: { kind: "image", path: "/bg.png" } }));
    await vi.waitFor(() =>
      expect(r.failures).toEqual([
        { revision: 1, asset: "background", index: null, code: "unavailable" },
      ]),
    );
  });

  // 受け取れても描けない (壊れた) 画像も知らせ、出さない。次のシーンの通知で取り直す (T0836)。
  it("reports a background image it cannot decode and fetches it again on the next scene", async () => {
    const created: string[] = [];
    const create = vi.spyOn(URL, "createObjectURL").mockImplementation(() => {
      created.push(`blob:${created.length + 1}`);
      return created.at(-1) as string;
    });
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    cleanups.push(() => {
      create.mockRestore();
      revoke.mockRestore();
    });
    vi.mocked(decodeImage).mockResolvedValueOnce(false);
    const background = vi.fn(async () => GOOD);
    const r = receiver({ background });
    const look = { modelRevision: 0, background: { kind: "image" as const, path: "/bg.png" } };
    await r.send(scene(1, look));
    await vi.waitFor(() =>
      expect(r.failures).toEqual([
        { revision: 1, asset: "background", index: null, code: "unreadable" },
      ]),
    );
    expect(r.output()?.style.background ?? "").not.toContain("blob:1");
    expect(revoke.mock.calls).toEqual([["blob:1"]]);
    // 次のシーンの通知 (同じ画像) で取り直し、描けたら出す。
    await r.send(scene(2, look));
    await vi.waitFor(() => expect(r.output()?.style.background).toContain("blob:2"));
    expect(background).toHaveBeenCalledTimes(2);
    expect(r.failures).toHaveLength(1);
  });

  it("reports that it cannot render without fetching the model", async () => {
    const model = vi.fn(async () => GOOD);
    const r = receiver({ model });
    r.player.status = "unavailable";
    await r.send(scene(1));
    await vi.waitFor(() =>
      expect(r.failures).toEqual([
        { revision: 1, asset: "model", index: null, code: "noRenderer" },
      ]),
    );
    expect(model).not.toHaveBeenCalled();
  });
});

/** 解決を外から決める Promise。 */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

describe("Output scene changes", () => {
  it("applies only the last of scenes that arrive in a row", async () => {
    const first = deferred<Uint8Array>();
    const model = vi.fn((revision: number) =>
      revision === 1 ? first.promise : Promise.resolve(GOOD),
    );
    const r = receiver({ model });
    await r.send(
      scene(1, { modelName: "one.ugoe", background: { kind: "color", color: "#ff0000" } }),
    );
    await settle();
    // 1 つ目のモデルを取っている間に、2 つ目と 3 つ目が続けて届く。
    r.post(scene(2, { modelName: "two.ugoe", background: { kind: "chroma", color: "#00ff00" } }));
    r.post(scene(3, { modelName: "three.ugoe", background: { kind: "transparent" } }));
    first.resolve(GOOD);
    await vi.waitFor(() => expect(r.player.followed).toEqual([3]));
    await settle();
    // 途中のモデルは取りも読みもしない。古いシーンの見せ方も当てない。
    expect(model.mock.calls.map(([revision]) => revision)).toEqual([1, 3]);
    expect(r.player.loaded).toEqual(["three.ugoe"]);
    expect(r.player.lightings).toBe(1);
    expect(r.output()?.dataset.background).toBe("transparent");
  });

  it("drops the look of a scene replaced while its background loads", async () => {
    const image = deferred<Uint8Array>();
    const r = receiver({ model: async () => GOOD, background: () => image.promise });
    await r.send(scene(1));
    await vi.waitFor(() => expect(r.player.lightings).toBe(1));
    r.post(
      scene(2, {
        modelRevision: 1,
        modulesRevision: 1,
        background: { kind: "image", path: "/bg.png" },
      }),
    );
    await settle();
    r.post(
      scene(3, {
        modelRevision: 1,
        modulesRevision: 1,
        background: { kind: "color", color: "#0000ff" },
      }),
    );
    image.resolve(GOOD);
    await vi.waitFor(() => expect(r.player.continued).toEqual([3]));
    expect(r.player.lightings).toBe(2);
    expect(r.output()?.dataset.background).toBe("color");
  });

  it("keeps the model for the same model and attaches modules again when they change", async () => {
    const model = vi.fn(async () => GOOD);
    const module = vi.fn(async () => GOOD);
    const r = receiver({ model, module });
    await r.send(scene(1, { modules: ["CatEars"] }));
    await vi.waitFor(() => expect(r.player.followed).toEqual([1]));
    // 見せ方だけが変わった。
    await r.send(scene(2, { modelRevision: 1, modulesRevision: 1, modules: ["CatEars"] }));
    await vi.waitFor(() => expect(r.player.continued).toEqual([2]));
    expect(model).toHaveBeenCalledTimes(1);
    expect(module).toHaveBeenCalledTimes(1);
    // モジュールの組だけが変わった。
    await r.send(scene(3, { modelRevision: 1, modulesRevision: 3, modules: ["Hat", "Ribbon"] }));
    await vi.waitFor(() => expect(r.player.continued).toEqual([2, 3]));
    expect(model).toHaveBeenCalledTimes(1);
    expect(r.player.loaded).toEqual(["avatar.ugoe"]);
    expect(r.player.followed).toEqual([1]);
    expect(module.mock.calls.slice(1)).toEqual([
      [3, 0],
      [3, 1],
    ]);
    expect(r.player.modules.map((m) => m.name)).toEqual(["Hat.ugoemod", "Ribbon.ugoemod"]);
  });

  it("revokes the previous blob URL when the background image changes", async () => {
    let n = 0;
    const create = vi.spyOn(URL, "createObjectURL").mockImplementation(() => `blob:${++n}`);
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    cleanups.push(() => {
      create.mockRestore();
      revoke.mockRestore();
    });
    const background = vi.fn(async () => GOOD);
    const r = receiver({ background });
    const image = (path: string) => ({ kind: "image" as const, path });
    await r.send(scene(1, { modelRevision: 0, background: image("/a.png") }));
    await vi.waitFor(() => expect(r.output()?.style.background).toContain("blob:1"));
    expect(revoke).not.toHaveBeenCalled();
    await r.send(scene(2, { modelRevision: 0, background: image("/b.png") }));
    await vi.waitFor(() => expect(r.output()?.style.background).toContain("blob:2"));
    expect(revoke.mock.calls).toEqual([["blob:1"]]);
    // 同じ画像なら取り直さない。
    await r.send(scene(3, { modelRevision: 0, background: image("/b.png") }));
    await settle();
    expect(background).toHaveBeenCalledTimes(2);
    expect(create).toHaveBeenCalledTimes(2);
    // 閉じたら今の画像も解放する。
    r.close();
    expect(revoke.mock.calls).toEqual([["blob:1"], ["blob:2"]]);
  });
});

// ポーズの要素は、読み直し・付け直しの後、そのシーンのフレームを当てる前に表示させる (T0842)。
describe("Output pose", () => {
  it("shows the scene's pose parts before following the scene", async () => {
    const r = receiver({ model: async () => GOOD, module: async () => GOOD });
    await r.send(scene(1, { pose: ["ArmB", "Hat/Off"] }));
    await vi.waitFor(() => expect(r.player.followed).toEqual([1]));
    expect(r.player.calls).toEqual(["pose:ArmB", "pose:Hat/Off", "follow:1"]);
    // 見せ方だけが変わったシーン (ポーズを選び直した) でも当て直す。
    await r.send(scene(2, { modelRevision: 1, modulesRevision: 1, pose: ["ArmA"] }));
    await vi.waitFor(() => expect(r.player.continued).toEqual([2]));
    expect(r.player.calls.slice(3)).toEqual(["pose:ArmA", "continue:2"]);
    // モジュールを付け直したシーンでも、付けた後に当てる。
    await r.send(scene(3, { modelRevision: 1, modules: ["Hat"], pose: ["ArmA", "Hat/On"] }));
    await vi.waitFor(() => expect(r.player.continued).toEqual([2, 3]));
    expect(r.player.modules.map((m) => m.name)).toEqual(["Hat.ugoemod"]);
    expect(r.player.calls.slice(5)).toEqual(["pose:ArmA", "pose:Hat/On", "continue:3"]);
  });
});
