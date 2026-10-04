#!/usr/bin/env node
// ugoe-wasm の手書きの型の写しを、wasm-bindgen の生成物 (`just wasm` の ugoe_wasm.d.ts) と比べる (`just wasm-types`)。
//
//   node scripts/check-wasm-types.mjs [<生成物の ugoe_wasm.d.ts>]
//
// 写しは 2 種類ある (docs/tasks/T0435-wasm-type-copies.md):
// - 全体の写し `bindings/web/stub/ugoe_wasm.d.ts`。生成物が無い環境の型検査で使う (@ugoe/runtime と 3 つのアプリ)。
//   クラス・メンバー・関数・型の別名が生成物と同じであること (引数の名前は比べない)。
//   interface (InitOutput) は wasm の内部の口なので、写しにあるものが生成物にあることだけを見る。
// - 一部の写し (使うものだけを書いた interface)。@ugoe/player の `WebPuppet` と @ugoe/face-tracking の `TrackingPuppet`。
//   どのメンバーも生成物にあり、型が同じであること。写しは末尾の省略できる引数を省いてよい。
//   `?` を付けたメンバー (使わないアプリがある) は生成物に無くてもよい。
//
// 引数を省くと、生成物の既定の場所 (エディタの `just wasm` の出力) を使う。それも無ければ (wasm-bindgen-cli の無い手元)、
// 一部の写しを全体の写しと比べる (全体の写しと Rust のソースの名前の比べ合わせは bindings/web/stub/ugoe_wasm.test.mjs)。
// 違いがあれば一覧を出して終了コード 1 で終える。

import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join, relative } from "node:path";

const root = join(import.meta.dirname, "..");
// TypeScript はルートの依存に無いので、同じ版を使う @ugoe/runtime (bindings/web) の依存から読む
const ts = createRequire(join(root, "bindings", "web", "package.json"))("typescript");

/** 生成物の既定の場所 (`just wasm` の出力)。 */
export const DEFAULT_GENERATED = "apps/editor/src/lib/viewport/generated/ugoe_wasm.d.ts";
/** 全体の写し。 */
export const FULL_COPY = "bindings/web/stub/ugoe_wasm.d.ts";
/** 一部の写し: ファイル・写しの宣言の名前・生成物のクラスの名前。 */
export const PARTIAL_COPIES = [
  { file: "packages/player/src/wasm.ts", name: "WebPuppet", target: "WebPuppet" },
  { file: "packages/face-tracking/src/frame.ts", name: "TrackingPuppet", target: "WebPuppet" },
];

/**
 * @typedef {{ name: string, type: string, optional: boolean, rest: boolean }} Param
 * @typedef {{ kind: "method" | "property" | "constructor", isStatic: boolean, optional: boolean,
 *   params: Param[], type: string }} Member
 * @typedef {{ kind: "class" | "interface", members: Map<string, Member> }} Container
 * @typedef {{ containers: Map<string, Container>, functions: Map<string, { params: Param[], type: string }>,
 *   aliases: Map<string, string> }} Declarations
 */

const printer = ts.createPrinter({ removeComments: true, newLine: ts.NewLineKind.LineFeed });

/** 型を書き方 (改行・空白・`|` の並び) によらない文字列にする。 */
function typeText(node, sf) {
  if (!node) return "any";
  if (ts.isUnionTypeNode(node)) {
    return node.types
      .map((t) => typeText(t, sf))
      .sort()
      .join(" | ");
  }
  if (ts.isParenthesizedTypeNode(node)) return typeText(node.type, sf);
  return printer.printNode(ts.EmitHint.Unspecified, node, sf).replace(/\s+/g, " ");
}

/** @returns {Param[]} */
function paramsOf(node, sf) {
  return node.parameters.map((p) => ({
    name: p.name.getText(sf),
    type: typeText(p.type, sf),
    optional: p.questionToken !== undefined || p.initializer !== undefined,
    rest: p.dotDotDotToken !== undefined,
  }));
}

/** メンバーの名前 (`[Symbol.dispose]` は書いたとおり)。 */
function memberName(node, sf) {
  if (ts.isConstructorDeclaration(node) || ts.isConstructSignatureDeclaration(node))
    return "constructor";
  return node.name?.getText(sf) ?? "";
}

