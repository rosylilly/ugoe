import { describe, expect, it } from "vitest";
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
