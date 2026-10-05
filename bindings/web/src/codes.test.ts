// Rust の `code` の一覧 (crates/ugoe-wasm/codes.json) と、このパッケージの和型の突き合わせ (T0452)。
//
// codes.json は ugoe-wasm のテスト (`src/codes.rs`) が、すべての種類の見本から作った一覧と比べている。
// ここではその一覧と TS の `code` の配列・補足の値のキーを比べる。Rust と TS の片方だけに種類やキーを足すと、
// どちらかのテストが落ちる。直し方は crates/ugoe-wasm/src/codes.rs の先頭のコメント。

import { describe, expect, expectTypeOf, it } from "vitest";
import codes from "../../../crates/ugoe-wasm/codes.json" with { type: "json" };
import { CREATE_ERROR_CODES } from "./create-errors.js";
import { LOAD_ERROR_CODES, type UgoeLoadErrorFields } from "./limits.js";
import { RENDER_ERROR_CODES } from "./render-errors.js";
import { SETTINGS_ERROR_CODES, type UgoeSettingsErrorFields } from "./settings-errors.js";
import {
  DRESS_UP_ERROR_CODES,
  LOAD_WARNING_CODES,
  type UgoeDressUpErrorFields,
  type UgoeKnownLoadWarning,
} from "./warnings.js";

type Codes = Record<string, string[]>;

const sorted = (values: Iterable<string>) => [...values].sort();
/** すべての種類のキーの和。 */
const allFields = (list: Codes) => sorted(new Set(Object.values(list).flat()));

/** 着せ替えの失敗の補足の値のキー (`UgoeDressUpErrorFields` のキーと同じであることを型で確かめる)。 */
const DRESS_UP_ERROR_FIELDS = [
  "module",
  "slot",
  "parameter",
  "mesh",
  "attached",
  "mask",
  "rig",
  "collider",
  "part",
  "texture",
  "required",
  "actual",
  "errors",
  "error_codes",
  "limit",
  "chunk",
  "max",
  "image",
  "width",
  "height",
] as const;

/** 読み込みの失敗の補足の値のキー (`UgoeLoadErrorFields` のキーと同じ)。 */
const LOAD_ERROR_FIELDS = [
  "found",
  "supported",
  "features",
  "errors",
  "error_codes",
  "limit",
  "chunk",
  "max",
  "image",
  "texture",
  "width",
  "height",
] as const;

/** 設定の差し替えの検査の失敗の補足の値のキー (`UgoeSettingsErrorFields` のキーと同じ)。 */
const SETTINGS_ERROR_FIELDS = ["errors", "error_codes"] as const;

/** `code` に `C` を含む読み込みの警告の型 (`UgoeTextureLoadWarning` は 3 つの `code` を持つ)。 */
type WarningOf<C extends string, W = UgoeKnownLoadWarning> = W extends { code: infer K }
  ? C extends K
    ? W
    : never
  : never;
/** 読み込みの警告の種類 `C` の補足の値のキー (`code` と `warning` を除く)。 */
type WarningFields<C extends UgoeKnownLoadWarning["code"]> = Exclude<
  keyof WarningOf<C>,
  "code" | "warning"
>;

/** 読み込みの警告の種類ごとのキー (`UgoeKnownLoadWarning` の各型のキーと同じ)。 */
const LOAD_WARNING_FIELDS = {
  texture_ktx2: ["texture"],
  texture_missing: ["texture"],
  normal_map_ktx2: ["texture"],
  physics_removed: ["reason", "errors", "error_codes"],
  module_detached: ["module", "slot", "error"],
  unknown_optional_feature: ["feature"],
  unknown_value: ["path"],
} as const;

describe("codes.json (Rust) and the TypeScript unions", () => {
  it("lists the same dress-up error codes and fields", () => {
    expectTypeOf<(typeof DRESS_UP_ERROR_FIELDS)[number]>().toEqualTypeOf<
      keyof UgoeDressUpErrorFields
    >();
    expect(sorted(DRESS_UP_ERROR_CODES)).toEqual(sorted(Object.keys(codes.dress_up_error)));
    expect(sorted(DRESS_UP_ERROR_FIELDS)).toEqual(allFields(codes.dress_up_error));
  });

  it("lists the same load error codes and fields", () => {
    expectTypeOf<(typeof LOAD_ERROR_FIELDS)[number]>().toEqualTypeOf<keyof UgoeLoadErrorFields>();
    expect(sorted(LOAD_ERROR_CODES)).toEqual(sorted(Object.keys(codes.load_error)));
    expect(sorted(LOAD_ERROR_FIELDS)).toEqual(allFields(codes.load_error));
  });

  it("lists the same create error codes (without fields)", () => {
    expect(sorted(CREATE_ERROR_CODES)).toEqual(sorted(Object.keys(codes.create_error)));
    expect(allFields(codes.create_error)).toEqual([]);
  });

  it("lists the same render error codes (without fields)", () => {
    expect(sorted(RENDER_ERROR_CODES)).toEqual(sorted(Object.keys(codes.render_error)));
    expect(allFields(codes.render_error)).toEqual([]);
  });

  // `set_base_physics_json` の合成の失敗は `DressUpError` なので、codes.json では `dress_up_error` に入る (T0476)。
  it("lists the same settings error codes and fields", () => {
    expectTypeOf<(typeof SETTINGS_ERROR_FIELDS)[number]>().toEqualTypeOf<
      keyof UgoeSettingsErrorFields
    >();
    expect(sorted(SETTINGS_ERROR_CODES)).toEqual(sorted(Object.keys(codes.settings_error)));
    expect(sorted(SETTINGS_ERROR_FIELDS)).toEqual(allFields(codes.settings_error));
  });

  it("lists the same load warning codes and fields per code", () => {
    expectTypeOf<(typeof LOAD_WARNING_CODES)[number]>().toEqualTypeOf<
      UgoeKnownLoadWarning["code"]
    >();
    expectTypeOf<{
      -readonly [C in keyof typeof LOAD_WARNING_FIELDS]: (typeof LOAD_WARNING_FIELDS)[C][number];
    }>().toEqualTypeOf<{ [C in UgoeKnownLoadWarning["code"]]: WarningFields<C> }>();
    expect(sorted(LOAD_WARNING_CODES)).toEqual(sorted(Object.keys(codes.load_warning)));
    const fields = Object.fromEntries(
      Object.entries(LOAD_WARNING_FIELDS).map(([code, keys]) => [code, sorted(keys)]),
    );
    expect(fields).toEqual(codes.load_warning);
  });
});