/** @returns {Member | undefined} */
function memberOf(node, sf) {
  const isStatic = (ts.getCombinedModifierFlags(node) & ts.ModifierFlags.Static) !== 0;
  const optional = node.questionToken !== undefined;
  if (ts.isMethodDeclaration(node) || ts.isMethodSignature(node)) {
    return {
      kind: "method",
      isStatic,
      optional,
      params: paramsOf(node, sf),
      type: typeText(node.type, sf),
    };
  }
  if (ts.isConstructorDeclaration(node)) {
    const isPrivate = (ts.getCombinedModifierFlags(node) & ts.ModifierFlags.Private) !== 0;
    return {
      kind: "constructor",
      isStatic: false,
      optional: false,
      params: paramsOf(node, sf),
      type: isPrivate ? "private" : "public",
    };
  }
  if (ts.isPropertyDeclaration(node) || ts.isPropertySignature(node)) {
    return { kind: "property", isStatic, optional, params: [], type: typeText(node.type, sf) };
  }
  return undefined;
}

/** d.ts / ts の文字列から、トップレベルのクラス・interface・関数・型の別名を読む。 */
export function parseDeclarations(text, fileName = "input.d.ts") {
  const sf = ts.createSourceFile(fileName, text, ts.ScriptTarget.Latest, true);
  /** @type {Declarations} */
  const out = { containers: new Map(), functions: new Map(), aliases: new Map() };
  for (const st of sf.statements) {
    if (ts.isClassDeclaration(st) || ts.isInterfaceDeclaration(st)) {
      if (!st.name) continue;
      const members = new Map();
      for (const m of st.members) {
        const member = memberOf(m, sf);
        if (!member) continue;
        members.set(`${member.isStatic ? "static " : ""}${memberName(m, sf)}`, member);
      }
      out.containers.set(st.name.text, {
        kind: ts.isClassDeclaration(st) ? "class" : "interface",
        members,
      });
    } else if (ts.isFunctionDeclaration(st)) {
      const isDefault = (ts.getCombinedModifierFlags(st) & ts.ModifierFlags.Default) !== 0;
      const name = isDefault ? "default" : (st.name?.text ?? "");
      out.functions.set(name, { params: paramsOf(st, sf), type: typeText(st.type, sf) });
    } else if (ts.isTypeAliasDeclaration(st)) {
      out.aliases.set(st.name.text, typeText(st.type, sf));
    }
  }
  return out;
}

/** 引数と戻り値の型を 1 行で (名前は省く)。 */
function signature(params, type) {
  const ps = params
    .map((p) => `${p.rest ? "..." : ""}${p.optional ? "?" : ""}${p.type}`)
    .join(", ");
  return `(${ps}) => ${type}`;
}

function memberSignature(m) {
  if (m.kind === "property") return m.type;
  return signature(m.params, m.type);
}

/**
 * 写しのメンバーが生成物のメンバーと同じ型か。`partial` なら写しは末尾の省略できる引数を省いてよい。
 * @returns {string | undefined} 違えば説明
 */
function compareMember(copy, generated, partial) {
  if (copy.kind !== generated.kind)
    return `is a ${copy.kind} in the copy but a ${generated.kind} in the generated`;
  if (copy.type !== generated.type) {
    return `type differs: copy ${memberSignature(copy)}, generated ${memberSignature(generated)}`;
  }
  const n = copy.params.length;
  const tooFew =
    n < generated.params.length && !(partial && generated.params.slice(n).every((p) => p.optional));
  const differs =
    n > generated.params.length ||
    tooFew ||
    copy.params.some((p, i) => {
      const g = generated.params[i];
      return g === undefined || p.type !== g.type || p.optional !== g.optional || p.rest !== g.rest;
    });
  if (differs)
    return `type differs: copy ${memberSignature(copy)}, generated ${memberSignature(generated)}`;
  return undefined;
}

/**
 * 全体の写しを生成物と比べる。
 * @param {Declarations} copy
 * @param {Declarations} generated
 * @returns {string[]} 違い
 */
