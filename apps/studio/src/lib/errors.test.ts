import { setLocale } from "@ugoe/i18n";
import { beforeEach, describe, expect, it } from "vitest";
import { englishErrorText } from "./commandMessage";
import { describeError, isCommandMessage, isSceneError, messageText } from "./errors";

beforeEach(() => setLocale("ja"));

const missing = {
  code: "scene.model_missing",
  args: { asset: "model", path: "/m/avatar.ugoe" },
  message: "/m/avatar.ugoe: not found",
};

describe("command errors", () => {
  it("translates the code with its args", () => {
    expect(messageText(missing)).toBe("モデルが見つかりません: /m/avatar.ugoe");
    setLocale("en");
    expect(messageText(missing)).toBe("The model was not found: /m/avatar.ugoe");
    expect(
      messageText({
        code: "profile.future_version",
        args: { path: "p.json", version: 2, supported: 1 },
        message: "p.json: profile version 2 is newer than this studio (up to 1)",
      }),
    ).toMatch(/p\.json.*version 2.*version 1/);
  });

  it("formats numbers in the current language", () => {
    setLocale("en");
    const error = {
      code: "profile.too_large",
      args: { path: "/p/big.json", size: 20000000 },
      message: "/p/big.json: the file is too large (20000000 bytes)",
    };
    expect(messageText(error)).toBe("/p/big.json is too large (20,000,000 bytes).");
  });

  it("shows the English message for an unknown code", () => {
    const error = { code: "scene.something_new", args: {}, message: "something new happened" };
    expect(messageText(error)).toBe("something new happened");
    expect(describeError(error)).toBe("something new happened");
  });

  it("describes any thrown value", () => {
    expect(isCommandMessage(missing)).toBe(true);
    expect(isCommandMessage("text")).toBe(false);
    expect(isCommandMessage({ code: 1, message: "x" })).toBe(false);
    // 前の形 (`message` の無い平らな形) はコマンドのエラーとみなさない。
    expect(isCommandMessage({ code: "missing", asset: "model", path: "/m/a.ugoe" })).toBe(false);
    expect(describeError(missing)).toBe("モデルが見つかりません: /m/avatar.ugoe");
    expect(
      describeError({ code: "internal", args: { reason: "task panicked" }, message: "x" }),
    ).toContain("task panicked");
    expect(describeError(new Error("boom"))).toBe("boom");
    expect(describeError("text")).toBe("text");
  });

  it("tells scene errors from the others", () => {
    expect(isSceneError(missing)).toBe(true);
    expect(isSceneError({ code: "profile.not_json", args: {}, message: "x" })).toBe(false);
    expect(isSceneError({ code: "scene_file.not_current", args: {}, message: "x" })).toBe(false);
    expect(isSceneError(new Error("scene.unknown"))).toBe(false);
  });

  it("keeps the English text for windows without a catalog", () => {
    expect(englishErrorText(missing)).toBe("/m/avatar.ugoe: not found");
    expect(englishErrorText(new Error("boom"))).toBe("boom");
    expect(englishErrorText(3)).toBe("3");
  });
});
