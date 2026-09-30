#!/usr/bin/env node
// 公開仕様 (`spec/`) の検査。`just spec` (と `just ci`) から呼ぶ。方針は docs/spec/public-spec.md#同期の検査。
//
// 1. 例のファイル (`spec/examples/`) が JSON Schema (`spec/schema/`) を満たす。
// 2. `spec/model.md` の型の説明 (フィールドの表・enum の値) が JSON Schema と一致する
//    (JSON Schema は Rust の型から生成するので、コードとの一致の検査になる)。
// 3. 機能 ID の表が `spec/bundle.md`・`docs/spec/file-formats.md`・`ugoe_format::features` で一致し、
//    未知の値のフォールバックの表の enum が `spec/model.md` と `docs/spec/file-formats.md` で一致する。
// 4. 対応表 (`docs/spec/public-spec.md`) が両方の文書の節を覆う。
// 5. `spec/README.md` の文書の一覧が `spec/*.md` と一致する。
//
// 見出しとリンクの先の存在は check-md-links.mjs が検査する。

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import { headings, stripCodeBlocks } from "./lib/markdown.mjs";

const root = resolve(dirname(new URL(import.meta.url).pathname), "..");
const errors = [];
const read = (path) => readFileSync(join(root, path), "utf8");
const readJson = (path) => JSON.parse(read(path));

// ---- 1. 例のファイルと JSON Schema ----

const SCHEMAS = ["model", "module", "physics", "motion", "expression"];
const schemas = Object.fromEntries(
  SCHEMAS.map((name) => [name, readJson(`spec/schema/${name}.schema.json`)]),
);

function checkExamples() {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  // schemars が付ける数値の format。範囲は minimum / maximum でも書かれているが、ここでも確かめる。
  const ranges = { uint8: 2 ** 8, uint16: 2 ** 16, uint32: 2 ** 32, uint64: 2 ** 64 };
  for (const [name, limit] of Object.entries(ranges)) {
    ajv.addFormat(name, {
      type: "number",
      validate: (x) => Number.isInteger(x) && x >= 0 && x < limit,
    });
  }
  ajv.addFormat("float", { type: "number", validate: (x) => Math.abs(x) <= 3.4028234663852886e38 });
  const validators = Object.fromEntries(SCHEMAS.map((name) => [name, ajv.compile(schemas[name])]));

  const dir = join(root, "spec/examples");
  let count = 0;
  for (const example of readdirSync(dir, { withFileTypes: true })) {
    if (!example.isDirectory()) continue;
    for (const file of readdirSync(join(dir, example.name)).sort()) {
      // model.json・module.json・*.physics.json などを、名前から決まるスキーマで検証する
      const kind = /^(?:.*\.)?(model|module|physics|motion|expression)\.json$/.exec(file)?.[1];
      if (!kind) continue;
      const path = `spec/examples/${example.name}/${file}`;
      if (!validators[kind](readJson(path))) {
        for (const e of validators[kind].errors) {
          errors.push(`${path}: ${e.instancePath || "/"} ${e.message} (${kind}.schema.json)`);
        }
      }
      count++;
    }
  }
  if (count === 0) errors.push("spec/examples: 検証するファイルがない");
}

// ---- 2. spec/model.md の型と JSON Schema ----

/** 生成した JSON Schema の型定義 (ルートの型を含む)。`KeyformGrid2` などの番号付きは `KeyformGrid` にまとめる。 */
function schemaTypes() {
  const types = new Map();
  const roots = {
    model: "Model",
    module: "Module",
    physics: "PhysicsSettings",
    motion: "Motion",
    expression: "Expression",
  };
  for (const [file, schema] of Object.entries(schemas)) {
    const { $defs = {}, ...rootSchema } = schema;
    types.set(roots[file], rootSchema);
    for (const [name, def] of Object.entries($defs)) {
      const base = name.replace(/^(KeyformGrid)\d+$/, "$1");
      if (!types.has(base)) types.set(base, def);
    }
  }
  return types;
}

/** `spec/model.md` のうち、型を説明していないもの (JSON Schema の補助の型)。 */
const NOT_DOCUMENTED = new Map([
  ["Vec2", "vec2 は Data conventions の表で定義する"],
  ["Rgb", "rgb は Data conventions の表で定義する"],
]);

/** 型名の行がなく、見出しの直後の表で説明している型。 */
const DOCUMENTED_UNDER_HEADING = new Map([
  ["Model", "Model"],
  ["WarpDeformer", "Warp deformers"],
  ["RotationDeformer", "Rotation deformers"],
]);

