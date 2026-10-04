// @vitest-environment happy-dom
// 受け手 (Output.svelte) が今のシーンのファイルを読めなかったことを返すか (T0825、docs/studio.md#受け手が読めなかったとき)。
// Player は偽物に置き換え、読めるかどうかはバイト列の先頭 (0 なら読めない) で決める。
import type { ModelFile } from "@ugoe/player";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
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
    }
    continueScene() {}
    updateLighting() {}
    resize() {}
    fit() {}
    zoomBy() {}
    panBy() {}
    detach() {}
  }
  return { ...original, Player: FakePlayer };
});

vi.mock("./lib/wasm/load", () => ({ loadWasm: async () => null }));

interface Fake {
  status: string;
  loaded: string[];
  followed: number[];
  modules: { name: string }[];
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
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  const player = players.at(-1) as Fake;
  return {
    failures,
    player,
    /** 購読が始まるのを待ってからシーンを送る。 */
    async send(message: StudioScene) {
      await vi.waitFor(() => expect(send).not.toBeNull());
      await settle();
      send?.(message);
    },
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
    expect(r.player.loaded).toEqual([]);
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
