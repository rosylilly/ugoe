#!/usr/bin/env node
// C ABI のヘッダ (crates/ugoe-ffi/include/ugoe.h) の関数と、C ABI を呼ぶバインディングの宣言を突き合わせる (`just refs`)。
//
// - ヘッダにあってバインディングに無い関数は、OMIT に理由を書いて除外しない限りエラー。
// - バインディングにあってヘッダに無い関数 (消した・名前を変えた関数) はエラー。
// - 型と引数までは比べない (それは `just unity-check` のテストと実機で確かめる)。
//
// M9 のコードレビュー (T0423・T0426) で、Unity に 9 つの関数の宣言が漏れていたことから足した (docs/workflow/code-review.md#m9-の結果)。
// Godot のバインディングは C ABI ではなく SDK (ugoe クレート) を直接使うので対象の外。

import { readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const HEADER = "crates/ugoe-ffi/include/ugoe.h";

/** C ABI を呼ぶバインディング。`omit` はヘッダの関数のうち宣言しないもの (名前 -> 理由)。 */
export const BINDINGS = [
  {
    name: "unity",
    file: "bindings/unity/com.ugoe.unity/Runtime/Core/Native.cs",
    omit: {
      ugoe_puppet_positions:
        "内部のバッファの借用。C# では配列に写すことになり ugoe_puppet_copy_positions と同じ (bindings/unity/README.md)",
    },
  },
];

/** ヘッダで宣言している関数の名前 (コメントの行は見ない)。 */
export function headerFunctions(text) {
  const names = new Set();
  for (const line of text.split("\n")) {
    if (/^\s*(\/\/|\/?\*)/.test(line)) continue;
    for (const m of line.matchAll(/\b(ugoe_[a-z0-9_]+)\s*\(/g)) names.add(m[1]);
  }
  return names;
}

/** C# の P/Invoke で宣言している関数の名前 (`EntryPoint` があればそちら)。 */
export function csharpFunctions(text) {
  const names = new Set();
  const decl = /\[DllImport\(([^\]]*)\)\][^;]*?\bstatic\s+extern\s+[^(;]*?\b(\w+)\s*\(/g;
  for (const m of text.matchAll(decl)) {
    const entry = /EntryPoint\s*=\s*"([^"]+)"/.exec(m[1]);
    names.add(entry ? entry[1] : m[2]);
  }
  return names;
}

/** ヘッダとバインディングの差。誤りの文の列を返す。 */
export function compare(header, declared, binding) {
  const errors = [];
  for (const name of [...header].sort()) {
    if (declared.has(name)) {
      if (name in binding.omit) {
        errors.push(`${binding.name}: ${name} は宣言されているので OMIT から外す`);
      }
    } else if (!(name in binding.omit)) {
      errors.push(`${binding.name}: ヘッダの ${name} が ${binding.file} に宣言されていない`);
    }
  }
  for (const name of [...declared].sort()) {
    if (!header.has(name)) {
      errors.push(`${binding.name}: ${binding.file} の ${name} がヘッダに無い`);
    }
  }
  for (const name of Object.keys(binding.omit).sort()) {
    if (!header.has(name)) errors.push(`${binding.name}: OMIT の ${name} がヘッダに無い`);
  }
  return errors;
}

function main() {
  const header = headerFunctions(readFileSync(join(ROOT, HEADER), "utf8"));
  if (header.size === 0) {
    console.error(`error: ${HEADER} から関数を読めなかった`);
    return 1;
  }
  const errors = BINDINGS.flatMap((b) =>
    compare(header, csharpFunctions(readFileSync(join(ROOT, b.file), "utf8")), b),
  );
  for (const e of errors) console.error(`error: ${e}`);
  if (errors.length > 0) {
    console.error(
      `ヘッダ (${relative(process.cwd(), join(ROOT, HEADER))}) に合わせて宣言を直すか、scripts/check-ffi-bindings.mjs の omit に理由を書く。`,
    );
    return 1;
  }
  console.log(`ffi bindings ok (${header.size} functions, ${BINDINGS.length} binding(s))`);
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main();
}
