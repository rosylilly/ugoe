import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { saveJsonFile, savePngFile, withExtension } from "./host";

// Tauri のウィンドウを模す: 保存のダイアログは `picked` を返し、`invoke` は呼ばれた引数を覚える。
let picked: string | null = null;
const invoke = vi.fn(async () => undefined);

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: (...args: unknown[]) => invoke(...(args as [])),
}));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: async () => picked }));
vi.mock("@tauri-apps/plugin-os", () => ({ locale: async () => null }));

beforeAll(() => {
  vi.stubGlobal("window", {});
});
beforeEach(() => {
  invoke.mockClear();
  picked = null;
});

describe("saving in Tauri", () => {
  it("adds .json when the save dialog drops the extension", async () => {
    // Linux の保存のダイアログは、フィルタの拡張子を付けずにパスを返すことがある。
    // バックエンドの `write_json_file` は `.json` のパスしか書かない。
    picked = "/home/u/rec";
    expect(await saveJsonFile("face-tracking.json", "{}")).toBe(true);
    expect(invoke).toHaveBeenCalledWith("write_json_file", {
      path: "/home/u/rec.json",
      contents: "{}",
    });
    picked = "C:\\u\\rec.JSON";
    await saveJsonFile("face-tracking.json", "{}");
    expect(invoke).toHaveBeenLastCalledWith("write_json_file", {
      path: "C:\\u\\rec.JSON",
      contents: "{}",
    });
  });

  it("adds .png to the picked path of a screenshot", async () => {
    picked = "/home/u/shot";
    expect(await savePngFile("a.png", new Uint8Array([1]))).toBe("/home/u/shot.png");
    expect(invoke).toHaveBeenCalledWith("write_png_file", new Uint8Array([1]), {
      headers: { "x-ugoe-path": encodeURIComponent("/home/u/shot.png") },
    });
  });

  it("does nothing when the dialog is cancelled", async () => {
    expect(await saveJsonFile("face-tracking.json", "{}")).toBe(false);
    expect(await savePngFile("a.png", new Uint8Array([1]))).toBeNull();
    expect(invoke).not.toHaveBeenCalled();
  });
});

describe("withExtension", () => {
  it("adds the extension only when it is missing", () => {
    expect(withExtension("/a/shot", "png")).toBe("/a/shot.png");
    expect(withExtension("C:\\a\\shot.PNG", "png")).toBe("C:\\a\\shot.PNG");
    expect(withExtension("/a/shot.png.txt", "png")).toBe("/a/shot.png.txt.png");
    expect(withExtension("/a/rec", "json")).toBe("/a/rec.json");
  });
});
