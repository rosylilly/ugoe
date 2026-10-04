// 読み込みの上限の JSON と、上限を超えた例外の見分け (T0152)。

import { describe, expect, expectTypeOf, it } from "vitest";
import { isLimitError, isLoadError, limitsJson } from "./limits.js";

/** wasm-bindgen が投げる読み込みの例外と同じ形 (`name`・`code`・補足の値を持つ `Error`)。 */
function codedError(name: string, code: string, fields: Record<string, unknown> = {}): Error {
  const e = new Error(`failed: ${code}`);
  e.name = name;
  return Object.assign(e, { code }, fields);
}

describe("limitsJson", () => {
  it("leaves the defaults when no limits are given", () => {
    expect(limitsJson(undefined)).toBeUndefined();
    expect(limitsJson({})).toBe("{}");
  });

  it("writes the names the wasm reads and drops the missing ones", () => {
    expect(JSON.parse(limitsJson({ maxTotalDecompressed: 1024, maxImageSide: 64 }) ?? "")).toEqual({
      max_total_decompressed: 1024,
      max_image_side: 64,
    });
    expect(JSON.parse(limitsJson({ maxChunkDecompressed: 1 }) ?? "")).toEqual({
      max_chunk_decompressed: 1,
    });
  });
});

describe("isLimitError", () => {
  it("accepts limit errors from load_bundle and load_module", () => {
    const e: unknown = codedError("LoadError", "read_limit_exceeded", {
      limit: "total_decompressed",
      chunk: "TEXR",
      max: 1024,
    });
    if (!isLimitError(e)) throw new Error("not a limit error");
    expectTypeOf(e.max).toEqualTypeOf<number | undefined>();
    expect([e.code, e.limit, e.chunk, e.max]).toEqual([
      "read_limit_exceeded",
      "total_decompressed",
      "TEXR",
      1024,
    ]);
    expect(
      isLimitError(
        codedError("DressUpError", "image_limit_exceeded", { image: "texture", texture: 0 }),
      ),
    ).toBe(true);
  });

  it("rejects other errors", () => {
    expect(isLimitError(codedError("DressUpError", "invalid_file"))).toBe(false);
    expect(isLimitError(codedError("TypeError", "read_limit_exceeded"))).toBe(false);
    expect(isLimitError(new Error("read_limit_exceeded"))).toBe(false);
    expect(isLimitError({ name: "LoadError", code: "read_limit_exceeded" })).toBe(false);
    expect(isLimitError(null)).toBe(false);
  });
});

describe("isLoadError", () => {
  it("accepts every load failure of load_json and load_bundle", () => {
    const e: unknown = codedError("LoadError", "unsupported_version", { found: 9, supported: 3 });
    if (!isLoadError(e)) throw new Error("not a load error");
    expectTypeOf(e.found).toEqualTypeOf<number | undefined>();
    expect([e.code, e.found, e.supported]).toEqual(["unsupported_version", 9, 3]);
    // 上限を超えたものも `LoadError` (isLimitError でさらに絞れる)。
    const limit = codedError("LoadError", "image_limit_exceeded", { image: "texture", texture: 0 });
    expect(isLoadError(limit) && isLimitError(limit)).toBe(true);
    // 知らないコード (新しい版のランタイム) も受ける。
    expect(isLoadError(codedError("LoadError", "future_code"))).toBe(true);
  });

  it("rejects other errors", () => {
    expect(isLoadError(codedError("DressUpError", "invalid_file"))).toBe(false);
    expect(isLoadError(new Error("invalid_file"))).toBe(false);
    expect(isLoadError({ name: "LoadError", code: "invalid_file" })).toBe(false);
    expect(isLoadError(null)).toBe(false);
  });
});
