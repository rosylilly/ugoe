#!/usr/bin/env node
// crates.io に公開するクレートの検査とリリースの補助 (docs/release.md#cratesio)。範囲の決定は
// docs/adr/0027-crates-io-publishing.md。公開は CI (.github/workflows/crates-release.yml) か、利用者が手元で行う。
//
//   node scripts/crates.mjs check [<tag>]   公開の準備が整っているか (`just publish-check`。`just ci` に含まれる)。
//                                          タグ `v<版>` を渡すと、タグと [workspace.package] の version も比べる
//   node scripts/crates.mjs sync            [workspace.dependencies] の公開するクレートの version を
//                                          [workspace.package] の version に揃える (版を上げたときに実行する)
//   node scripts/crates.mjs order           公開する順 (依存の順) を 1 行に 1 つ表示する
//
// 検査すること:
// - `publish = true` のクレートが PUBLISHED と一致する (公開の範囲を広げるときは ADR とこの一覧を直す)。
// - 公開するクレートの license・description・repository・readme・keywords・categories が揃い、
//   クレートのディレクトリの LICENSE がルートの LICENSE と同じ内容である。
// - 公開するクレートが (dev 以外で) 依存する内部のクレートも公開され、version が指定されている。
// - [workspace.dependencies] の公開するクレートの version が [workspace.package] の version と同じ。

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const root = join(import.meta.dirname, "..");

/** crates.io に公開するクレート (公開する順ではない。順は `publishOrder` が依存から決める)。 */
export const PUBLISHED = [
  "ugoe-core",
  "ugoe-model",
  "ugoe-runtime",
  "ugoe-format",
  "ugoe-physics",
  "ugoe-motion",
  "ugoe-render",
  "ugoe",
];

// crates.io の制限: keywords は 5 個まで、1 つ 20 文字まで (https://doc.rust-lang.org/cargo/reference/manifest.html#the-keywords-field)
const MAX_KEYWORDS = 5;
const MAX_KEYWORD_LEN = 20;

/** Cargo.toml の `[section]` の本文 (次の見出しまで)。 */
function section(toml, name) {
  const escaped = name.replace(/\./g, "\\.");
  return toml.match(
    new RegExp(`^\\[${escaped}\\]\\r?\\n([\\s\\S]*?)(?=^\\[|(?![\\s\\S]))`, "m"),
  )?.[1];
}

/** ルートの Cargo.toml の `[workspace.package] version`。 */
export function workspaceVersion(toml) {
  const version = section(toml, "workspace.package")?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) throw new Error("Cargo.toml has no [workspace.package] version");
  return version;
}

/** `[workspace.dependencies]` の内部のクレートの version (名前 -> 版。version が無ければ null)。 */
export function internalDependencyVersions(toml) {
  const body = section(toml, "workspace.dependencies") ?? "";
  const versions = new Map();
  for (const m of body.matchAll(/^([\w-]+)\s*=\s*\{([^}\n]*\bpath\s*=[^}\n]*)\}/gm)) {
    versions.set(m[1], m[2].match(/\bversion\s*=\s*"([^"]+)"/)?.[1] ?? null);
  }
  return versions;
}

/** `[workspace.dependencies]` の公開するクレートの version を `version` に書き換えた Cargo.toml。 */
export function syncVersions(toml, version, published = PUBLISHED) {
  const body = section(toml, "workspace.dependencies");
  if (body === undefined) throw new Error("Cargo.toml has no [workspace.dependencies]");
  const next = body.replace(
    /^([\w-]+)(\s*=\s*\{[^}\n]*\bpath\s*=[^}\n]*)\bversion\s*=\s*"[^"]*"/gm,
    (line, name, rest) =>
      published.includes(name) ? `${name}${rest}version = "${version}"` : line,
  );
  return toml.replace(body, next);
}

/**
 * 公開する順。内部のクレートへの依存 (dev を含む。crates.io は dev の依存も公開済みであることを求める) が
 * 先に来るように並べる。同じ段では名前の順。循環があれば例外。
 */
export function publishOrder(packages) {
  const names = new Set(packages.map((p) => p.name));
  const deps = new Map(
    packages.map((p) => [
      p.name,
      new Set(p.dependencies.map((d) => d.name).filter((n) => names.has(n) && n !== p.name)),
    ]),
  );
  const order = [];
  const done = new Set();
  while (order.length < packages.length) {
    const ready = [...deps.keys()]
      .filter((n) => !done.has(n) && [...deps.get(n)].every((d) => done.has(d)))
      .sort();
    if (ready.length === 0) {
      const rest = [...deps.keys()].filter((n) => !done.has(n));
      throw new Error(`dependency cycle among published crates: ${rest.join(", ")}`);
    }
    for (const n of ready) {
      order.push(n);
      done.add(n);
    }
  }
  return order;
}

