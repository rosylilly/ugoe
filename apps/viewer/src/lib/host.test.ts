import { setLocale } from "@ugoe/i18n";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { CommandError } from "./errors";
import { readFileAt, saveJsonFile, savePngFile, viewerHost } from "./host";

// Tauri のウィンドウを模す: ダイアログのコマンドは `picked` を返し、`invoke` は呼ばれた引数を覚える。
// ダイアログはバックエンドが開く (選んだパスだけを読み書きできるようにするため。T0727)。
let picked: string | string[] | null = null;
/** 次のコマンドを失敗させる値 (バックエンドが reject する `code`・`args`・`message`)。 */
let failure: unknown = null;
/** バックエンドの `initial_files` が返す、OS から起動時に渡されたパス。 */
let initialPaths: string[] = [];
const invoke = vi.fn(async (command: string, _args?: unknown, _options?: unknown) => {
  if (failure !== null && command !== "save_file_dialog" && !command.startsWith("listen:"))
    throw failure;
  if (command === "initial_files") return initialPaths;
  if (command === "save_file_dialog" || command === "open_file_dialog") return picked;
  if (command === "read_model_file") return new Uint8Array([7]).buffer;
  return undefined;
});

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: (...args: [string, unknown?, unknown?]) => invoke(...args),
}));
// イベントの待ち受け (`listen`)。待ち受けたことを `invoke` と同じ列に `listen:<名前>` として残す (順序を確かめる)。
const listeners = new Map<string, (e: { payload: unknown }) => void>();
const unlisten = vi.fn();
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, handler: (e: { payload: unknown }) => void) => {
    invoke(`listen:${name}`);
    listeners.set(name, handler);
    return unlisten;
  },
}));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({}) }));
vi.mock("@tauri-apps/plugin-os", () => ({ locale: async () => null }));

beforeAll(() => {
  vi.stubGlobal("window", {});
});
beforeEach(() => {
  invoke.mockClear();
  unlisten.mockClear();
  listeners.clear();
  initialPaths = [];
  picked = null;
  failure = null;
  setLocale("ja");
});

describe("saving in Tauri", () => {
  it("asks the backend for the path and writes the JSON there", async () => {
    // 拡張子が無ければバックエンドが付けて返す (Linux のダイアログはフィルタの拡張子を付けずに返すことがある)。
    picked = "/home/u/rec.json";
    expect(await saveJsonFile("face-tracking.json", "{}")).toBe(true);
    expect(invoke).toHaveBeenNthCalledWith(1, "save_file_dialog", {
      title: expect.any(String),
      defaultName: "face-tracking.json",
      filterName: "JSON ファイル",
      extension: "json",
    });
    expect(invoke).toHaveBeenLastCalledWith("write_json_file", {
      path: "/home/u/rec.json",
      contents: "{}",
    });
  });

  it("writes a screenshot to the path the backend returned", async () => {
    picked = "/home/u/shot.png";
    expect(await savePngFile("a.png", new Uint8Array([1]))).toBe("/home/u/shot.png");
    expect(invoke).toHaveBeenNthCalledWith(
      1,
      "save_file_dialog",
      expect.objectContaining({ defaultName: "a.png", extension: "png" }),
    );
    expect(invoke).toHaveBeenLastCalledWith("write_png_file", new Uint8Array([1]), {
      headers: { "x-ugoe-path": encodeURIComponent("/home/u/shot.png") },
    });
  });

  it("does nothing when the dialog is cancelled", async () => {
    expect(await saveJsonFile("face-tracking.json", "{}")).toBe(false);
    expect(await savePngFile("a.png", new Uint8Array([1]))).toBeNull();
    expect(invoke.mock.calls.map(([command]) => command)).toEqual([
      "save_file_dialog",
      "save_file_dialog",
    ]);
  });
});