export function compareFull(copy, generated) {
  const problems = [];
  for (const [name, g] of generated.containers) {
    const c = copy.containers.get(name);
    if (!c) {
      problems.push(`${g.kind} ${name}: missing in the copy`);
      continue;
    }
    for (const [key, gm] of g.members) {
      const cm = c.members.get(key);
      if (!cm) {
        // InitOutput は wasm の内部の口 (書き出した関数の一覧) なので、写しに無くてよい
        if (g.kind === "class") problems.push(`${name}.${key}: missing in the copy`);
        continue;
      }
      const diff = compareMember(cm, gm, false);
      if (diff) problems.push(`${name}.${key}: ${diff}`);
    }
    for (const key of c.members.keys()) {
      if (!g.members.has(key))
        problems.push(`${name}.${key}: not in the generated (removed or renamed?)`);
    }
  }
  for (const name of copy.containers.keys()) {
    if (!generated.containers.has(name))
      problems.push(`${name}: not in the generated (removed or renamed?)`);
  }
  for (const [name, g] of generated.functions) {
    const c = copy.functions.get(name);
    if (!c) problems.push(`function ${name}: missing in the copy`);
    else if (signature(c.params, c.type) !== signature(g.params, g.type)) {
      problems.push(
        `function ${name}: type differs: copy ${signature(c.params, c.type)}, generated ${signature(g.params, g.type)}`,
      );
    }
  }
  for (const name of copy.functions.keys()) {
    if (!generated.functions.has(name))
      problems.push(`function ${name}: not in the generated (removed or renamed?)`);
  }
  for (const [name, g] of generated.aliases) {
    const c = copy.aliases.get(name);
    if (c === undefined) problems.push(`type ${name}: missing in the copy`);
    else if (c !== g) problems.push(`type ${name}: differs: copy ${c}, generated ${g}`);
  }
  for (const name of copy.aliases.keys()) {
    if (!generated.aliases.has(name))
      problems.push(`type ${name}: not in the generated (removed or renamed?)`);
  }
  return problems;
}

/**
 * 一部の写し (interface) を生成物のクラスと比べる。
 * @param {Container} copy
 * @param {Container} generated
 * @param {string} label 報告に使う名前
 * @returns {string[]} 違い
 */
export function comparePartial(copy, generated, label) {
  const problems = [];
  for (const [key, cm] of copy.members) {
    const gm = generated.members.get(key);
    if (!gm) {
      if (!cm.optional) problems.push(`${label}.${key}: not in the generated`);
      continue;
    }
    const diff = compareMember(cm, gm, true);
    if (diff) problems.push(`${label}.${key}: ${diff}`);
  }
  return problems;
}

/** ファイルを読んで、全体の写しと一部の写しを `generatedText` と比べる。 */
export function checkAll(generatedText, generatedLabel, { compareFullCopy = true } = {}) {
  const generated = parseDeclarations(generatedText, generatedLabel);
  const problems = [];
  if (compareFullCopy) {
    const full = parseDeclarations(readFileSync(join(root, FULL_COPY), "utf8"), FULL_COPY);
    problems.push(...compareFull(full, generated).map((p) => `${FULL_COPY}: ${p}`));
  }
  for (const { file, name, target } of PARTIAL_COPIES) {
    const copy = parseDeclarations(readFileSync(join(root, file), "utf8"), file).containers.get(
      name,
    );
    const g = generated.containers.get(target);
    if (!copy) problems.push(`${file}: ${name} is not found`);
    else if (!g) problems.push(`${file}: ${target} is not in ${generatedLabel}`);
    else problems.push(...comparePartial(copy, g, name).map((p) => `${file}: ${p}`));
  }
  return problems;
}

function main() {
  const arg = process.argv[2];
  let path = arg ?? DEFAULT_GENERATED;
  let compareFullCopy = true;
  if (!existsSync(join(root, path)) && !existsSync(path)) {
    if (arg !== undefined) {
      console.error(`error: ${arg} is not found (run \`just wasm\` first)`);
      process.exit(1);
    }
    console.log(
      `note: ${DEFAULT_GENERATED} is not generated; comparing the partial copies with ${FULL_COPY}`,
    );
    path = FULL_COPY;
    compareFullCopy = false;
  }
  const abs = existsSync(path) ? path : join(root, path);
  const label = relative(root, abs) || abs;
  const problems = checkAll(readFileSync(abs, "utf8"), label, { compareFullCopy });
  if (problems.length > 0) {
    console.error(`error: the hand-written ugoe-wasm types differ from ${label}:`);
    for (const p of problems) console.error(`  ${p}`);
    console.error(
      "fix the copies (docs/tasks/T0435-wasm-type-copies.md, bindings/web/README.md#開発)",
    );
    process.exit(1);
  }
  console.log(`ugoe-wasm type copies match ${label}`);
}

if (import.meta.main ?? process.argv[1] === import.meta.filename) main();
