import { describe, expect, it } from "vitest";
import { describeFiles, externalAction, isExternalChangeError } from "./externalChange";
import type { ExternalChange } from "./ipc";

const change: ExternalChange = { files: ["model.json"], missing: false, token: "abc" };
const clean = { modified: false, autoReload: false, dismissed: null };

describe("external change", () => {
  it("does nothing when nothing changed or the change was dismissed", () => {
    expect(externalAction(null, clean)).toBe("none");
    expect(externalAction(change, { ...clean, dismissed: "abc" })).toBe("none");
  });

  it("asks unless it may reload silently", () => {
    expect(externalAction(change, clean)).toBe("ask");
    expect(externalAction(change, { ...clean, dismissed: "other" })).toBe("ask");
    expect(externalAction(change, { ...clean, autoReload: true })).toBe("reload");
    // 未保存の変更があれば、自動で開き直す設定でも尋ねる
    expect(externalAction(change, { ...clean, modified: true, autoReload: true })).toBe("ask");
    // 消されたプロジェクトは開き直せない
    const missing = { files: [], missing: true, token: "missing" };
    expect(externalAction(missing, { ...clean, autoReload: true })).toBe("ask");
  });

  it("lists a few changed files", () => {
    expect(describeFiles(["model.json"])).toBe("model.json");
    expect(describeFiles(["a", "b", "c"])).toBe("a, b, c");
    expect(describeFiles(["a", "b", "c", "d"])).toBe("a, b, c and 1 more file");
    expect(describeFiles(["a", "b", "c", "d", "e"])).toBe("a, b, c and 2 more files");
  });

  it("recognizes the refused save", () => {
    expect(
      isExternalChangeError(
        "the project was changed outside the editor after it was opened; reload it, or save again to overwrite",
      ),
    ).toBe(true);
    expect(isExternalChangeError("disk full")).toBe(false);
  });
});
