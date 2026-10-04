import { setLocale } from "@ugoe/i18n";
import { beforeEach, describe, expect, it } from "vitest";
import { CommandError, commandFailure, errorText, isCommandMessage, messageText } from "./errors";

beforeEach(() => setLocale("ja"));

const notModel = {
  code: "file.not_model",
  args: { path: "/m/a.psd" },
  message: "not a model or module file (.ugoe, .json or .ugoemod): /m/a.psd",
};

describe("command errors", () => {
  it("translates the code with its args", () => {
    expect(messageText(notModel)).toBe(
      "/m/a.psd はモデルでも着せ替えのモジュールでもありません (開けるのは .ugoe・.json・.ugoemod のファイル)",
    );
    setLocale("en");
    expect(messageText(notModel)).toBe(
      "/m/a.psd is not a model or a dress-up module (only .ugoe, .json and .ugoemod files can be opened)",
    );
  });

  it("formats numbers in the current language", () => {
    setLocale("en");
    const error = {
      code: "file.too_large",
      args: { path: "/m/a.ugoe", size: 600000000, max: 536870912 },
      message: "/m/a.ugoe is too large",
    };
    expect(messageText(error)).toBe(
      "/m/a.ugoe is too large to open (600,000,000 bytes; the limit is 536,870,912 bytes)",
    );
  });

  it("shows the English message for an unknown code", () => {
    const error = { code: "file.something_new", args: {}, message: "something new happened" };
    expect(messageText(error)).toBe("something new happened");
  });

  it("turns a rejected value into an error", () => {
    expect(isCommandMessage(notModel)).toBe(true);
    expect(isCommandMessage("text")).toBe(false);
    expect(isCommandMessage({ code: 1, message: "x" })).toBe(false);
    const error = commandFailure(notModel);
    expect(error).toBeInstanceOf(CommandError);
    expect(error).toBeInstanceOf(Error);
    expect((error as CommandError).code).toBe("file.not_model");
    expect(commandFailure("plain")).toBe("plain");
  });

  it("describes any thrown value", () => {
    expect(errorText(notModel)).toMatch(/^\/m\/a\.psd はモデルでも/);
    expect(errorText(new Error("broken"))).toBe("broken");
    expect(errorText("text")).toBe("text");
  });
});
