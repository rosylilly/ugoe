// 描画と読み戻しの失敗 (`RenderError`。T0465) の見分け。

import { describe, expect, expectTypeOf, it } from "vitest";
import { isRenderError } from "./render-errors.js";

/** wasm-bindgen が投げる描画の例外と同じ形 (`name`・`code` を持つ `Error`)。 */
function codedError(name: string, code: unknown): Error {
  const e = new Error("the GPU device was lost");
  e.name = name;
  return Object.assign(e, { code });
}

describe("isRenderError", () => {
  it("accepts a coded RenderError", () => {
    const e: unknown = codedError("RenderError", "device_lost");
    expect(isRenderError(e)).toBe(true);
    if (isRenderError(e)) expectTypeOf(e.code).toExtend<string>();
  });

  it("rejects other errors", () => {
    expect(isRenderError(codedError("LoadError", "device_lost"))).toBe(false);
    expect(isRenderError(codedError("RenderError", 1))).toBe(false);
    expect(isRenderError(new Error("the GPU device was lost"))).toBe(false);
    expect(isRenderError({ name: "RenderError", code: "device_lost" })).toBe(false);
  });
});