/** `cargo metadata` の package が crates.io に公開できる設定か (`publish` が null = 制限なし、または crates-io を含む)。 */
function isPublished(pkg) {
  return pkg.publish === null || pkg.publish.includes("crates-io");
}

/**
 * 公開の準備の問題を文の配列で返す (無ければ空)。
 * `metadata` は `cargo metadata --no-deps` の結果、`readFile` は絶対パスを受け取って内容を返す関数。
 */
export function problems({ metadata, toml, license, readFile, tag, published = PUBLISHED }) {
  const out = [];
  const version = workspaceVersion(toml);
  const members = new Set(metadata.workspace_members);
  const packages = metadata.packages.filter((p) => members.has(p.id));
  const byName = new Map(packages.map((p) => [p.name, p]));
  const internal = new Set(packages.map((p) => p.name));

  const actual = packages.filter(isPublished).map((p) => p.name);
  for (const n of actual.filter((n) => !published.includes(n))) {
    out.push(
      `${n} is publishable but not in PUBLISHED (scripts/crates.mjs); set publish = false or update the ADR`,
    );
  }
  for (const n of published.filter((n) => !actual.includes(n))) {
    out.push(
      `${n} is in PUBLISHED but ${byName.has(n) ? "not publishable (set publish = true)" : "not in the workspace"}`,
    );
  }

  for (const name of published) {
    const p = byName.get(name);
    if (!p) continue;
    if (p.version !== version)
      out.push(`${name}: version ${p.version} differs from the workspace version ${version}`);
    if (p.license !== "MIT")
      out.push(`${name}: license must be MIT (docs/adr/0018-licensing-and-1.0-scope.md)`);
    for (const field of ["description", "repository", "readme"]) {
      if (!p[field]) out.push(`${name}: missing ${field}`);
    }
    if (!p.keywords?.length) out.push(`${name}: missing keywords`);
    if (p.keywords?.length > MAX_KEYWORDS) out.push(`${name}: at most ${MAX_KEYWORDS} keywords`);
    for (const k of p.keywords ?? []) {
      if (k.length > MAX_KEYWORD_LEN || !/^[a-z0-9][a-z0-9_+-]*$/.test(k)) {
        out.push(`${name}: invalid keyword "${k}"`);
      }
    }
    if (!p.categories?.length) out.push(`${name}: missing categories`);
    const dir = dirname(p.manifest_path);
    let copy = null;
    try {
      copy = readFile(join(dir, "LICENSE"));
    } catch {}
    if (copy !== license) {
      out.push(`${name}: ${join(dir, "LICENSE")} must be a copy of the root LICENSE`);
    }
    for (const d of p.dependencies) {
      if (!internal.has(d.name) || d.kind === "dev") continue;
      if (!published.includes(d.name)) {
        out.push(`${name} depends on ${d.name}, which is not published`);
      } else if (d.req === "*") {
        out.push(
          `${name}: dependency on ${d.name} has no version (add it in [workspace.dependencies])`,
        );
      }
    }
  }

  const versions = internalDependencyVersions(toml);
  for (const name of published) {
    const v = versions.get(name);
    if (v !== version) {
      out.push(
        `[workspace.dependencies] ${name} version ${v ?? "(none)"} differs from ${version} (run: node scripts/crates.mjs sync)`,
      );
    }
  }

  if (tag && tag !== `v${version}`) {
    out.push(`tag ${tag} does not match the workspace version ${version} (expected v${version})`);
  }
  return out;
}

function metadata() {
  return JSON.parse(
    execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
}

function publishedPackages(meta) {
  const members = new Set(meta.workspace_members);
  return meta.packages.filter((p) => members.has(p.id) && PUBLISHED.includes(p.name));
}

function check(tag) {
  const meta = metadata();
  const found = problems({
    metadata: meta,
    toml: readFileSync(join(root, "Cargo.toml"), "utf8"),
    license: readFileSync(join(root, "LICENSE"), "utf8"),
    readFile: (path) => readFileSync(path, "utf8"),
    tag,
  });
  for (const p of found) console.error(`error: ${p}`);
  if (found.length > 0) process.exit(1);
  const version = workspaceVersion(readFileSync(join(root, "Cargo.toml"), "utf8"));
  console.log(`crates.io: ${publishOrder(publishedPackages(meta)).join(" -> ")} (${version})`);
}

function sync() {
  const path = join(root, "Cargo.toml");
  const toml = readFileSync(path, "utf8");
  const version = workspaceVersion(toml);
  writeFileSync(path, syncVersions(toml, version));
  console.log(`[workspace.dependencies] published crates: ${version}`);
}

if (import.meta.main) {
  const [command, arg] = process.argv.slice(2);
  if (command === "check") check(arg);
  else if (command === "sync") sync();
  else if (command === "order") console.log(publishOrder(publishedPackages(metadata())).join("\n"));
  else {
    console.error("usage: node scripts/crates.mjs check [<tag>] | sync | order");
    process.exit(2);
  }
}