describe("opening in Tauri", () => {
  it("reads the file the backend dialog picked", async () => {
    picked = ["/home/u/model.ugoe"];
    const file = await viewerHost().pickModelFile();
    expect(file).toMatchObject({ name: "model.ugoe", location: "path:/home/u/model.ugoe" });
    expect(invoke).toHaveBeenNthCalledWith(
      1,
      "open_file_dialog",
      expect.objectContaining({ multiple: false, extensions: ["ugoe", "json", "ugoemod"] }),
    );
    expect(invoke).toHaveBeenLastCalledWith("read_model_file", { path: "/home/u/model.ugoe" });
  });

  it("reads every module and nothing when cancelled", async () => {
    picked = ["/m/a.ugoemod", "/m/b.ugoemod"];
    expect((await viewerHost().pickModuleFiles()).map((f) => f.name)).toEqual([
      "a.ugoemod",
      "b.ugoemod",
    ]);
    picked = [];
    invoke.mockClear();
    expect(await viewerHost().pickModelFile()).toBeNull();
    expect(await viewerHost().pickModuleFiles()).toEqual([]);
    expect(invoke.mock.calls.map(([command]) => command)).toEqual([
      "open_file_dialog",
      "open_file_dialog",
    ]);
  });
});

describe("command errors (T0729)", () => {
  it("throws the error of the backend translated into the current language", async () => {
    failure = {
      code: "file.too_large",
      args: { path: "/m/big.ugoe", size: 600000000, max: 536870912 },
      message: "/m/big.ugoe: file is too large (600000000 bytes; the limit is 536870912 bytes)",
    };
    const error = await readFileAt("path:/m/big.ugoe").catch((e: unknown) => e);
    expect(error).toBeInstanceOf(CommandError);
    expect(error).toMatchObject({
      code: "file.too_large",
      args: { path: "/m/big.ugoe" },
      message: "/m/big.ugoe は大きすぎて開けません (600,000,000 バイト。上限は 536,870,912 バイト)",
    });
  });

  it("throws the error of a failed save", async () => {
    setLocale("en");
    picked = "/home/u/rec.json";
    failure = {
      code: "file.write_failed",
      args: { path: "/home/u/rec.json", reason: "Permission denied (os error 13)" },
      message: "/home/u/rec.json: Permission denied (os error 13)",
    };
    await expect(saveJsonFile("rec.json", "{}")).rejects.toThrow(
      "Cannot write /home/u/rec.json (Permission denied (os error 13))",
    );
  });

  it("passes other failures through", async () => {
    failure = new Error("IPC is gone");
    await expect(readFileAt("path:/m/a.ugoe")).rejects.toThrow("IPC is gone");
  });
});

describe("files opened from the OS in Tauri (T0732)", () => {
  it("listens before taking the files given at startup, then receives the ones opened later", async () => {
    initialPaths = ["/home/u/model.ugoe", "/home/u/hair.ugoemod"];
    const received: string[][] = [];
    const stop = await viewerHost().listenOpenedFiles?.((paths) => received.push(paths));
    // 起動中に届いたもの (macOS の `Opened`) を取りこぼさないよう、待ち受けてから起動時のものを受け取る。
    expect(invoke.mock.calls.map(([command]) => command)).toEqual([
      "listen:open-files",
      "initial_files",
    ]);
    expect(received).toEqual([["/home/u/model.ugoe", "/home/u/hair.ugoemod"]]);
    listeners.get("open-files")?.({ payload: ["/home/u/other.ugoe"] });
    expect(received).toEqual([
      ["/home/u/model.ugoe", "/home/u/hair.ugoemod"],
      ["/home/u/other.ugoe"],
    ]);
    stop?.();
    expect(unlisten).toHaveBeenCalled();
  });

  it("does not call the listener when nothing was given at startup", async () => {
    const received: string[][] = [];
    await viewerHost().listenOpenedFiles?.((paths) => received.push(paths));
    expect(received).toEqual([]);
  });

  it("stops listening when the files given at startup cannot be taken", async () => {
    failure = { code: "internal", args: {}, message: "boom" };
    await expect(viewerHost().listenOpenedFiles?.(() => {})).rejects.toBeInstanceOf(CommandError);
    expect(unlisten).toHaveBeenCalled();
  });
});
