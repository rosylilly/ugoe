import { setLocale } from "@ugoe/i18n";
import { afterEach, describe, expect, it } from "vitest";
import { errorCode, errorText, isMessage, messageText } from "./errors";
import type { Message } from "./ipc";

const message: Message = {
  code: "project.not_a_project",
  args: { path: "/work/a.ugoeproj" },
  message: "/work/a.ugoeproj: not a ugoe project (project.json is missing)",
};

afterEach(() => setLocale("en"));

describe("backend errors", () => {
  it("reads the code and translates the message from the catalog", () => {
    expect(isMessage(message)).toBe(true);
    expect(errorCode(message)).toBe("project.not_a_project");
    expect(errorText(message)).toBe(
      "There is no ugoe project at /work/a.ugoeproj (project.json is missing).",
    );
    setLocale("ja");
    expect(errorText(message)).toBe(
      "/work/a.ugoeproj は ugoe のプロジェクトではありません (project.json がありません)。",
    );
  });

  it("shows the English message for a code the catalog does not know", () => {
    const unknown = { code: "future.error", args: {}, message: "something new happened" };
    setLocale("ja");
    expect(errorText(unknown)).toBe("something new happened");
  });

  it("translates identifier arguments and plurals", () => {
    const notFound = {
      code: "edit.not_found",
      args: { kind: "mesh", id: "EyeL" },
      message: "mesh `EyeL` not found",
    };
    setLocale("ja");
    expect(errorText(notFound)).toBe("メッシュ EyeL が見つかりません。");
    setLocale("en");
    const invalid = (count: number) => ({
      code: "format.invalid_model",
      args: { count, details: "x" },
      message: "invalid model",
    });
    expect(errorText(invalid(1))).toContain("1 problem:");
    expect(errorText(invalid(2))).toContain("2 problems:");
  });

  it("names the template that cannot rig the model", () => {
    const template = {
      code: "ai.template",
      args: { template: "module", detail: "nothing to rig" },
      message: "the module template cannot rig this model: nothing to rig",
    };
    setLocale("ja");
    expect(errorText(template)).toBe(
      "このモデルはモジュールのテンプレートで組めません (nothing to rig)。",
    );
    setLocale("en");
    expect(errorText(template)).toBe("The module template cannot rig this model (nothing to rig).");
  });

  it("appends the cause", () => {
    const rejected: Message = {
      code: "proposal.rejected",
      args: {},
      message: "proposal rejected: mesh `EyeL` not found",
      cause: { code: "edit.not_found", args: { kind: "mesh", id: "EyeL" }, message: "x" },
    };
    expect(messageText(rejected)).toBe(
      "The proposal cannot be applied. Cause: The mesh EyeL was not found.",
    );
  });

  it("falls back to the text of other errors", () => {
    expect(isMessage("disk full")).toBe(false);
    expect(errorCode("disk full")).toBeNull();
    expect(errorText("disk full")).toBe("disk full");
    expect(errorText(new Error("broken"))).toBe("broken");
    expect(errorText({ code: 1 })).toBe("[object Object]");
  });
});
