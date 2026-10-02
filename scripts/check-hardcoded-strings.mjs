#!/usr/bin/env node
// 画面の直書きの文言の検査 (`just i18n`。ADR 0029 の 2、docs/ui.md#文言の書き方、docs/translation.md#直書きの文言の検査)。
//
// `apps/*` と `packages/*` の画面の `.svelte` と `.ts` (テスト・設定・生成物を除く) から、カタログ (`t()`) を通さずに
// 表示している文言を探す。Svelte の公式のパーサ (`svelte/compiler` の `parse`) で読み、次のものを見つける。
//
// - マークアップのテキストノード (文字を含むもの)
// - 表示に使う属性 (`title`・`aria-label`・`placeholder`・`alt`・部品の `label` など。DISPLAY_ATTRS) の値の文字列
// - `{...}` で表示する式の、値になる位置の文字列 (`{busy ? "Saving" : "Save"}`。`t("key")` の引数は値ではない)
// - スクリプト (`.svelte` の `<script>` と `.ts`) の文字列のうち、表示に使う場所 (`label: "..."`・`el.title = "..."`・
//   `alert("...")` など) にあるもの、ASCII でない文字 (日本語など) を含むもの、英語の文に見えるもの
//
// 対象の外: 文字を含まない文字列 (記号・数値)、ID・キー・単位のような小文字の識別子 (`dialog-title`・`px`)、比較 (`=== "Escape"`)・`case`・型・
// import の文字列、`console.*` の引数、例外 (`throw`・`new Error(...)`) の文、`i18n-ignore` と書いたコメントの行と次の行。
//
// 移行の途中は、ファイルごとの件数を許す一覧 (`scripts/hardcoded-strings-allowlist.json`) に持つ。一覧より増えたら失敗する。
// 減ったら一覧を縮めるまで失敗する (一覧は減る向きにしか動かない)。
//
//   node scripts/check-hardcoded-strings.mjs           検査する
//   node scripts/check-hardcoded-strings.mjs --update  減ったファイルの件数を一覧に書き戻す (増えた分は書かない)
//   node scripts/check-hardcoded-strings.mjs --list [パス...]  見つけた文言をすべて出す (許す一覧の分も。移行の作業用)

import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..");
const ALLOWLIST = join(HERE, "hardcoded-strings-allowlist.json");

// svelte はワークスペースのルートに無いので、svelte を devDependencies に持つ @ugoe/i18n から読む
const { parse } = createRequire(join(ROOT, "packages/i18n/package.json"))("svelte/compiler");

/** 値が表示される属性 (HTML の属性と、部品の props)。 */
export const DISPLAY_ATTRS = new Set([
  "title",
  "alt",
  "placeholder",
  "label",
  "aria-label",
  "aria-description",
  "aria-placeholder",
  "aria-roledescription",
  "aria-valuetext",
  "tooltip",
  "description",
  "hint",
  "heading",
  "caption",
  "message",
  "text",
]);
/** スクリプトの中で値が表示されるオブジェクトのプロパティ。 */
const DISPLAY_PROPS = new Set([
  "title",
  "alt",
  "placeholder",
  "label",
  "ariaLabel",
  "tooltip",
  "description",
  "hint",
  "heading",
  "caption",
  "message",
  "text",
]);
/** 代入すると表示される DOM のプロパティ。 */
const DISPLAY_DOM_PROPS = new Set([
  "textContent",
  "innerText",
  "innerHTML",
  "title",
  "alt",
  "placeholder",
  "ariaLabel",
  "label",
]);
/** 引数を表示する関数 (ブラウザの `alert` など、Tauri の dialog の `message`・`ask`)。 */
const DISPLAY_CALLS = new Set(["alert", "confirm", "prompt", "message", "ask"]);
/** 中を見ない要素。 */
const SKIP_ELEMENTS = new Set(["style", "script"]);

const LETTER = /\p{L}/u;
const NON_ASCII_LETTER = /[^\P{L}\p{ASCII}]/u; // ASCII でない文字 (かな・漢字・é など)
/** ID・キー・クラス名のような小文字の識別子 (`dialog-title`・`editor.save.label`・`image/png`)。 */
const IDENTIFIER = /^[a-z0-9]+(?:[._:/-][a-z0-9]+)*$/;
/** 英語の文に見えるもの: 大文字で始まる語の後に、空白で区切った語が 1 つ以上続く (`Warp deformer`・`Open the file.`)。 */
const SENTENCE = /^\p{Lu}[\p{L}'’]*(?:,? [\p{L}\p{N}'’()…-]+)+[.!?…:]?$/u;

/** スクリプトの文字列が直書きの文言かどうか。`display` は表示に使う場所にあるか。 */
export function isHardcoded(text, display) {
  const s = text.replace(/\s+/g, " ").trim();
  if (!LETTER.test(s)) return false;
  if (display) return !IDENTIFIER.test(s);
  return NON_ASCII_LETTER.test(s) || SENTENCE.test(s);
}

