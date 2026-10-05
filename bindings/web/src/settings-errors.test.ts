// 設定の差し替え (`set_effects_json`・`set_physics_json`・`set_base_physics_json`) の検査の失敗 (T0469) の見分け。

import { describe, expect, expectTypeOf, it } from "vitest";
import { isSettingsError } from "./settings-errors.js";

/** wasm-bindgen が投げる検査の失敗と同じ形 (`name` が `"Error"` で、`code` と補足の値を持つ `Error`)。 */
function codedError(code: unknown, fields: Record<string, unknown> = {}): Error {
  const e = new Error("physics settings failed validation with 1 error(s): ...");
  return Object.assign(e, { code, ...fields });
}

describe("isSettingsError", () => {
  it("accepts the validation failures of the settings setters", () => {
    const physics: unknown = codedError("invalid_physics", {
      errors: 2,
      error_codes: "unknown_reference, duplicate_id",
    });
    expect(isSettingsError(physics)).toBe(true);
    if (isSettingsError(physics)) {
      expectTypeOf(physics.code).toEqualTypeOf<"invalid_model" | "invalid_physics">();
      expectTypeOf(physics.errors).toEqualTypeOf<number | undefined>();
      expectTypeOf(physics.error_codes).toEqualTypeOf<string | undefined>();
      expect(physics.error_codes?.split(", ")).toEqual(["unknown_reference", "duplicate_id"]);
    }
    expect(isSettingsError(codedError("invalid_model", { errors: 1 }))).toBe(true);
  });

  it("rejects other errors", () => {
    // 読めない JSON などは `code` を持たない。
    expect(isSettingsError(new Error("expected value at line 1 column 1"))).toBe(false);
    expect(isSettingsError(codedError("device_lost"))).toBe(false);
    expect(isSettingsError(codedError(1))).toBe(false);
    expect(isSettingsError({ name: "Error", code: "invalid_physics" })).toBe(false);
  });
});
