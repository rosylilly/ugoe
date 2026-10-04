import { describe, expect, it } from "vitest";
import { screenshotFileName } from "./screenshot";

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
