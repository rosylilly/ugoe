// 手書きの型 (ugoe_wasm.d.ts) が crates/ugoe-wasm の公開 API に追いついているか。
// 生成物が無い環境の型検査 (scripts/check.mjs) はこのスタブを使うので、ずれると新しい API を使うコードを検査できない。
// 名前だけを比べる (引数と戻り値の型は生成物から写す。README.md#開発)。

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const crate = join(import.meta.dirname, "..", "..", "..", "crates", "ugoe-wasm", "src");

/**
 * `#[wasm_bindgen]` の `impl WebPuppet` のメソッドと、`#[wasm_bindgen]` の関数の名前。
 * `internal` は doc コメントに `@internal` を持つメソッド (エディタ向けで公開の API の外)。
 */
function rustExports() {
  const methods = new Set();
  const internal = new Set();
  const functions = new Set();
  for (const entry of readdirSync(crate, { recursive: true })) {
    const file = String(entry);
    if (!file.endsWith(".rs")) continue;
    const lines = readFileSync(join(crate, file), "utf8").split("\n");
    for (let i = 0; i < lines.length; i++) {
      if (lines[i]?.trim() !== "#[wasm_bindgen]") continue;
      const next = lines[i + 1] ?? "";
      const fn = next.match(/^pub (?:async )?fn (\w+)/);
      if (fn?.[1]) functions.add(fn[1]);
      if (!next.startsWith("impl WebPuppet")) continue;
      for (let j = i + 2; j < lines.length && lines[j] !== "}"; j++) {
        const m = lines[j]?.match(/^ {4}pub (?:async )?fn (\w+)/);
        if (!m?.[1]) continue;
        methods.add(m[1]);
        for (let k = j - 1; k >= 0 && lines[k]?.startsWith("    ///"); k--) {
          if (/^ {4}\/\/\/ @internal\b/.test(lines[k] ?? "")) internal.add(m[1]);
        }
      }
    }
  }
  return { methods, internal, functions };
}

/** スタブの `WebPuppet` のメソッドと、`export function` の名前 (wasm-bindgen の初期化の関数を除く)。 */
function stubExports() {
  const text = readFileSync(join(import.meta.dirname, "ugoe_wasm.d.ts"), "utf8");
  const body = text.match(/export class WebPuppet \{([\s\S]*?)\n\}/)?.[1] ?? "";
  const methods = new Set(
    [...body.matchAll(/^ {2}(?:static )?(\w+)\(/gm)].map((m) => m[1]).filter((n) => n !== "free"),
  );
  const internal = new Set(
    [...body.matchAll(/^ {2}\/\*\* @internal\b.*\*\/\n {2}(?:static )?(\w+)\(/gm)].map((m) => m[1]),
  );
  const functions = new Set(
    [...text.matchAll(/^export function (\w+)/gm)].map((m) => m[1]).filter((n) => n !== "initSync"),
  );
  return { methods, internal, functions };
}

describe("stub/ugoe_wasm.d.ts", () => {
  it("has the same WebPuppet methods and functions as crates/ugoe-wasm", () => {
    const rust = rustExports();
    const stub = stubExports();
    expect(rust.methods.size).toBeGreaterThan(50);
    expect([...stub.methods].sort()).toEqual([...rust.methods].sort());
    expect([...stub.functions].sort()).toEqual([...rust.functions].sort());
  });

  it("marks the same WebPuppet methods @internal as crates/ugoe-wasm", () => {
    const rust = rustExports();
    const stub = stubExports();
    // エディタ向けの問い合わせ (crates/ugoe-wasm/README.md)
    expect(rust.internal).toContain("rotation_local_keyform");
    expect([...stub.internal].sort()).toEqual([...rust.internal].sort());
  });
});
