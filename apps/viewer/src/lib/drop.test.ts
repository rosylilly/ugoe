import type { DragDropEvent } from "@tauri-apps/api/webview";
import { setLocale } from "@ugoe/i18n";
import {
  defaultLighting,
  dressUpMemory,
  lightingMemory,
  type ModelFile,
  type ModuleToOpen,
  modelIdentity,
  planRestore,
} from "@ugoe/player";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import {
  classifyDragType,
  describeDropPaths,
  describeDropTypes,
  isOpenableName,
  openDroppedItems,
  skippedMessage,
} from "./drop";
import { type FileDropEvent, readFileAt, viewerHost } from "./host";

// 文言は日本語の訳で確かめる。
beforeEach(() => setLocale("ja"));

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

/** Tauri のドロップを送り、App.svelte と同じくパスから読んで開く。`notice` は飛ばしたものの知らせ。 */
async function dropPaths(
  paths: string[],
  target: Target,
): Promise<{ events: FileDropEvent[]; notice: string | null }> {
  const events: FileDropEvent[] = [];
  let pending: Promise<string | null> = Promise.resolve(null);
  const stop = await viewerHost().listenFileDrop?.((event) => {
    events.push(event);
    if (event.type === "drop")
      pending = openDroppedItems(
        event.paths.map((p) => ({ name: p, read: () => readFileAt(`path:${p}`) })),
        target,
      );
  });
  const position = { x: 0, y: 0 } as never;
  dragDrop?.({ payload: { type: "enter", paths, position } });
  dragDrop?.({ payload: { type: "over", position } });
  dragDrop?.({ payload: { type: "drop", paths, position } });
  const notice = await pending;
  stop?.();
  return { events, notice };
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
    const { events, notice } = await dropPaths([HAIR, MODEL], target);
    expect(events).toEqual([
      { type: "hover", active: true, paths: [HAIR, MODEL] },
      { type: "drop", paths: [HAIR, MODEL] },
    ]);
    expect(notice).toBeNull();
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

  it("opens only the readable files and tells what was skipped, in Japanese (T0717)", async () => {
    const target = new Target();
    const folder = "/home/u/mods";
    const image = "/home/u/Pictures/face.PNG";
    // 拡張子は開けるが読めない (フォルダ・消えたファイル。read_model_file のエラーは英語)。
    const gone = "/home/u/old.ugoe";
    const { notice } = await dropPaths([folder, MODEL, image, gone, HAIR], target);
    expect(target.calls).toHaveLength(1);
    expect(target.calls[0]?.file.location).toBe(`path:${MODEL}`);
    expect(target.calls[0]?.modules.map((m) => m.file.location)).toEqual([`path:${HAIR}`]);
    expect(notice).toBe(
      "開けないものを 3 個飛ばしました: mods、face.PNG (開けるのはモデル .ugoe・.json と着せ替えのモジュール .ugoemod。" +
        "フォルダの中は開きません)。old.ugoe (読めませんでした)",
    );
  });

  it("opens nothing when nothing can be opened", async () => {
    const target = new Target();
    const { notice } = await dropPaths(["/a/b.txt", "/a/folder"], target);
    expect(target.calls).toEqual([]);
    expect(notice).toMatch(/^開けるファイルがありませんでした: b\.txt、folder \(/);
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

describe("drop checks (T0717)", () => {
  it("decides by the extension, like read_model_file", () => {
    expect(isOpenableName("/a/model.UGOE")).toBe(true);
    expect(isOpenableName("C:\\m\\hair.ugoemod")).toBe(true);
    expect(isOpenableName("model.json")).toBe(true);
    expect(isOpenableName("/a/folder")).toBe(false);
    expect(isOpenableName("/a/.json")).toBe(false);
    expect(isOpenableName("project.ugoeproj")).toBe(false);
    expect(isOpenableName("face.psd")).toBe(false);
  });

  it("previews the dragged paths", () => {
    expect(describeDropPaths([MODEL, HAIR])).toEqual({ kind: "all", message: "2 個を開きます" });
    expect(describeDropPaths([MODEL, "/a/folder"])).toEqual({
      kind: "some",
      message: "1 個を開きます (1 個は開けないので飛ばします)",
    });
    expect(describeDropPaths(["/a/folder"]).kind).toBe("none");
  });

  it("speaks the current language", () => {
    setLocale("en");
    expect(describeDropPaths([MODEL])).toEqual({ kind: "all", message: "Opens 1 file" });
    expect(describeDropPaths([MODEL, HAIR, "/a/folder"]).message).toBe(
      "Opens 2 files (1 cannot be opened and will be skipped)",
    );
    expect(
      skippedMessage(
        ["a.psd", "b.psd", "c.psd", "d.psd"].map((name) => ({
          name,
          reason: "extension" as const,
        })),
        0,
      ),
    ).toMatch(/^No files could be opened: a\.psd, b\.psd, and c\.psd and 1 more \(only models/);
  });

  it("classifies the dragged types (T0718)", () => {
    // .ugoe・.ugoemod・フォルダはブラウザでは種類が空。
    expect(classifyDragType("")).toBe("unknown");
    expect(classifyDragType("application/octet-stream")).toBe("unknown");
    expect(classifyDragType("application/json")).toBe("open");
    expect(classifyDragType("application/ld+json")).toBe("open");
    expect(classifyDragType("image/png")).toBe("skip");
    expect(classifyDragType("text/plain")).toBe("skip");
    expect(classifyDragType("application/pdf")).toBe("skip");
  });

  it("previews the dragged types in the browser (T0718)", () => {
    const file = (type: string) => ({ kind: "file", type });
    // 種類の分かるものだけ。
    expect(describeDropTypes([file("application/json")])).toEqual({
      kind: "all",
      message: "1 個を開きます",
    });
    expect(describeDropTypes([file("application/json"), file("image/png")])).toEqual({
      kind: "some",
      message: "1 個を開きます (1 個は開けないので飛ばします)",
    });
    expect(describeDropTypes([file("image/png"), file("text/plain")])?.kind).toBe("none");
    // 種類の分からないもの (.ugoe など) があれば、落としたときに確かめる。
    expect(describeDropTypes([file(""), file("application/json")])).toEqual({
      kind: "unknown",
      message: "2 個のファイル (2 個は、開けるかを落としたときに名前で確かめます)",
    });
    expect(describeDropTypes([file(""), file("image/png")])).toEqual({
      kind: "some",
      message:
        "1 個は開けない種類なので飛ばします (1 個は、開けるかを落としたときに名前で確かめます)",
    });
    // ファイルでないもの (一緒に来る URL) は数えない。ファイルが分からなければ見立てない。
    expect(
      describeDropTypes([file(""), { kind: "string", type: "text/uri-list" }])?.message,
    ).toMatch(/^1 個のファイル/);
    expect(describeDropTypes([])).toBeNull();
    expect(describeDropTypes([{ kind: "string", type: "text/plain" }])).toBeNull();
  });

  it("shortens a long list of names", () => {
    const skipped = ["a", "b", "c", "d", "e"].map((name) => ({
      name,
      reason: "extension" as const,
    }));
    expect(skippedMessage(skipped, 1)).toMatch(
      /^開けないものを 5 個飛ばしました: a、b、c ほか 2 個 \(/,
    );
    expect(skippedMessage([], 1)).toBeNull();
  });
});