const PRIMITIVES = new Set([
  "f32",
  "u8",
  "u16",
  "u32",
  "u64",
  "bool",
  "string",
  "id",
  "vec2",
  "rgb",
  "none",
  "null",
]);

/** 表の行をセルに分ける。 */
const cells = (line) =>
  line
    .trim()
    .replace(/^\||\|$/g, "")
    .split("|")
    .map((c) => c.trim());

/** セルの最初の `code` (なければ null)。 */
const firstCode = (cell) => /`([^`]+)`/.exec(cell)?.[1] ?? null;

/** 文の中の `code` のうち、フィールド名や値に見えるもの (小文字で始まり、型名ではない)。`{ "mesh": … }` はキー。 */
const codeNames = (text) =>
  [...text.matchAll(/`([^`]+)`/g)]
    .map((m) => /^\{\s*"(\w+)"/.exec(m[1])?.[1] ?? m[1])
    .filter((c) => /^[a-z][a-z0-9_]*$/.test(c) && !PRIMITIVES.has(c));

/**
 * `spec/model.md` から型の説明を集める: 型名 → `{ line, text, header, table }`。
 *
 * - 型名の行: 「`TypeName`:」「`TypeName` (…):」「`TypeName` — …:」「`TypeName` is …」 (箇条書きでもよい)。
 *   その後の段落 (`text`) と、段落の直後の表 (`header` と行の `table`) を説明とする。
 * - 見出しの直後の表 ([`DOCUMENTED_UNDER_HEADING`])。
 * - 文の中の「`TypeName` is `{ "a": …, "b": … }`」。
 * - 表の 1 列目が型名の行 (「| `AvatarPermission` | `only_author`, … |」)。
 * - どれもなければ、フィールドの表でその型を持つ行の説明の列 (「| `op` | `ExpressionOp` | — | `add`, … |」)。
 */
function documentedTypes(file) {
  const lines = stripCodeBlocks(read(file)).split("\n");
  const docs = new Map();
  const isRow = (line) => line.trimStart().startsWith("|");
  const collect = (start) => {
    let i = start;
    const text = [];
    while (i < lines.length && lines[i].trim() !== "" && !isRow(lines[i])) text.push(lines[i++]);
    while (i < lines.length && lines[i].trim() === "") i++;
    if (i < lines.length && isRow(lines[i])) {
      const header = cells(lines[i]);
      const rows = [];
      for (i += 2; i < lines.length && isRow(lines[i]); i++) rows.push(cells(lines[i]));
      return { text: text.join(" "), header, table: rows };
    }
    return { text: text.join(" "), header: null, table: null };
  };
  const add = (name, doc) => {
    if (!docs.has(name)) docs.set(name, doc);
  };
  lines.forEach((line, index) => {
    const m = /^\s*(?:- )?`([A-Z][A-Za-z0-9]*)(?:<[^>]*>)?`(?=:| \(| —| is |,)/.exec(line);
    if (m) add(m[1], { line: index + 1, ...collect(index) });
    for (const inline of line.matchAll(/`([A-Z][A-Za-z0-9]*)` is `\{([^`]*)\}`/g)) {
      add(inline[1], {
        line: index + 1,
        text: inline[2].replace(/"(\w+)"/g, "`$1`"),
        header: null,
        table: null,
      });
    }
  });
  for (const [type, heading] of DOCUMENTED_UNDER_HEADING) {
    const h = headings(join(root, file)).find((h) => h.text === heading);
    if (!h) {
      errors.push(`${file}: 見出し「${heading}」がない (${type} の説明)`);
      continue;
    }
    let i = h.line;
    while (i < lines.length && !isRow(lines[i]) && !/^#/.test(lines[i])) i++;
    add(type, { line: h.line, ...collect(i) });
  }
  const fallbacks = [];
  lines.forEach((line, index) => {
    if (!isRow(line)) return;
    const row = cells(line);
    const first = /^`([A-Z][A-Za-z0-9]*)`$/.exec(row[0]);
    if (first && row.length >= 2)
      add(first[1], { line: index + 1, text: row[1], header: null, table: null });
    const type = /^`([A-Z][A-Za-z0-9]*)\??`$/.exec(row[1] ?? "");
    if (type && row.length >= 4) {
      fallbacks.push([type[1], { line: index + 1, text: row[3], header: null, table: null }]);
    }
  });
  for (const [name, doc] of fallbacks) add(name, doc);
  return docs;
}

/** 列挙の値: 文字列の enum か、タグ付きの oneOf のタグの値。 */
function enumShape(def) {
  const variants = def.oneOf ?? def.anyOf;
  if (def.enum) return { kind: "string", values: def.enum };
  if (!variants) return null;
  if (variants.every((v) => v.enum || v.const !== undefined)) {
    return { kind: "string", values: variants.flatMap((v) => v.enum ?? [v.const]) };
  }
  // タグ付き (`type`・`method`・`kind`) か、1 つのキーのオブジェクト
  const tags = ["type", "method", "kind"];
  const tag = tags.find((t) => variants.every((v) => v.properties?.[t]?.const !== undefined));
  if (tag) {
    return {
      kind: "tagged",
      tag,
      variants: variants.map((v) => ({
        value: v.properties[tag].const,
        ref: v.$ref?.replace("#/$defs/", "") ?? null,
        fields: Object.keys(v.properties).filter((k) => k !== tag),
      })),
    };
  }
  if (variants.every((v) => v.type === "object" && Object.keys(v.properties ?? {}).length === 1)) {
    return { kind: "single-key", values: variants.map((v) => Object.keys(v.properties)[0]) };
  }
  return null;
}

const describe = (list) => (list.length === 0 ? "(なし)" : list.join(", "));

function compare(where, what, documented, actual) {
  const missing = actual.filter((x) => !documented.includes(x));
  const extra = documented.filter((x) => !actual.includes(x));
  if (missing.length > 0) errors.push(`${where}: ${what} に書かれていない: ${describe(missing)}`);
  if (extra.length > 0)
    errors.push(`${where}: ${what} にあるがコード (JSON Schema) にない: ${describe(extra)}`);
}

function checkModelTypes() {
  const file = "spec/model.md";
  const types = schemaTypes();
  const docs = documentedTypes(file);
  for (const [name, def] of types) {
    if (NOT_DOCUMENTED.has(name)) continue;
    const doc = docs.get(name);
    if (!doc) {
      // タグ付きの enum の中身として説明される型 (`DepthRotation` など) は enum の側で検査する
      const referenced = [...types.values()].some((d) =>
        (d.oneOf ?? []).some((v) => v.$ref === `#/$defs/${name}`),
      );
      if (!referenced)
        errors.push(`${file}: 型 ${name} の説明 (「\`${name}\`:」とフィールドの表) がない`);
      continue;
    }
    const where = `${file}:${doc.line} ${name}`;
    const shape = enumShape(def);
    if (def.properties && !shape) {
      const actual = Object.keys(def.properties);
      const documented =
        doc.table && doc.header[0] === "Field"
          ? doc.table.map((row) => firstCode(row[0])).filter(Boolean)
          : codeNames(doc.text);
      compare(where, "フィールドの一覧", documented, actual);
    } else if (shape?.kind === "string" || shape?.kind === "single-key") {
      const documented =
        doc.table && ["Value", "`kind`"].includes(doc.header[0])
          ? doc.table.map((row) => firstCode(row[0])).filter(Boolean)
          : codeNames(doc.text);
      compare(where, "値の一覧", documented, shape.values);
    } else if (shape?.kind === "tagged") {
      if (!doc.table || doc.header[0] !== `\`${shape.tag}\``) {
        errors.push(`${where}: \`${shape.tag}\` の値の表がない`);
        continue;
      }
      const rows = new Map(doc.table.map((row) => [firstCode(row[0]), row]));
      compare(
        where,
        `\`${shape.tag}\` の値の一覧`,
        [...rows.keys()],
        shape.variants.map((v) => v.value),
      );
      const further = doc.header.indexOf("Further fields");
      const idColumn = doc.header.indexOf("`id`");
      for (const variant of shape.variants) {
        const row = rows.get(variant.value);
        if (!row) continue;
        const variantWhere = `${where} ${shape.tag} = ${variant.value}`;
        if (further >= 0) {
          const text = row[further];
          const refType = firstCode(text);
          if (variant.ref && refType === variant.ref) continue;
          const documented = [...text.matchAll(/`([a-z][a-z0-9_]*)`/g)].map((m) => m[1]);
          let actual = variant.fields;
          if (variant.ref) {
            actual = [...actual, ...Object.keys(types.get(variant.ref)?.properties ?? {})];
          }
          // 説明の列には型や値も書かれるので、コードのフィールドがすべて書かれていることだけを確かめる
          const missing = actual.filter((x) => !documented.includes(x));
          if (missing.length > 0)
            errors.push(`${variantWhere}: フィールドが書かれていない: ${describe(missing)}`);
        } else if (idColumn >= 0) {
          compare(variantWhere, "フィールド", ["id"], variant.fields);
        }
      }
    }
  }
  for (const [name, doc] of docs) {
    if (!types.has(name) && !NOT_DOCUMENTED.has(name)) {
      // 説明はあるがスキーマにない型。表を持つものだけを誤りとする (文の中で型名から始まる行を除く)
      if (doc.table && doc.header[0] === "Field")
        errors.push(`${file}:${doc.line}: 型 ${name} はコード (JSON Schema) にない`);
    }
  }
}

