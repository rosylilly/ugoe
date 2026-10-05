import { setLocale } from "@ugoe/i18n";
import { afterEach, describe, expect, it } from "vitest";
import { errorText } from "./errors";
import { screenshotErrorText, screenshotFileName } from "./screenshot";

// 画素の検査と PNG への変換のテストは packages/player/src/screenshot.test.ts。
const NOW = new Date(2026, 9, 2, 9, 5, 7);

describe("screenshotFileName", () => {
  it("モデルの名前と地方時から作る", () => {
    expect(screenshotFileName("avatar.ugoe", NOW)).toBe("avatar-20261002-090507.png");
    expect(screenshotFileName("model.JSON", NOW)).toBe("model-20261002-090507.png");
  });

  it("見本の括弧書きを除く", () => {
    expect(screenshotFileName("avatar.ugoe (サンプル)", NOW)).toBe("avatar-20261002-090507.png");
  });

  it("使えない文字を _ にし、名前が無ければ screenshot", () => {
    expect(screenshotFileName('a:b*c?"d.ugoe', NOW)).toBe("a_b_c__d-20261002-090507.png");
    expect(screenshotFileName(null, NOW)).toBe("screenshot-20261002-090507.png");
    expect(screenshotFileName(" .ugoe", NOW)).toBe("screenshot-20261002-090507.png");
  });
});

describe("screenshotErrorText", () => {
  afterEach(() => setLocale("en"));

  /** wasm の `read_pixels` が投げる `RenderError` (T0465)。 */
  function renderError(code: string, message = "the GPU device was lost"): Error {
    return Object.assign(new Error(message), { name: "RenderError", code });
  }

  it("GPU が止まった読み戻しの失敗を今の言語の文にする", () => {
    setLocale("ja");
    expect(screenshotErrorText(renderError("device_lost"))).toContain("GPU が止まりました");
    setLocale("en");
    expect(screenshotErrorText(renderError("device_lost"))).toContain("The GPU stopped working");
  });

  it("知らないコードとほかの失敗は例外の文のまま", () => {
    expect(screenshotErrorText(renderError("something_new", "new"))).toBe("new");
    expect(screenshotErrorText(new Error("plain"))).toBe("plain");
    expect(screenshotErrorText("text")).toBe("text");
  });

  it("コマンドのエラーは errorText と同じ文にする", () => {
    setLocale("ja");
    const failure = { code: "file.not_model", args: { path: "/m/a.psd" }, message: "english" };
    expect(screenshotErrorText(failure)).toBe(errorText(failure));
    expect(screenshotErrorText(failure)).not.toBe("english");
  });
});