/** マークアップのテキストノードが直書きの文言かどうか。値の名前・単位のような小文字の 1 語 (`yaw`・`px`) は除く。 */
export function isHardcodedText(text) {
  const s = text.replace(/\s+/g, " ").trim();
  return LETTER.test(s) && !IDENTIFIER.test(s);
}

const COMPARISON = new Set(["==", "===", "!=", "!=="]);

function calleeName(callee) {
  if (!callee) return null;
  if (callee.type === "Identifier") return callee.name;
  if (callee.type === "MemberExpression" && !callee.computed) return callee.property.name;
  return null;
}

function isConsole(callee) {
  return (
    callee?.type === "MemberExpression" &&
    callee.object.type === "Identifier" &&
    callee.object.name === "console"
  );
}

function propertyName(key) {
  if (key.type === "Identifier") return key.name;
  if (key.type === "Literal") return String(key.value);
  return null;
}

/** 表示に使う式の中で、値になる (表示される) 子。 */
function valueChildren(node) {
  switch (node.type) {
    case "ConditionalExpression":
      return [node.consequent, node.alternate];
    case "LogicalExpression":
      return node.operator === "&&" ? [node.right] : [node.left, node.right];
    case "BinaryExpression":
      return node.operator === "+" ? [node.left, node.right] : [];
    case "TemplateLiteral":
      return node.expressions;
    case "SequenceExpression":
      return [node.expressions.at(-1)];
    case "ArrowFunctionExpression":
      return node.body.type === "BlockStatement" ? [] : [node.body];
    case "TSAsExpression":
    case "TSSatisfiesExpression":
    case "TSNonNullExpression":
    case "ParenthesizedExpression":
      return [node.expression];
    default:
      return [];
  }
}

const SKIP_KEYS = new Set(["type", "start", "end", "loc", "metadata", "name_loc", "parent"]);

/**
 * 1 つのファイルの直書きの文言を探す。`kind` は "svelte" か "ts"。
 * @returns {{ line: number, kind: string, text: string }[]}
 */
