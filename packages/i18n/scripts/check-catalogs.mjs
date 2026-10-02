#!/usr/bin/env node
// 翻訳のカタログの検査 (`just i18n`。ADR 0029 の 2、docs/ui.md#文言の書き方)。
//
// `packages/*/locales/` と `apps/*/locales/` の `<言語>.json` (i18next JSON v4) について、次を確かめる。
//
// - ファイルの名前が対応する言語 (src/languages.json) で、原文の言語 (英語) のカタログがある。値はすべて文字列。
// - 原文に無いキーが訳に無い (余り)。常に欠けのない言語 (`complete`。日本語) は、原文のキーがすべてある (欠け)。
//   ほかの言語は欠けてよい (英語を出す)。原文と `complete` の言語は、空の文字列を持たない。
// - 複数形 (`_one`・`_other` など) は、その言語の CLDR の区分 (`Intl.PluralRules`) がそろっている。`_zero` は
//   どの言語でも足してよい (i18next の決まり)。
// - 引数 (`{{name}}`・`{{name, number}}`) の名前が原文と同じ (`count` は省いてよい)。
// - 未使用のキー: パッケージのソース (テストを除く) の文字列に、キーがそのまま現れない。`` `source.${kind}` `` のように
//   テンプレート文字列の頭がキーの頭 (`.` まで) に合えば、使っているとみなす。
//
// `node packages/i18n/scripts/check-catalogs.mjs [ディレクトリ...]`。ディレクトリを省くと上の場所をすべて調べる。

import { existsSync, readdirSync, readFileSync, realpathSync, statSync } from "node:fs";
import { basename, dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../..");
const LANGUAGES = JSON.parse(readFileSync(join(HERE, "../src/languages.json"), "utf8"));

const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/;
const PLACEHOLDER = /\{\{\s*([^,}\s]+)\s*(?:,[^}]*)?\}\}/g;
const SOURCE_EXT = /\.(ts|js|mjs|svelte)$/;
const TEST_FILE = /\.test\.[cm]?[jt]s$/;

/** 入れ子のカタログを `a.b.c` → 値 の Map にする。値が文字列でないものは `errors` に積む。 */
export function flatten(catalog, errors, file, prefix = "") {
  const out = new Map();
  for (const [key, value] of Object.entries(catalog)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (key.includes(".")) errors.push(`${file}: ${path}: キーに "." を使えない (入れ子にする)`);
    if (typeof value === "string") out.set(path, value);
    else if (value && typeof value === "object" && !Array.isArray(value)) {
      for (const [k, v] of flatten(value, errors, file, path)) out.set(k, v);
    } else errors.push(`${file}: ${path}: 値は文字列か入れ子のオブジェクトにする`);
  }
  return out;
}

/** `a.b_one` → { base: "a.b", form: "one" }。複数形でなければ form は null。 */
export function pluralForm(key) {
  const m = PLURAL_SUFFIX.exec(key);
  return m ? { base: key.slice(0, m.index), form: m[1] } : { base: key, form: null };
}

/** 文の中の引数の名前。 */
export function placeholders(text) {
  return new Set([...text.matchAll(PLACEHOLDER)].map((m) => m[1]));
}

/** キーを、元のキー (複数形の接尾辞を除いたもの) → { 区分 → 文 } にまとめる。複数形でないものは区分 `""`。 */
function group(flat) {
  const out = new Map();
  for (const [key, text] of flat) {
    const { base, form } = pluralForm(key);
    if (!out.has(base)) out.set(base, new Map());
    out.get(base).set(form ?? "", text);
  }
  return out;
}

/** ソースの中の文字列 (引用符とテンプレート文字列) と、テンプレート文字列の頭 (`${` の前)。 */
export function sourceStrings(text) {
  const literals = new Set();
  const prefixes = new Set();
  for (const m of text.matchAll(
    /"((?:[^"\\\n]|\\.)*)"|'((?:[^'\\\n]|\\.)*)'|`((?:[^`\\]|\\.)*)`/g,
  )) {
    const value = m[1] ?? m[2] ?? m[3] ?? "";
    if (m[3] !== undefined && value.includes("${")) {
      const prefix = value.slice(0, value.indexOf("${"));
      if (prefix) prefixes.add(prefix);
      // 差し込みの式 (`${t("a.b")}`) の中の文字列も拾う
      const inner = sourceStrings(value);
      for (const l of inner.literals) literals.add(l);
      for (const p of inner.prefixes) prefixes.add(p);
    } else literals.add(value);
  }
  return { literals, prefixes };
}

function sourceFiles(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "locales" || name.startsWith(".")) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...sourceFiles(path));
    else if (SOURCE_EXT.test(name) && !TEST_FILE.test(name)) out.push(path);
  }
  return out;
}

/**
 * 1 つの `locales/` を検査し、エラーの一覧を返す。`sources` はキーを探すソースのディレクトリ
 * (既定は `locales/` の親の `src/`)。
 */
