// 読み込みの警告と着せ替えの例外の型と補助 (T0438)。

import { describe, expect, expectTypeOf, it } from "vitest";
import {
  detachedModules,
  isDressUpError,
  isKnownLoadWarning,
  loadWarnings,
  type UgoeDressUpErrorCode,
  type UgoeLoadWarning,
} from "./warnings.js";
import type { WebPuppet } from "./wasm/ugoe_wasm.js";

/** wasm-bindgen が投げる `DressUpError` と同じ形 (`name`・`code`・補足の値を持つ `Error`)。 */
function dressUpError(code: string, fields: Record<string, string | number> = {}): Error {
  const e = new Error(`failed: ${code}`);
  e.name = "DressUpError";
  return Object.assign(e, { code }, fields);
}

/** `WebPuppet` の `*_json()` だけを持つ偽物 (メソッドの名前 → 返す値)。 */
const puppetWith = (values: Record<string, unknown>) =>
  Object.fromEntries(
    Object.entries(values).map(([name, value]) => [name, () => JSON.stringify(value)]),
  ) as unknown as WebPuppet;

describe("isDressUpError", () => {
  it("accepts the errors thrown by dress-up and reads the fields", () => {
    const e: unknown = dressUpError("slot_occupied", { slot: "Hair", module: "B", attached: "A" });
    if (!isDressUpError(e)) throw new Error("not a DressUpError");
    expectTypeOf(e.code).toExtend<string>();
    expectTypeOf<UgoeDressUpErrorCode>().toExtend<typeof e.code>();
    expectTypeOf(e.attached).toEqualTypeOf<string | undefined>();
    expect([e.code, e.slot, e.module, e.attached, e.parameter]).toEqual([
      "slot_occupied",
      "Hair",
      "B",
      "A",
      undefined,
    ]);
  });

  it("reads the validation error codes as a comma separated string", () => {
    const e: unknown = dressUpError("invalid_composition", {
      errors: 3,
      error_codes: "unknown_reference, duplicate_id",
    });
    if (!isDressUpError(e)) throw new Error("not a DressUpError");
    expectTypeOf(e.error_codes).toEqualTypeOf<string | undefined>();
    expect(e.error_codes?.split(", ")).toEqual(["unknown_reference", "duplicate_id"]);
  });

  it("accepts codes from a newer runtime", () => {
    expect(isDressUpError(dressUpError("future_code"))).toBe(true);
  });

  it("rejects other errors", () => {
    expect(isDressUpError(new Error("x"))).toBe(false);
    expect(isDressUpError(Object.assign(new Error("x"), { name: "DressUpError" }))).toBe(false);
    expect(isDressUpError({ name: "DressUpError", code: "no_model", message: "x" })).toBe(false);
    expect(isDressUpError("DressUpError")).toBe(false);
    expect(isDressUpError(null)).toBe(false);
  });
});

/** 利用者向けの言い換えの例。知っている種類に絞ると、`code` で補足の値の型が決まる。 */
function describeWarning(w: UgoeLoadWarning): string {
  if (!isKnownLoadWarning(w)) return `unknown ${w.code}: ${w.warning}`;
  switch (w.code) {
    case "texture_ktx2":
    case "texture_missing":
    case "normal_map_ktx2":
      expectTypeOf(w.texture).toEqualTypeOf<number>();
      return `texture ${w.texture}`;
    case "physics_removed":
      expectTypeOf(w.error_codes).toEqualTypeOf<string | undefined>();
      return `physics ${w.reason} ${w.error_codes ?? "-"}`;
    case "module_detached":
      return `detached ${w.module} ${w.error.code} ${w.error.slot}`;
    case "unknown_optional_feature":
      return `feature ${w.feature}`;
    case "unknown_value":
      return `value ${w.path}`;
  }
}

describe("loadWarnings", () => {
  const warnings = [
    { code: "texture_ktx2", warning: "a", texture: 2 },
    {
      code: "physics_removed",
      warning: "b",
      reason: "r",
      errors: 1,
      error_codes: "unknown_reference",
    },
    {
      code: "module_detached",
      warning: "c",
      module: "Hat",
      slot: "Head",
      error: { code: "missing_slot", message: "m", module: "Hat", slot: "Head" },
    },
    { code: "unknown_optional_feature", warning: "d", feature: "f" },
    { code: "unknown_value", warning: "e", path: "MODL/x" },
    { code: "future_warning", warning: "f", extra: 1 },
  ];

  it("parses load_warnings_json and narrows known codes", () => {
    const parsed = loadWarnings(puppetWith({ load_warnings_json: warnings }));
    expect(parsed).toEqual(warnings);
    const described = parsed.map(describeWarning);
    expect(described).toEqual([
      "texture 2",
      "physics r unknown_reference",
      "detached Hat missing_slot Head",
      "feature f",
      "value MODL/x",
      "unknown future_warning: f",
    ]);
  });

  it("parses detached_modules_json", () => {
    const detached = [warnings[2]];
    const parsed = detachedModules(puppetWith({ detached_modules_json: detached }));
    expect(parsed).toEqual(detached);
  });
});