export function scan(source, kind) {
  // .ts は <script lang="ts"> に包んで同じパーサで読む (位置は包みの分だけずらす)
  const prefix = kind === "ts" ? '<script lang="ts">' : "";
  const ast = parse(kind === "ts" ? `${prefix}${source}</script>` : source, { modern: true });
  const lineStarts = [0];
  for (let i = 0; i < source.length; i++) if (source[i] === "\n") lineStarts.push(i + 1);
  const lineOf = (offset) => {
    const pos = offset - prefix.length;
    let lo = 0;
    let hi = lineStarts.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (lineStarts[mid] <= pos) lo = mid;
      else hi = mid - 1;
    }
    return lo + 1;
  };
  const ignored = new Set();
  source.split("\n").forEach((text, i) => {
    if (text.includes("i18n-ignore")) {
      ignored.add(i + 1);
      ignored.add(i + 2);
    }
  });

  const found = [];
  const report = (node, kindName, text) => {
    const line = lineOf(node.start);
    if (ignored.has(line)) return;
    found.push({ line, kind: kindName, text: text.replace(/\s+/g, " ").trim() });
  };

  /** 文字列のノードを検査する。 */
  const literal = (node, display, label) => {
    if (node.type === "Literal" && typeof node.value === "string") {
      if (isHardcoded(node.value, display)) report(node, label, node.value);
      return true;
    }
    if (node.type === "TemplateLiteral") {
      const text = node.quasis.map((q) => q.value.cooked ?? q.value.raw).join("0");
      if (isHardcoded(text, display)) report(node, label, text);
      return false; // 差し込みの式は続けて見る
    }
    return false;
  };

  /** ESTree とスクリプトを歩く。 */
  const visit = (node, display, label) => {
    if (!node || typeof node !== "object") return;
    if (Array.isArray(node)) {
      for (const n of node) visit(n, display, label);
      return;
    }
    if (typeof node.type !== "string") return;
    if (node.type.startsWith("TS") && !valueChildren(node).length) return; // 型
    if (literal(node, display, label)) return;

    switch (node.type) {
      case "ImportDeclaration":
      case "ExportAllDeclaration":
      case "ImportExpression":
      case "ThrowStatement":
        return;
      case "ExportNamedDeclaration":
        visit(node.declaration, false, label);
        return;
      case "ExpressionStatement":
        if (node.directive) return;
        break;
      case "CallExpression":
      case "NewExpression": {
        const name = calleeName(node.callee);
        if (isConsole(node.callee) || name?.endsWith("Error")) return;
        visit(node.callee, false, label);
        if (name === "setAttribute" && node.arguments[0]?.type === "Literal") {
          visit(node.arguments[1], DISPLAY_ATTRS.has(node.arguments[0].value), label);
          return;
        }
        visit(node.arguments, DISPLAY_CALLS.has(name), label);
        return;
      }
      case "BinaryExpression":
        if (COMPARISON.has(node.operator)) {
          for (const side of [node.left, node.right]) {
            if (side.type !== "Literal" && side.type !== "TemplateLiteral")
              visit(side, false, label);
          }
          return;
        }
        break;
      case "SwitchCase":
        visit(node.consequent, false, label);
        return;
      case "MemberExpression":
        visit(node.object, false, label);
        if (node.computed && node.property.type !== "Literal") visit(node.property, false, label);
        return;
      case "Property":
        if (node.computed) visit(node.key, false, label);
        visit(node.value, DISPLAY_PROPS.has(propertyName(node.key)), label);
        return;
      case "MethodDefinition":
      case "PropertyDefinition":
        visit(node.value, false, label);
        return;
      case "AssignmentExpression":
        visit(node.left, false, label);
        visit(
          node.right,
          node.left.type === "MemberExpression" &&
            !node.left.computed &&
            DISPLAY_DOM_PROPS.has(node.left.property.name),
          label,
        );
        return;
    }

    const values = display ? new Set(valueChildren(node)) : new Set();
    for (const [key, child] of Object.entries(node)) {
      if (SKIP_KEYS.has(key) || !child || typeof child !== "object") continue;
      for (const c of Array.isArray(child) ? child : [child]) visit(c, values.has(c), label);
    }
  };

  /** Svelte のマークアップを歩く。 */
  const markup = (node) => {
    if (!node || typeof node !== "object") return;
    if (Array.isArray(node)) {
      for (const n of node) markup(n);
      return;
    }
    switch (node.type) {
      case "Fragment":
        for (const child of node.nodes) {
          if (child.type === "Text") {
            if (isHardcodedText(child.data)) report(child, "text", child.data);
          } else if (child.type === "ExpressionTag" || child.type === "HtmlTag") {
            visit(child.expression, true, "expression");
          } else markup(child);
        }
        return;
      case "Comment":
        return;
      case "Attribute": {
        const display = DISPLAY_ATTRS.has(node.name);
        const label = display ? `attr ${node.name}` : "script";
        if (node.value === true) return;
        for (const part of Array.isArray(node.value) ? node.value : [node.value]) {
          if (part.type === "Text") {
            if (display && isHardcoded(part.data, true)) report(part, label, part.data);
          } else visit(part.expression, display, label);
        }
        return;
      }
      case "RegularElement":
        if (SKIP_ELEMENTS.has(node.name)) return;
        break;
    }
    for (const [key, child] of Object.entries(node)) {
      if (SKIP_KEYS.has(key) || !child || typeof child !== "object") continue;
      if (typeof child.type === "string" && !isMarkup(child)) visit(child, false, "script");
      else if (Array.isArray(child)) {
        for (const c of child) {
          if (isMarkup(c)) markup(c);
          else visit(c, false, "script");
        }
      } else markup(child);
    }
  };

  if (ast.instance) visit(ast.instance.content, false, "script");
  if (ast.module) visit(ast.module.content, false, "script");
  if (kind === "svelte") markup(ast.fragment);
  return found;
}

/** Svelte のマークアップのノード (ESTree ではないもの) か。 */
function isMarkup(node) {
  return (
    node &&
    typeof node === "object" &&
    (node.type === "Fragment" ||
      node.type === "Text" ||
      node.type === "Comment" ||
      node.type === "Attribute" ||
      node.type === "SpreadAttribute" ||
      node.type === "ExpressionTag" ||
      node.type === "HtmlTag" ||
      node.type === "ConstTag" ||
      node.type === "DebugTag" ||
      node.type === "RenderTag" ||
      node.type === "AttachTag" ||
      node.type.endsWith("Element") ||
      node.type.endsWith("Block") ||
      node.type.endsWith("Directive") ||
      node.type === "Component" ||
      node.type.startsWith("Svelte") ||
      node.type === "TitleElement" ||
      node.type === "SlotElement")
  );
}

// ---- 対象のファイル ----

const SKIP_DIRS = new Set([
  "node_modules",
  "dist",
  "src-tauri",
  "generated",
  "locales",
  "scripts",
  "test",
  "tests",
  "pkg",
  "gallery", // @ugoe/ui の部品の見本 (開発者向け。見本の文言は gallery/strings.ts に持つ)
]);
const TARGET_FILE = /\.(svelte|ts)$/;
const NOT_SCREEN = /(\.test\.ts|\.d\.ts|\.config\.ts)$/;

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    if (name.startsWith(".") || SKIP_DIRS.has(name)) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...walk(path));
    else if (TARGET_FILE.test(name) && !NOT_SCREEN.test(name)) out.push(path);
  }
  return out;
}

