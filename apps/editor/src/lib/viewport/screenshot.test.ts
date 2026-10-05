import { setLocale } from "@ugoe/i18n";
import { afterEach, describe, expect, it } from "vitest";
import { errorText } from "../errors";
import { screenshotErrorText } from "./saveScreenshot";
import { screenshotFileName } from "./screenshot";

// 画素の検査と PNG への変換のテストは packages/player/src/screenshot.test.ts。
const NOW = new Date(2026, 9, 2, 9, 5, 7);

describe("screenshotFileName", () => {
  it("プロジェクト名と地方時から作る", () => {
    expect(screenshotFileName("character", NOW)).toBe("character-20261002-090507.png");
  });

  it("使えない文字を _ にし、名前が無ければ Untitled", () => {
    expect(screenshotFileName("a/b:c", NOW)).toBe("a_b_c-20261002-090507.png");
    expect(screenshotFileName(null, NOW)).toBe("Untitled-20261002-090507.png");
    expect(screenshotFileName("  ", NOW)).toBe("Untitled-20261002-090507.png");
  });
});

describe("screenshotErrorText", () => {
  afterEach(() => setLocale("en"));

  /** wasm の `read_pixels` が投げる `RenderError` (T0465)。 */
  function renderError(code: string, message = "the GPU device was lost"): Error {
    return Object.assign(new Error(message), { name: "RenderError", code });
  }

  it("GPU が止まった読み戻しの失敗をエディタのカタログの文にする", () => {
    setLocale("ja");
    expect(screenshotErrorText(renderError("device_lost"))).toBe(
      "GPU が止まりました。描き直すには、エディタを起動し直してください。",
    );
    setLocale("en");
    expect(screenshotErrorText(renderError("device_lost"))).toBe(
      "The GPU stopped working. Restart the editor to draw again.",
    );
  });

  it("知らないコードとほかの失敗は例外の文のまま", () => {
    expect(screenshotErrorText(renderError("something_new", "new"))).toBe("new");
    expect(screenshotErrorText(new Error("plain"))).toBe("plain");
    expect(screenshotErrorText("text")).toBe("text");
  });

  it("バックエンドのエラーは errorText と同じ文にする", () => {
    const failure = { code: "io", args: { path: "/a.png" }, message: "english" };
    expect(screenshotErrorText(failure)).toBe(errorText(failure));
  });
});
