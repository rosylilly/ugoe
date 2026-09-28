import { describe, expect, it } from "vitest";
import {
  BUNDLE_FILTER,
  joinPath,
  PROJECT_FILTER,
  PSD_FILTER,
  parentDir,
  safeFileName,
  withExtension,
} from "./filePaths";

describe("filters", () => {
  it("list extensions without a leading dot", () => {
    expect(PSD_FILTER.extensions).toEqual(["psd"]);
    expect(PROJECT_FILTER.extensions).toEqual(["ugoeproj"]);
    expect(BUNDLE_FILTER.extensions).toEqual(["ugoe"]);
  });
});

describe("withExtension", () => {
  it("appends the extension only when it is missing", () => {
    expect(withExtension("/home/a/model", "ugoeproj")).toBe("/home/a/model.ugoeproj");
    expect(withExtension("/home/a/model.ugoeproj", "ugoeproj")).toBe("/home/a/model.ugoeproj");
    expect(withExtension("C:\\Users\\a\\Model.UGOE", "ugoe")).toBe("C:\\Users\\a\\Model.UGOE");
    expect(withExtension("/home/a/model.ugoeproj", "ugoe")).toBe("/home/a/model.ugoeproj.ugoe");
  });
});

describe("joinPath", () => {
  it("uses the separator of the directory", () => {
    expect(joinPath("/home/a", "x.ugoe")).toBe("/home/a/x.ugoe");
    expect(joinPath("/home/a/", "x.ugoe")).toBe("/home/a/x.ugoe");
    expect(joinPath("C:\\Users\\a\\Documents", "x.ugoe")).toBe("C:\\Users\\a\\Documents\\x.ugoe");
    expect(joinPath("C:\\", "x.ugoe")).toBe("C:\\x.ugoe");
  });
});

describe("parentDir", () => {
  it("returns the containing directory", () => {
    expect(parentDir("/home/a/model.ugoeproj")).toBe("/home/a");
    expect(parentDir("/home/a/model.ugoeproj/")).toBe("/home/a");
    expect(parentDir("C:\\Users\\a\\x.psd")).toBe("C:\\Users\\a");
  });

  it("keeps the root and rejects bare names", () => {
    expect(parentDir("/x.psd")).toBe("/");
    expect(parentDir("C:\\x.psd")).toBe("C:\\");
    expect(parentDir("x.psd")).toBeNull();
  });
});

describe("safeFileName", () => {
  it("replaces characters that are not allowed in file names", () => {
    expect(safeFileName("a/b:c*?")).toBe("a_b_c__");
    expect(safeFileName("character")).toBe("character");
    expect(safeFileName("  ")).toBe("Untitled");
    expect(safeFileName("..")).toBe("Untitled");
  });
});
