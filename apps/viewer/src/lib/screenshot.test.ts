import { setLocale } from "@ugoe/i18n";
import { beforeEach, describe, expect, it } from "vitest";
import { checkScreenshot, encodePng, type PngCanvas, screenshotFileName } from "./screenshot";

const NOW = new Date(2026, 9, 2, 9, 5, 7);

// 例外の文は画面に出る (`screenshot.error`) ので、今の言語で書く。ここでは日本語の訳で確かめる。
beforeEach(() => setLocale("ja"));

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

describe("checkScreenshot", () => {
  it("大きさと画素の数が合うものを通す", () => {
    expect(() => checkScreenshot({ width: 2, height: 1, pixels: new Uint8Array(8) })).not.toThrow();
  });

  it("合わないものは例外", () => {
    expect(() => checkScreenshot({ width: 2, height: 2, pixels: new Uint8Array(8) })).toThrow(
      /画素の数/,
    );
    expect(() => checkScreenshot({ width: 0, height: 1, pixels: new Uint8Array(0) })).toThrow(
      /大きさ/,
    );
    expect(() => checkScreenshot({ width: 1.5, height: 1, pixels: new Uint8Array(6) })).toThrow(
      /大きさ/,
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
    ).rejects.toThrow(/画素の数/);
    await expect(
      encodePng({ width: 1, height: 1, pixels: new Uint8Array(4) }, () => canvas),
    ).rejects.toThrow(/PNG に変換できません/);
  });
});

describe("messages", () => {
  it("speaks the current language", async () => {
    setLocale("en");
    expect(() => checkScreenshot({ width: 0, height: 1, pixels: new Uint8Array(0) })).toThrow(
      "The image size is invalid (0×1)",
    );
    expect(() => checkScreenshot({ width: 1, height: 1, pixels: new Uint8Array(3) })).toThrow(
      "The pixel data does not match the size (3 bytes; 1×1 RGBA needs 4 bytes)",
    );
    const { canvas } = fakeCanvas(null);
    await expect(
      encodePng({ width: 1, height: 1, pixels: new Uint8Array(4) }, () => canvas),
    ).rejects.toThrow("Cannot convert the image to PNG");
  });
});
