import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { type FileDialogs, fileDialogs } from "./fileDialogs";
import { JSON_FILTER, PNG_FILTER, PROJECT_FILTER } from "./filePaths";

vi.mock("./ipc", () => ({ ipc: {} }));

let answer: string | null;
let stub: FileDialogs;

beforeEach(() => {
  answer = null;
  stub = {
    openFile: vi.fn(async () => answer),
    openDirectory: vi.fn(async () => answer),
    saveFile: vi.fn(async () => answer),
    defaultDirectory: vi.fn(async () => "/home/a"),
  };
  vi.stubGlobal("window", { __UGOE_FILE_DIALOGS__: stub });
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("fileDialogs: the browser stub", () => {
  it("appends the first extension of the first filter to a saved name without it", async () => {
    answer = "/home/a/model";
    const path = await fileDialogs().saveFile({
      title: "Save",
      filters: [PROJECT_FILTER, JSON_FILTER],
    });
    expect(path).toBe("/home/a/model.ugoeproj");
  });

  it("keeps a saved name that already has the extension, ignoring case", async () => {
    answer = "C:\\Users\\a\\Shot.PNG";
    expect(await fileDialogs().saveFile({ title: "Save", filters: [PNG_FILTER] })).toBe(
      "C:\\Users\\a\\Shot.PNG",
    );
  });

  it("passes the options through and leaves cancelled or unfiltered answers alone", async () => {
    const options = { title: "Save", defaultPath: "/home/a/x.json", filters: [JSON_FILTER] };
    expect(await fileDialogs().saveFile(options)).toBeNull();
    expect(stub.saveFile).toHaveBeenCalledWith(options);
    answer = "/home/a/notes";
    expect(await fileDialogs().saveFile({ title: "Save" })).toBe("/home/a/notes");
  });

  it("does not touch opened paths", async () => {
    answer = "/home/a/model";
    const dialogs = fileDialogs();
    expect(await dialogs.openFile({ title: "Open", filters: [PROJECT_FILTER] })).toBe(
      "/home/a/model",
    );
    expect(await dialogs.openDirectory({ title: "Open" })).toBe("/home/a/model");
    expect(await dialogs.defaultDirectory()).toBe("/home/a");
  });
});
