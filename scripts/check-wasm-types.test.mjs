// scripts/check-wasm-types.mjs の比べ方のテスト (`node --test scripts/check-wasm-types.test.mjs`)。

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";
import {
  checkAll,
  compareFull,
  comparePartial,
  FULL_COPY,
  parseDeclarations,
} from "./check-wasm-types.mjs";

const root = join(import.meta.dirname, "..");

// wasm-bindgen の生成物の書き方 (4 つの空白・1 行に 1 メンバー・doc コメント) を模したもの
const GENERATED = `/* tslint:disable */
export class WebPuppet {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * 読み込む。
     */
    load_bundle(bytes: Uint8Array, limits_json?: string | null): string[];
    parameter(id: string): number | undefined;
    static create(canvas: HTMLCanvasElement, backend?: string | null): Promise<WebPuppet>;
}

export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly webpuppet_free: (a: number, b: number) => void;
}

export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
`;

// Biome で整形した写し (2 つの空白・引数の改行・union の並びの違い)
const COPY = `// 写し
export class WebPuppet {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  load_bundle(
    bytes: Uint8Array,
    limits_json?: null | string,
  ): string[];
  parameter(id: string): number | undefined;
  static create(canvas: HTMLCanvasElement, backend?: string | null): Promise<WebPuppet>;
}

export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
}

export default function init(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput>;
`;

const generated = parseDeclarations(GENERATED);

describe("compareFull", () => {
  it("ignores formatting, union order, parameter names and InitOutput's glue exports", () => {
    assert.deepEqual(compareFull(parseDeclarations(COPY), generated), []);
  });

  it("reports a missing method", () => {
    const copy = parseDeclarations(COPY.replace(/ {2}parameter\(id: string\).*\n/, ""));
    assert.deepEqual(compareFull(copy, generated), ["WebPuppet.parameter: missing in the copy"]);
  });

  it("reports a method that is not generated", () => {
    const copy = parseDeclarations(
      COPY.replace("  free(): void;", "  free(): void;\n  gone(): void;"),
    );
    assert.deepEqual(compareFull(copy, generated), [
      "WebPuppet.gone: not in the generated (removed or renamed?)",
    ]);
  });

  it("reports a missing parameter and a different return type", () => {
    const copy = parseDeclarations(
      COPY.replace(
        /load_bundle\([\s\S]*?\): string\[\];/,
        "load_bundle(bytes: Uint8Array): string[];",
      ).replace("parameter(id: string): number | undefined;", "parameter(id: string): number;"),
    );
    assert.deepEqual(compareFull(copy, generated), [
      "WebPuppet.load_bundle: type differs: copy (Uint8Array) => string[], generated (Uint8Array, ?null | string) => string[]",
      "WebPuppet.parameter: type differs: copy (string) => number, generated (string) => number | undefined",
    ]);
  });

  it("reports a missing static method and function", () => {
    const copy = parseDeclarations(
      COPY.replace(/ {2}static create.*\n/, "").replace(/export function version.*\n/, ""),
    );
    assert.deepEqual(compareFull(copy, generated), [
      "WebPuppet.static create: missing in the copy",
      "function version: missing in the copy",
    ]);
  });
});

describe("comparePartial", () => {
  const partial = (body) =>
    parseDeclarations(`export interface P {\n${body}\n}`).containers.get("P") ??
    assert.fail("no P");
  const puppet = generated.containers.get("WebPuppet") ?? assert.fail("no WebPuppet");

  it("allows omitting trailing optional parameters and optional members that are not generated", () => {
    assert.deepEqual(
      comparePartial(
        partial("load_bundle(bytes: Uint8Array): string[];\nfollow?(dt: number): void;"),
        puppet,
        "P",
      ),
      [],
    );
  });

  it("reports a member that is not generated and a different type", () => {
    assert.deepEqual(
      comparePartial(
        partial("follow(dt: number): void;\nparameter(id: number): number | undefined;"),
        puppet,
        "P",
      ),
      [
        "P.follow: not in the generated",
        "P.parameter: type differs: copy (number) => number | undefined, generated (string) => number | undefined",
      ],
    );
  });

  it("reports a required parameter that the copy omits", () => {
    assert.deepEqual(comparePartial(partial("parameter(): number | undefined;"), puppet, "P"), [
      "P.parameter: type differs: copy () => number | undefined, generated (string) => number | undefined",
    ]);
  });
});

describe("the repository's copies", () => {
  it("match the full copy (bindings/web/stub/ugoe_wasm.d.ts)", () => {
    const text = readFileSync(join(root, FULL_COPY), "utf8");
    assert.deepEqual(checkAll(text, FULL_COPY, { compareFullCopy: false }), []);
  });
});
