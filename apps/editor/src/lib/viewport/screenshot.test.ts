import { describe, expect, it } from "vitest";
import { checkScreenshot, encodePng, type PngCanvas, screenshotFileName } from "./screenshot";

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

describe("checkScreenshot", () => {
  it("大きさと画素の数が合うものを通す", () => {
    expect(() => checkScreenshot({ width: 2, height: 1, pixels: new Uint8Array(8) })).not.toThrow();
  });

  it("合わないものは例外", () => {
    expect(() => checkScreenshot({ width: 2, height: 2, pixels: new Uint8Array(8) })).toThrow(
      /do not match/,
    );
    expect(() => checkScreenshot({ width: 0, height: 1, pixels: new Uint8Array(0) })).toThrow(
      /invalid image size/,
    );
    expect(() => checkScreenshot({ width: 1.5, height: 1, pixels: new Uint8Array(6) })).toThrow(
      /invalid image size/,
    );
  });
});

/** putImageData に渡した画素と toBlob の形式を覚える偽の canvas。 */
function fakeCanvas(blob: Blob | null = new Blob([new Uint8Array([0x89, 0x50])])) {
  const seen: { data?: Uint8ClampedArray; at?: [number, number]; type?: string } = {};
  const canvas: PngCanvas = {
    width: 0,
    height: 0,
    getContext: () => ({
      createImageData: (w, h) => ({ data: new Uint8ClampedArray(w * h * 4) }),
      putImageData: (image, x, y) => {
        seen.data = image.data;
        seen.at = [x, y];
      },
    }),
    toBlob: (callback, type) => {
      seen.type = type;
      callback(blob);
    },
  };
  return { canvas, seen };
}

describe("encodePng", () => {
  it("RGBA をそのまま canvas に置き、PNG で取り出す", async () => {
    const { canvas, seen } = fakeCanvas();
    const pixels = new Uint8Array([255, 0, 0, 255, 0, 0, 255, 128]);
    const png = await encodePng({ width: 2, height: 1, pixels }, () => canvas);
    expect([canvas.width, canvas.height]).toEqual([2, 1]);
    expect([...(seen.data ?? [])]).toEqual([...pixels]);
    expect(seen.at).toEqual([0, 0]);
    expect(seen.type).toBe("image/png");
    expect([...png]).toEqual([0x89, 0x50]);
  });

  it("大きさが合わない・変換できないときは例外", async () => {
    const { canvas } = fakeCanvas(null);
    await expect(
      encodePng({ width: 1, height: 1, pixels: new Uint8Array(3) }, () => canvas),
    ).rejects.toThrow(/do not match/);
    await expect(
      encodePng({ width: 1, height: 1, pixels: new Uint8Array(4) }, () => canvas),
    ).rejects.toThrow(/cannot encode PNG/);
  });
});
