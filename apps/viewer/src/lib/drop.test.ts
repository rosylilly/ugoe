import type { DragDropEvent } from "@tauri-apps/api/webview";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { dressUpMemory, planRestore } from "./dressUpMemory";
import { openDroppedFiles } from "./drop";
import { type FileDropEvent, readFileAt, viewerHost } from "./host";
import { defaultLighting, lightingMemory } from "./lighting";
import type { ModelFile, ModuleToOpen } from "./modelFile";
import { modelIdentity } from "./modelMemory";

// Tauri のウィンドウを模す: `invoke("read_model_file")` はこの表から読み、ドロップのイベントは `dragDrop` で送る。
const disk = new Map<string, Uint8Array>();
let dragDrop: ((e: { payload: DragDropEvent }) => void) | null = null;
const unlisten = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: vi.fn(async (command: string, args: { path: string }) => {
    const bytes = command === "read_model_file" ? disk.get(args.path) : undefined;
    if (bytes === undefined) throw new Error(`${args.path}: not found`);
    return bytes.slice().buffer;
  }),
}));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: async (handler: typeof dragDrop) => {
      dragDrop = handler;
      return unlisten;
    },
  }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));

class MemoryStorage implements Storage {
  #items = new Map<string, string>();
  get length() {
    return this.#items.size;
  }
  clear() {
    this.#items.clear();
  }
  getItem(key: string) {
    return this.#items.get(key) ?? null;
  }
  key(index: number) {
    return [...this.#items.keys()][index] ?? null;
  }
  removeItem(key: string) {
    this.#items.delete(key);
  }
  setItem(key: string, value: string) {
    this.#items.set(key, value);
  }
}

/** チャンク表に `MODU` だけを持つコンテナ (モジュールと判別される)。 */
function moduleBytes(seed: number): Uint8Array {
  const bytes = new Uint8Array(16 + 24 + 1);
  bytes.set([0x55, 0x47, 0x4f, 0x45]);
  new DataView(bytes.buffer).setUint32(12, 1, true);
  bytes.set([0x4d, 0x4f, 0x44, 0x55], 16);
  bytes[40] = seed;
  return bytes;
}

const MODEL = "/home/u/models/avatar.json";
const HAIR = "/home/u/mods/hair.ugoemod";
const HAT = "/home/u/mods/hat.ugoemod";

class Target {
  calls: { file: ModelFile; modules: readonly ModuleToOpen[] }[] = [];
  async load(file: ModelFile, modules: readonly ModuleToOpen[] = []) {
    this.calls.push({ file, modules });
  }
}

/** Tauri のドロップを送り、App.svelte と同じくパスから読んで開く。 */
async function dropPaths(paths: string[], target: Target): Promise<FileDropEvent[]> {
  const events: FileDropEvent[] = [];
  let pending: Promise<void> = Promise.resolve();
  const stop = await viewerHost().listenFileDrop?.((event) => {
    events.push(event);
    if (event.type === "drop")
      pending = Promise.all(event.paths.map((p) => readFileAt(`path:${p}`))).then((files) =>
        openDroppedFiles(files, target),
      );
  });
  const position = { x: 0, y: 0 } as never;
  dragDrop?.({ payload: { type: "enter", paths, position } });
  dragDrop?.({ payload: { type: "over", position } });
  dragDrop?.({ payload: { type: "drop", paths, position } });
  await pending;
  stop?.();
  return events;
}

beforeAll(() => {
  vi.stubGlobal("window", {});
});
afterAll(() => {
  vi.unstubAllGlobals();
});
beforeEach(() => {
  disk.clear();
  disk.set(MODEL, new TextEncoder().encode('{"name": "avatar"}'));
  disk.set(HAIR, moduleBytes(1));
  disk.set(HAT, moduleBytes(2));
});

describe("dropped files", () => {
  it("receives the paths from Tauri and reads them with a path: location", async () => {
    const target = new Target();
    const events = await dropPaths([HAIR, MODEL], target);
    expect(events).toEqual([
      { type: "hover", active: true },
      { type: "drop", paths: [HAIR, MODEL] },
    ]);
    expect(unlisten).toHaveBeenCalled();
    // モデルをモジュールと一緒に開き、どちらにも場所が付く。
    expect(target.calls).toHaveLength(1);
    const [call] = target.calls;
    expect(call?.file).toMatchObject({ name: "avatar.json", location: `path:${MODEL}` });
    expect(call?.modules.map((m) => [m.file.location, m.attach])).toEqual([[`path:${HAIR}`, true]]);
  });

  it("loads modules one by one when no model is dropped", async () => {
    const target = new Target();
    await dropPaths([HAIR, HAT], target);
    expect(target.calls.map((c) => [c.file.location, c.modules.length])).toEqual([
      [`path:${HAIR}`, 0],
      [`path:${HAT}`, 0],
    ]);
  });

  it("restores the set and the lighting for a dropped model after a restart", async () => {
    const storage = new MemoryStorage();
    const target = new Target();
    await dropPaths([MODEL, HAIR, HAT], target);
    const [call] = target.calls;
    if (call === undefined) throw new Error("not opened");
    // 開いている間に覚える (Player と同じく `modelIdentity` を鍵にする)。
    const model = modelIdentity(call.file);
    dressUpMemory(storage).set(model, {
      modules: call.modules.map((m, i) => ({ name: `M${i}`, file: modelIdentity(m.file) })),
      attached: ["M1", "M0"],
    });
    const lit = defaultLighting();
    lit.enabled = !lit.enabled;
    lightingMemory(storage).set(model, lit);

    // 起動し直す: 手元の中身は無い。モデルの中身が変わっても (書き出し直し) 場所で同じモデルと分かる。
    disk.set(MODEL, new TextEncoder().encode('{"name": "avatar", "v": 2}'));
    const reopened = modelIdentity(await readFileAt(`path:${MODEL}`));
    expect(lightingMemory(storage).get(reopened)).toEqual(lit);
    const set = dressUpMemory(storage).get(reopened);
    expect(set?.attached).toEqual(["M1", "M0"]);
    if (set === null) throw new Error("not remembered");
    const plan = planRestore(set, new Set(), () => undefined);
    expect(plan.map((p) => p.source)).toEqual([
      { kind: "location", location: `path:${HAIR}` },
      { kind: "location", location: `path:${HAT}` },
    ]);
    const hat = await readFileAt(`path:${HAT}`);
    expect(hat.bytes).toEqual(moduleBytes(2));
  });
});