/** 検査するアプリ・パッケージ (`apps/<名前>` と `packages/<名前>`)。 */
export function targets(root = ROOT) {
  const out = [];
  for (const parent of ["apps", "packages"]) {
    const dir = join(root, parent);
    if (!existsSync(dir)) continue;
    for (const name of readdirSync(dir).sort()) {
      if (statSync(join(dir, name)).isDirectory()) out.push(`${parent}/${name}`);
    }
  }
  return out;
}

/** すべての対象を検査し、ファイル (ルートからの相対パス) → 見つけた文言 の Map を返す。 */
export function scanAll(root = ROOT) {
  const out = new Map();
  for (const target of targets(root)) {
    for (const path of walk(join(root, target))) {
      const file = relative(root, path).split("\\").join("/");
      const found = scan(readFileSync(path, "utf8"), path.endsWith(".svelte") ? "svelte" : "ts");
      if (found.length) out.set(file, found);
    }
  }
  return out;
}

/** 許す一覧 ({ 対象: { ファイル: 件数 } }) を ファイル → 件数 にする。 */
function flattenAllowlist(allowlist) {
  const out = new Map();
  for (const files of Object.values(allowlist)) {
    for (const [file, count] of Object.entries(files)) out.set(file, count);
  }
  return out;
}

/**
 * 見つけた文言と許す一覧を比べる。`errors` は増えたもの (失敗)、`shrunk` は減ったもの (一覧を縮める)。
 * @param {Map<string, {line: number, kind: string, text: string}[]>} found
 */
export function compare(found, allowlist) {
  const allowed = flattenAllowlist(allowlist);
  const errors = [];
  const shrunk = [];
  for (const [file, items] of found) {
    const limit = allowed.get(file) ?? 0;
    if (items.length > limit) {
      errors.push(`${file}: 直書きの文言が ${items.length} 件 (許す一覧は ${limit} 件)`);
      for (const f of items) errors.push(`  ${file}:${f.line}: ${f.kind}: ${short(f.text)}`);
    } else if (items.length < limit) shrunk.push({ file, from: limit, to: items.length });
  }
  for (const [file, limit] of allowed) {
    if (!found.has(file) && limit > 0) shrunk.push({ file, from: limit, to: 0 });
  }
  return { errors, shrunk };
}

/** 一覧を縮める (増えた分は書かない)。 */
export function shrinkAllowlist(found, allowlist) {
  const out = {};
  for (const [target, files] of Object.entries(allowlist)) {
    const next = {};
    for (const [file, count] of Object.entries(files)) {
      const now = Math.min(count, found.get(file)?.length ?? 0);
      if (now > 0) next[file] = now;
    }
    if (Object.keys(next).length) out[target] = next;
  }
  return out;
}

function short(text) {
  return text.length > 80 ? `${text.slice(0, 77)}...` : text;
}

function main(args) {
  const found = scanAll();
  if (args[0] === "--list") {
    const filters = args.slice(1);
    for (const [file, items] of found) {
      if (filters.length && !filters.some((f) => file.startsWith(f))) continue;
      for (const f of items) console.log(`${file}:${f.line}: ${f.kind}: ${short(f.text)}`);
    }
    return 0;
  }
  let allowlist = JSON.parse(readFileSync(ALLOWLIST, "utf8"));
  const { errors, shrunk } = compare(found, allowlist);
  const increased = errors.length > 0;
  if (args[0] === "--update") {
    allowlist = shrinkAllowlist(found, allowlist);
    writeFileSync(ALLOWLIST, `${JSON.stringify(allowlist, null, 2)}\n`);
    for (const s of shrunk) console.log(`${s.file}: ${s.from} → ${s.to}`);
  } else if (shrunk.length) {
    for (const s of shrunk) {
      errors.push(
        `${s.file}: 直書きの文言が ${s.from} 件から ${s.to} 件に減った (許す一覧を縮める)`,
      );
    }
    errors.push(
      "減った分は `node scripts/check-hardcoded-strings.mjs --update` で scripts/hardcoded-strings-allowlist.json に書き戻す",
    );
  }
  if (errors.length) {
    console.error(errors.join("\n"));
    if (increased) {
      console.error(
        "\n直書きの文言はカタログに置き、t() で出す (docs/translation.md#直書きの文言の検査)。" +
          "訳さない文字列は、その行か前の行に `i18n-ignore` のコメントを書く",
      );
    }
    return 1;
  }
  const total = [...flattenAllowlist(allowlist).values()].reduce((a, b) => a + b, 0);
  console.log(`直書きの文言: 新しいものなし (移行を待つもの ${total} 件)`);
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main(process.argv.slice(2));
}
