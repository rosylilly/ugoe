// 作成の失敗 (`CreateError`。T0475) の見分け。

import { describe, expect, expectTypeOf, it } from "vitest";
import { isCreateError } from "./create-errors.js";

/** wasm-bindgen が投げる作成の例外と同じ形 (`name`・`code` を持つ `Error`)。 */
function codedError(name: string, code: unknown): Error {
  const e = new Error("the canvas could not provide a GPU context: the WebGL2 context is lost");
  e.name = name;
  return Object.assign(e, { code });
}

describe("isCreateError", () => {
  it("accepts a coded CreateError", () => {
    const e: unknown = codedError("CreateError", "context_unavailable");
    expect(isCreateError(e)).toBe(true);
    if (isCreateError(e)) expectTypeOf(e.code).toExtend<string>();
  });

  it("rejects other errors", () => {
    expect(isCreateError(codedError("RenderError", "context_unavailable"))).toBe(false);
    expect(isCreateError(codedError("CreateError", 1))).toBe(false);
    expect(isCreateError(new Error("unknown backend"))).toBe(false);
    expect(isCreateError({ name: "CreateError", code: "context_unavailable" })).toBe(false);
  });
});