// ---- 3. 機能 ID と未知の値の表 ----

/** 見出し (`heading`) の後の最初の表の、1 列目の `code` の一覧。 */
function tableAfter(file, heading, column = 0) {
  const lines = stripCodeBlocks(read(file)).split("\n");
  const start = lines.findIndex(
    (l) => /^#{1,6}\s/.test(l) && l.replace(/^#+\s+/, "").trim() === heading,
  );
  if (start < 0) {
    errors.push(`${file}: 見出し「${heading}」がない`);
    return [];
  }
  let i = start + 1;
  while (i < lines.length && !lines[i].startsWith("|")) i++;
  const rows = [];
  for (i += 2; i < lines.length && lines[i].startsWith("|"); i++) rows.push(cells(lines[i]));
  return rows.map((row) => row[column]).filter((c) => c !== undefined);
}

function checkFeatures() {
  const code = [
    ...read("crates/ugoe-format/src/features.rs").matchAll(/^\s*id: "([^"]+)",$/gm),
  ].map((m) => m[1]);
  const spec = [
    ...tableAfter("spec/bundle.md", "Baseline features"),
    ...tableAfter("spec/bundle.md", "Features after 1.0"),
  ]
    .map(firstCode)
    .filter(Boolean);
  const docs = [
    ...tableAfter("docs/spec/file-formats.md", "機能とフォールバックの一覧"),
    ...tableAfter("docs/spec/file-formats.md", "1.0 以降の機能"),
  ]
    .map(firstCode)
    .filter(Boolean);
  compare("spec/bundle.md", "機能 ID の表", spec, code);
  compare("docs/spec/file-formats.md", "機能 ID の表", docs, code);

  // 未知の値のフォールバック: enum の名前の集合
  const specEnums = tableAfter("spec/model.md", "Unknown values", 1).flatMap((c) =>
    [...c.matchAll(/`([A-Z]\w*)`/g)].map((m) => m[1]),
  );
  const docsEnums = tableAfter("docs/spec/file-formats.md", "未知の値の読み込み", 0).flatMap((c) =>
    [...c.matchAll(/\(([^)]*)\)/g)].flatMap((m) =>
      [...m[1].matchAll(/`([A-Z]\w*)`/g)].map((n) => n[1]),
    ),
  );
  compare("docs/spec/file-formats.md#未知の値の読み込み", "enum の一覧", docsEnums, specEnums);
  // 拡張されうる enum (JSON Schema で enum の形のもの) はすべて表にある
  const types = schemaTypes();
  const enums = [...types].filter(([, def]) => enumShape(def)).map(([name]) => name);
  const notExtensible = ["VertexOwner"];
  compare(
    "spec/model.md#unknown-values",
    "フォールバックの表",
    [...specEnums, ...notExtensible],
    enums,
  );
}