export function checkLocales(localesDir, { languages = LANGUAGES, sources } = {}) {
  const errors = [];
  const name = relative(ROOT, localesDir) || localesDir;
  const tags = new Map(languages.languages.map((l) => [l.tag, l]));
  const catalogs = new Map();
  for (const file of readdirSync(localesDir).filter((f) => f.endsWith(".json"))) {
    const tag = basename(file, ".json");
    const path = `${name}/${file}`;
    if (!tags.has(tag)) {
      errors.push(`${path}: 対応する言語ではない (packages/i18n/src/languages.json に無い)`);
      continue;
    }
    let json;
    try {
      json = JSON.parse(readFileSync(join(localesDir, file), "utf8"));
    } catch (e) {
      errors.push(`${path}: JSON として読めない (${e.message})`);
      continue;
    }
    catalogs.set(tag, group(flatten(json, errors, path)));
  }
  const source = catalogs.get(languages.source);
  if (!source) {
    errors.push(`${name}: 原文のカタログ ${languages.source}.json が無い`);
    return errors;
  }

  for (const [tag, catalog] of catalogs) {
    const path = `${name}/${tag}.json`;
    const isSource = tag === languages.source;
    const complete = isSource || tags.get(tag)?.complete === true;
    const categories = new Set(new Intl.PluralRules(tag).resolvedOptions().pluralCategories);
    for (const [base, forms] of catalog) {
      const original = source.get(base);
      if (!original) {
        errors.push(`${path}: ${base}: 原文 (${languages.source}.json) に無いキー`);
        continue;
      }
      const plural = !original.has("");
      if (plural !== !forms.has("")) {
        errors.push(`${path}: ${base}: 原文と複数形かどうかが違う`);
        continue;
      }
      if (plural) {
        for (const c of categories) {
          if (!forms.has(c)) errors.push(`${path}: ${base}: 複数形の区分 _${c} が無い`);
        }
        for (const f of forms.keys()) {
          if (f !== "zero" && !categories.has(f))
            errors.push(
              `${path}: ${base}_${f}: ${tag} に無い複数形の区分 (${[...categories].join("・")})`,
            );
        }
      }
      const expected = new Set([...original.values()].flatMap((t) => [...placeholders(t)]));
      expected.delete("count");
      for (const [f, text] of forms) {
        const key = f ? `${base}_${f}` : base;
        if (complete && text.trim() === "") errors.push(`${path}: ${key}: 空の文字列`);
        if (isSource || text === "") continue;
        const got = placeholders(text);
        got.delete("count");
        const missing = [...expected].filter((p) => !got.has(p));
        const extra = [...got].filter((p) => !expected.has(p));
        if (missing.length || extra.length) {
          const detail = [
            missing.length ? `無い: ${missing.join(", ")}` : "",
            extra.length ? `原文に無い: ${extra.join(", ")}` : "",
          ];
          errors.push(`${path}: ${key}: 引数が原文と違う (${detail.filter(Boolean).join("。")})`);
        }
      }
    }
    if (complete && !isSource) {
      for (const base of source.keys()) {
        if (!catalog.has(base))
          errors.push(`${path}: ${base}: 訳が無い (${tag} は欠けのない訳にする)`);
      }
    }
  }

  const srcDir = sources ?? join(dirname(localesDir), "src");
  const literals = new Set();
  const prefixes = new Set();
  if (existsSync(srcDir)) {
    for (const file of sourceFiles(srcDir)) {
      const found = sourceStrings(readFileSync(file, "utf8"));
      for (const l of found.literals) literals.add(l);
      for (const p of found.prefixes) prefixes.add(p);
    }
  }
  for (const base of source.keys()) {
    const used =
      literals.has(base) || [...prefixes].some((p) => p.endsWith(".") && base.startsWith(p));
    if (!used) errors.push(`${name}/${languages.source}.json: ${base}: 使われていないキー`);
  }
  return errors;
}

/** 既定で調べる `locales/` (packages/*、apps/*)。 */
export function findLocales(root = ROOT) {
  const out = [];
  for (const parent of ["packages", "apps"]) {
    const dir = join(root, parent);
    if (!existsSync(dir)) continue;
    for (const name of readdirSync(dir).sort()) {
      const locales = join(dir, name, "locales");
      if (existsSync(locales) && statSync(locales).isDirectory()) out.push(locales);
    }
  }
  return out;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === realpathSync(process.argv[1])) {
  const dirs = process.argv.length > 2 ? process.argv.slice(2) : findLocales();
  const errors = dirs.flatMap((dir) => checkLocales(dir));
  for (const e of errors) console.error(`error: ${e}`);
  if (errors.length === 0) console.log(`i18n ok (${dirs.length} catalogs)`);
  process.exitCode = errors.length > 0 ? 1 : 0;
}
