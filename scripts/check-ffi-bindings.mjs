#!/usr/bin/env node
// C ABI のヘッダ (crates/ugoe-ffi/include/ugoe.h) の関数と、C ABI を呼ぶコードの関数を突き合わせる (`just refs`)。
//
// - ヘッダにあって呼ぶ側に無い関数は、omit に理由を書いて除外しない限りエラー。
// - 呼ぶ側にあってヘッダに無い関数 (消した・名前を変えた関数) はエラー。
// - 型と引数までは比べない (それは `just unity-check` のテストと実機で確かめる)。
//
// 呼ぶ側は 2 つ。
// - Unity のバインディングの P/Invoke の宣言。M9 のコードレビュー (T0423・T0426) で、9 つの関数の宣言が漏れていたことから足した
//   (docs/workflow/code-review.md#m9-の結果)。Godot のバインディングは C ABI ではなく SDK (ugoe クレート) を直接使うので対象の外。
// - fuzz の入口 (crates/ugoe-ffi/src/fuzzing.rs。T0497)。C ABI に関数を足したときの fuzz への入れ忘れを止める (T0498)。
//   呼ぶ (関数のポインタとして渡すのも含む) かどうかだけを見て、どの入力で届くかまでは見ない。

import { readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const HEADER = "crates/ugoe-ffi/include/ugoe.h";

/**
 * C ABI を呼ぶコード。`parse` は呼ぶ側のファイルから関数の名前を集める (下で定義)。`verb` は誤りの文に使う
 * (「宣言されて」いない・「呼ばれて」いない)。`omit` はヘッダの関数のうち呼ばないもの (名前 -> 理由)。
 */
export const BINDINGS = [
  {
    name: "unity",
    file: "bindings/unity/com.ugoe.unity/Runtime/Core/Native.cs",
    parse: (text) => csharpFunctions(text),
    verb: "宣言されて",
    omit: {
      ugoe_puppet_frame:
        "毎フレームの状態の借用。C# では配列に写すことになり ugoe_puppet_copy_frame と同じ (bindings/unity/README.md)",
    },
  },
  {
    name: "fuzz",
    file: "crates/ugoe-ffi/src/fuzzing.rs",
    parse: (text) => rustFunctions(text),
    verb: "呼ばれて",
    omit: {
      ugoe_abi_version: "引数を取らず、決まった数を返すだけ",
      ugoe_version: "引数を取らず、決まった文字列を返すだけ",
      ugoe_read_limits_default: "引数を取らず、決まった上限を返すだけ",
      ugoe_image_limits_default: "引数を取らず、決まった上限を返すだけ",
      ugoe_set_panic_handler:
        "プロセスに 1 つの登録を変えるだけで入力を読まない。std がある形 (fuzz のビルド) では登録した関数を呼ばない",
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

/**
 * Rust のコードが使う C ABI の関数の名前 (呼び出しと、関数のポインタとして渡すもの)。コメントと文字列の中の名前は数えない
 * (`self.check("ugoe_puppet_update", code)` の文字列は、呼んだことにしない)。
 */
export function rustFunctions(text) {
  let code = "";
  for (let i = 0; i < text.length; ) {
    if (text.startsWith("//", i)) {
      const end = text.indexOf("\n", i);
      i = end < 0 ? text.length : end;
    } else if (text.startsWith("/*", i)) {
      const end = text.indexOf("*/", i + 2);
      i = end < 0 ? text.length : end + 2;
    } else if (text[i] === '"') {
      i += 1;
      while (i < text.length && text[i] !== '"') i += text[i] === "\\" ? 2 : 1;
      i += 1;
      code += '""';
    } else {
      code += text[i];
      i += 1;
    }
  }
  return new Set([...code.matchAll(/\b(ugoe_[a-z0-9_]+)\b/g)].map((m) => m[1]));
}

/** ヘッダと呼ぶ側の差。誤りの文の列を返す。 */
export function compare(header, declared, binding) {
  const verb = binding.verb ?? "宣言されて";
  const errors = [];
  for (const name of [...header].sort()) {
    if (declared.has(name)) {
      if (name in binding.omit) {
        errors.push(`${binding.name}: ${name} は${verb}いるので omit から外す`);
      }
    } else if (!(name in binding.omit)) {
      errors.push(`${binding.name}: ヘッダの ${name} が ${binding.file} で${verb}いない`);
    }
  }
  for (const name of [...declared].sort()) {
    if (!header.has(name)) {
      errors.push(`${binding.name}: ${binding.file} の ${name} がヘッダに無い`);
    }
  }
  for (const name of Object.keys(binding.omit).sort()) {
    if (!header.has(name)) errors.push(`${binding.name}: omit の ${name} がヘッダに無い`);
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
    compare(header, b.parse(readFileSync(join(ROOT, b.file), "utf8")), b),
  );
  for (const e of errors) console.error(`error: ${e}`);
  if (errors.length > 0) {
    console.error(
      `ヘッダ (${relative(process.cwd(), join(ROOT, HEADER))}) に合わせて宣言や呼び出しを足すか、scripts/check-ffi-bindings.mjs の omit に理由を書く。`,
    );
    return 1;
  }
  console.log(`ffi bindings ok (${header.size} functions, ${BINDINGS.length} caller(s))`);
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main();
}