// ---- 4. 対応表 ----

/** 対応表の節: `{ heading, rows: [{ left: [リンク], right: [リンク] }] }`。リンクは root からのパスとアンカー。 */
function correspondence(file) {
  const lines = stripCodeBlocks(read(file)).split("\n");
  const sections = [];
  let current = null;
  const links = (cell) =>
    [...cell.matchAll(/\]\(([^)\s]+)\)/g)]
      .map((m) => m[1])
      .filter((l) => !/^[a-z]+:/.test(l))
      .map((l) => {
        const [path, anchor] = l.split("#");
        const target = relative(root, resolve(dirname(join(root, file)), decodeURI(path)));
        return { path: target, anchor: anchor === undefined ? null : decodeURIComponent(anchor) };
      });
  lines.forEach((line, index) => {
    const h = /^##\s+(.*)$/.exec(line);
    if (h) {
      current = { heading: h[1], rows: [], line: index + 1, excluded: [] };
      sections.push(current);
      return;
    }
    if (!current) return;
    if (line.startsWith("|") && !/^\|\s*-/.test(line)) {
      const [left = "", right = ""] = cells(line);
      current.rows.push({ left: links(left), right: links(right) });
    } else if (/^\s*-\s/.test(line)) {
      current.excluded.push(...links(line));
    }
  });
  return sections;
}

/** 表の行に対応を書かない節 (公開仕様に含めないものなど) を箇条書きで挙げる、対応表の節。 */
const EXCLUDED_SECTION = "表の行にない節";

/** まだ英語版がない文書の表の節。ここに挙げた `docs/spec/` の文書は、節の対応を検査しない。 */
const PENDING_SECTION = "まだ公開仕様にない文書";

function checkCorrespondence() {
  const file = "docs/spec/public-spec.md";
  const sections = correspondence(file);
  const left = sections.flatMap((s) => s.rows.flatMap((r) => r.left));
  const right = sections.flatMap((s) => s.rows.flatMap((r) => r.right));
  const excluded = sections
    .filter((s) => s.heading === EXCLUDED_SECTION)
    .flatMap((s) => s.excluded);
  const pending = new Set(
    sections
      .filter((s) => s.heading === PENDING_SECTION)
      .flatMap((s) => s.rows.flatMap((r) => r.left.map((l) => l.path))),
  );

  // spec/*.md: 対応表に出てくる文書の `##` 見出しは、それか配下の見出しが左の列にある
  const specDocs = readdirSync(join(root, "spec")).filter(
    (f) => f.endsWith(".md") && f !== "README.md",
  );
  for (const doc of specDocs) {
    const path = `spec/${doc}`;
    const linked = left.filter((l) => l.path === path);
    if (linked.length === 0) {
      errors.push(`${file}: ${path} の節の対応がない`);
      continue;
    }
    coverSections(path, linked, `${file} の左の列`);
  }

  // docs/spec/*.md: どれも対応表に出てくる。英語版がある文書のうち、節へのリンクがあるものは、
  // その `##` 見出しがどれも右の列か「表の行にない節」にある
  const docsDocs = readdirSync(join(root, "docs/spec")).filter(
    (f) => f.endsWith(".md") && f !== "public-spec.md",
  );
  const mentioned = new Set([...right, ...left, ...excluded].map((l) => l.path));
  for (const doc of docsDocs) {
    const path = `docs/spec/${doc}`;
    if (!mentioned.has(path)) {
      errors.push(`${file}: ${path} が対応表にない`);
      continue;
    }
    if (pending.has(path)) continue;
    const linked = [...right, ...excluded].filter((l) => l.path === path);
    if (linked.some((l) => l.anchor !== null)) {
      coverSections(path, linked, `${file} の右の列か「${EXCLUDED_SECTION}」`);
    }
  }
}

/** `path` の `##` 見出し (`Contents` を除く) が、`links` のどれかの先 (その見出しか配下の見出し、または文書全体) か。 */
function coverSections(path, links, where) {
  if (links.some((l) => l.anchor === null) && !links.some((l) => l.anchor !== null)) return;
  const hs = headings(join(root, path));
  const anchors = new Set(links.map((l) => l.anchor));
  hs.forEach((h, i) => {
    if (h.level !== 2 || h.text === "Contents") return;
    const end = hs.findIndex((x, j) => j > i && x.level <= 2);
    const inside = hs.slice(i, end < 0 ? hs.length : end);
    if (!inside.some((x) => anchors.has(x.anchor))) {
      errors.push(`${path}:${h.line}: 節「${h.text}」が ${where} にない`);
    }
  });
}

// ---- 5. spec/README.md の文書の一覧 ----

function checkReadme() {
  const listed = tableAfter("spec/README.md", "Documents")
    .flatMap((c) => [...c.matchAll(/\]\(([^)#\s]+)\)/g)].map((m) => m[1]))
    .filter((l) => l.endsWith(".md") && !l.includes("/"));
  const actual = readdirSync(join(root, "spec")).filter(
    (f) => f.endsWith(".md") && f !== "README.md",
  );
  compare("spec/README.md#documents", "文書の一覧", listed, actual);
  for (const dir of ["schema", "examples"]) {
    if (!existsSync(join(root, "spec", dir, "README.md")))
      errors.push(`spec/${dir}/README.md がない`);
  }
}

checkExamples();
checkModelTypes();
checkFeatures();
checkCorrespondence();
checkReadme();

for (const e of errors) console.error(`error: ${e}`);
if (errors.length === 0) console.log("spec ok");
process.exitCode = errors.length > 0 ? 1 : 0;
