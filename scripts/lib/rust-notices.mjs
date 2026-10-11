// 配布物に入る Rust のクレートのライセンスの表記 (クレートの一覧とライセンスの文) を、cargo-about で作る。
// アプリの表記 (scripts/third-party-notices.mjs) が使う。設定は scripts/about.toml、手順は docs/release.md#同梱物のライセンスの表記。
//
// cargo-about の JSON の出力 (`cargo about generate --format json`) を読み、文はここで組み立てる (handlebars の
// テンプレートは使わない)。ワークスペースのクレート (ugoe 自身) と、バイナリに入らない手続きマクロのクレート
// (`cargo metadata` で求める。compileOnlyCrates) を除き、ライセンスのファイルではないソースの見出しから
// 拾った文を除き、ライセンスのファイルを持たないクレートには Cargo.toml の authors を添えるため。

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { exportMbxPath } from "./mbx.mjs";

/** 使う cargo-about の版 (CI も手元もこの版。`just cargo-about` が入れる)。 */
export const CARGO_ABOUT_VERSION = "0.9.2";

/** cargo-about を入れるコマンド (0.9 から CLI は `cli` の機能の中にある)。 */
export const CARGO_ABOUT_INSTALL = `cargo install cargo-about --version ${CARGO_ABOUT_VERSION} --locked --features cli`;

/** cargo-about の設定 (リポジトリのルートから)。 */
export const ABOUT_CONFIG = "scripts/about.toml";

/** cargo-about の JSON を覚えておく場所 (リポジトリのルートから)。 */
const CACHE_DIR = "target/cargo-about";

/** 区切りの線 (アプリの表記と同じ幅)。 */
const ENTRY_RULE = "-".repeat(78);

/** `cargo about --version` の出力から版を読む。読めなければ undefined。 */
export function parseCargoAboutVersion(output) {
  return /^cargo-about (\S+)/m.exec(output ?? "")?.[1];
}

/** 入っている cargo-about の版。入っていなければ undefined。 */
function installedCargoAbout() {
  exportMbxPath();
  const r = spawnSync("cargo", ["about", "--version"], { encoding: "utf8" });
  return r.status === 0 ? parseCargoAboutVersion(r.stdout) : undefined;
}

/** 決めた版の cargo-about が入っていなければ入れる (ソースからビルドする)。 */
export function installCargoAbout() {
  const version = installedCargoAbout();
  if (version === CARGO_ABOUT_VERSION) {
    console.log(`cargo-about ${version} は入っている`);
    return;
  }
  const [cmd, ...args] = CARGO_ABOUT_INSTALL.split(" ");
  const r = spawnSync(cmd, args, { stdio: "inherit" });
  if (r.error) throw r.error;
  if (r.status !== 0) throw new Error(`${CARGO_ABOUT_INSTALL} が失敗した (終了コード ${r.status})`);
}

/** 入っている cargo-about が決めた版でなければ、入れ方を書いた例外を投げる。 */
export function assertCargoAbout() {
  const version = installedCargoAbout();
  if (version !== CARGO_ABOUT_VERSION) {
    const found = version ? `${version} が入っている` : "入っていない";
    throw new Error(
      `cargo-about ${CARGO_ABOUT_VERSION} が要る (${found})。\`just cargo-about\` (${CARGO_ABOUT_INSTALL}) で入れる`,
    );
  }
}

/**
 * `cargo about generate` の引数。`targets` は 1 つずつ `--target` に渡す (cargo-about の `--target` は値を 1 つしか取らない)。
 */
export function aboutArgs(manifest, targets) {
  const args = ["about", "generate", "--format", "json", "--frozen", "--fail"];
  args.push("--config", ABOUT_CONFIG, "--manifest-path", manifest);
  for (const target of targets ?? []) args.push("--target", target);
  return args;
}

/** cargo-about の設定 (about.toml) の文字列の配列 (`key = [ "a", "b" ]`) を読む。無ければ undefined。 */
export function aboutList(toml, key) {
  const body = new RegExp(`^${key}\\s*=\\s*\\[([^\\]]*)\\]`, "m").exec(toml)?.[1];
  if (body === undefined) return undefined;
  return [...body.matchAll(/"([^"]*)"/g)].map((m) => m[1]);
}

/** `cargo metadata` の引数。`targets` は 1 つずつ `--filter-platform` に渡す (cargo-about の `--target` と同じ対象にする)。 */
export function metadataArgs(manifest, targets) {
  const args = ["metadata", "--format-version", "1", "--frozen", "--manifest-path", manifest];
  for (const target of targets) args.push("--filter-platform", target);
  return args;
}

/**
 * コンパイルのときだけ動き、配布物のバイナリに入らないクレート (鍵は名前と版、値は理由) を `cargo metadata` の結果から求める。
 * - `"proc-macro"`: 手続きマクロのクレート (target の kind が `proc-macro`)。コンパイラがホスト向けにビルドして読み込み、
 *   生成したコードだけが残る。
 * - `"compile-time"`: 手続きマクロのクレートとビルドスクリプトからしか使われないクレート (`syn`・`quote`・`proc-macro2` など)。
 *   ホスト向けにビルドされて手続きマクロかビルドスクリプトにリンクされるだけで、配布物のバイナリにはリンクされない。
 *   テストだけが使うクレート (cargo-about も載せない) もこちらになる。
 *
 * 根のクレート (`resolve.root`。無ければワークスペースのクレート) から通常の依存 (kind が null) だけをたどり、手続きマクロの
 * クレートの先はたどらない。たどれないものが上の 2 つ。通常の依存からも使われるクレートは残る。
 * cargo metadata の依存はワークスペースのすべてのクレートの機能を合わせた上での集合なので、根のビルドに入らないクレート
 * (ほかのクレートが有効にした機能の先) も通ってしまう。`within` (cargo-about が見つけたクレートの鍵の集合) を渡すと、その中だけを
 * たどる (バイナリに入るクレートへの道は、すべてバイナリに入るクレートを通るので、除きすぎることはない)。
 */
export function compileOnlyCrates(metadata, within) {
  const packages = new Map(metadata.packages.map((p) => [p.id, p]));
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const isProcMacro = (id) =>
    packages.get(id)?.targets.some((t) => t.kind.includes("proc-macro")) ?? false;
  const roots = metadata.resolve.root ? [metadata.resolve.root] : metadata.workspace_members;
  const reached = new Set(roots);
  const queue = [...roots];
  while (queue.length > 0) {
    const node = nodes.get(queue.pop());
    for (const dep of node?.deps ?? []) {
      if (reached.has(dep.pkg) || isProcMacro(dep.pkg)) continue;
      const pkg = packages.get(dep.pkg);
      if (within && pkg && !within.has(crateKey(pkg))) continue;
      if (!dep.dep_kinds.some((k) => k.kind === null)) continue;
      reached.add(dep.pkg);
      queue.push(dep.pkg);
    }
  }
  const result = new Map();
  for (const id of nodes.keys()) {
    if (reached.has(id)) continue;
    const pkg = packages.get(id);
    if (pkg) result.set(crateKey(pkg), isProcMacro(id) ? "proc-macro" : "compile-time");
  }
  return result;
}

/**
 * `manifest` (Cargo.toml、リポジトリのルートから) のクレートについて、cargo-about の JSON (`about`) と、表記から除く
 * コンパイルのときだけ使うクレート (`compileOnly`。compileOnlyCrates) を返す。
 * 取り出しはネットワークを使わない (`--frozen`。結果が手元とネットワークの状態で変わらないように)。
 * 先に `cargo fetch --locked` でクレートのソースを揃える。`targets` を渡すと about.toml の targets を上書きする。
 */
export function rustNoticeSources(root, manifest, { targets } = {}) {
  // cargo-about は依存のすべてのファイルを読むので時間がかかる (アプリ 1 つで数十秒)。結果を左右するもの
  // (cargo-about の版・設定・Cargo.lock・すべての Cargo.toml・対象) が同じなら、前の結果を使う
  const key = cacheKey(root, [
    CARGO_ABOUT_VERSION,
    "compile-only",
    manifest,
    JSON.stringify(targets ?? null),
  ]);
  const cached = key && join(root, CACHE_DIR, `${key}.json`);
  if (cached && existsSync(cached)) {
    const { about, compileOnly } = JSON.parse(readFileSync(cached, "utf8"));
    return { about, compileOnly: new Map(compileOnly) };
  }
  assertCargoAbout();
  const run = (args) => {
    const r = spawnSync("cargo", args, {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 256 * 1024 * 1024,
    });
    if (r.error) throw r.error;
    if (r.status !== 0) {
      process.stderr.write(r.stderr);
      throw new Error(`cargo ${args.join(" ")} が失敗した (終了コード ${r.status})`);
    }
    return r.stdout;
  };
  run(["fetch", "--locked", "--quiet", "--manifest-path", manifest]);
  const about = JSON.parse(run(aboutArgs(manifest, targets)));
  const platforms =
    targets ?? aboutList(readFileSync(join(root, ABOUT_CONFIG), "utf8"), "targets") ?? [];
  const within = new Set(about.crates.map((c) => crateKey(c.package)));
  const compileOnly = compileOnlyCrates(JSON.parse(run(metadataArgs(manifest, platforms))), within);
  if (cached) {
    mkdirSync(join(root, CACHE_DIR), { recursive: true });
    writeFileSync(`${cached}.tmp`, JSON.stringify({ about, compileOnly: [...compileOnly] }));
    renameSync(`${cached}.tmp`, cached);
  }
  return { about, compileOnly };
}

/**
 * cargo-about の結果を左右するファイル (設定・git が追う Cargo.lock と Cargo.toml。bindings/godot は自分の Cargo.lock を持つ) と `extra` の SHA-256。
 * git が使えなければ undefined (覚えない)。
 */
function cacheKey(root, extra) {
  const r = spawnSync("git", ["ls-files", "-z", "--", "*Cargo.lock", "*Cargo.toml", ABOUT_CONFIG], {
    cwd: root,
    encoding: "utf8",
  });
  if (r.status !== 0) return undefined;
  const hash = createHash("sha256");
  for (const value of extra) hash.update(`${value}\0`);
  for (const file of r.stdout.split("\0").filter(Boolean).sort()) {
    const path = join(root, file);
    hash.update(`${file}\0`);
    hash.update(existsSync(path) ? readFileSync(path) : "");
    hash.update("\0");
  }
  return hash.digest("hex");
}

/**
 * cargo-about が見つけたライセンスの文の元のファイルが、ライセンスの文を置くファイルか (LICENSE・COPYING・NOTICE など。
 * LICENSES/ の下も)。cargo-about はソースのファイルの見出しからも文を拾うが、それはソースの全文なので載せない。
 */
export function isLicenseFile(path) {
  const parts = path.replaceAll("\\", "/").split("/");
  const name = parts.at(-1);
  if (/^(un)?licen[cs]e|^copying|^copyright|^notice/i.test(name)) return true;
  return parts.slice(0, -1).some((dir) => /^licen[cs]es$/i.test(dir));
}

/** クレートを指す鍵 (名前と版)。 */
function crateKey(pkg) {
  return `${pkg.name} ${pkg.version}`;
}

/** Cargo.toml の repository (無ければ homepage・crates.io のページ) を、ブラウザで開ける URL にする。 */
export function crateSource(pkg) {
  const url = pkg.repository ?? pkg.homepage;
  if (!url) return `https://crates.io/crates/${pkg.name}`;
  return url.replace(/\.git$/, "").replace(/\/$/, "");
}

/** Cargo.toml の authors から名前だけを取る (メールアドレスは載せない)。 */
export function crateAuthors(pkg) {
  return (pkg.authors ?? []).map((a) => a.replace(/\s*<[^>]*>\s*/g, " ").trim()).filter(Boolean);
}

/**
 * クレートがライセンスのファイルを持たないときに cargo-about が使う、SPDX の標準の文から、著作権者を書き込む欄
 * (`Copyright (c) <year> <copyright holders>` など) の行を除く。著作権者はクレートの authors を添えて示す。
 */
export function stripPlaceholders(text) {
  return text
    .split("\n")
    .filter((line) => !/<\s*year\s*>/i.test(line))
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

/**
 * cargo-about の JSON から、表記に載せるクレートと文を作る。
 * - ワークスペースのクレート (ソースが無い、パスで依存するもの。ugoe 自身) は載せない。
 * - `compileOnly` (compileOnlyCrates の結果) のクレートは、バイナリに入らないので載せない。
 * - 同じ文は 1 つにまとめ、使うクレートを並べる。
 * - 文が 1 つも残らないクレートがあれば例外 (ライセンスを確かめて about.toml か読み方を直す)。
 *
 * 返すもの: `crates` (名前と版で並ぶ。name・version・license・source・authors・used (使ったライセンスの ID)・
 * noFile (ライセンスのファイルを持たず、標準の文を使う)) と `texts` (ID と最初のクレートで並ぶ。id・name・text・
 * standard (標準の文か)・crates (鍵の列))、`excluded` (`compileOnly` で除いたクレートの数を理由ごとに。`proc-macro`・`compile-time`)。
 */
export function rustNotices(about, { compileOnly = new Map() } = {}) {
  const crates = new Map();
  const excluded = { "proc-macro": 0, "compile-time": 0 };
  for (const { package: pkg } of about.crates) {
    if (pkg.source === null || pkg.source === undefined) continue;
    const reason = compileOnly.get(crateKey(pkg));
    if (reason) {
      excluded[reason] += 1;
      continue;
    }
    crates.set(crateKey(pkg), {
      name: pkg.name,
      version: pkg.version,
      license: pkg.license ?? "",
      source: crateSource(pkg),
      authors: crateAuthors(pkg),
      used: new Set(),
      noFile: false,
    });
  }
  const texts = new Map();
  for (const license of about.licenses) {
    const standard = license.source_path === null || license.source_path === undefined;
    if (!standard && !isLicenseFile(license.source_path)) continue;
    const users = license.used_by.map((u) => crateKey(u.crate)).filter((k) => crates.has(k));
    if (users.length === 0) continue;
    let text = license.text.replaceAll("\r\n", "\n").trimEnd();
    if (standard) text = stripPlaceholders(text);
    // 標準の文は、ファイルの文と同じでも分けて置く (著作権者を authors で示すため)
    const id = `${standard ? "standard" : "file"}\n${license.id}\n${text}`;
    const entry = texts.get(id) ?? {
      id: license.id,
      name: license.name,
      text,
      standard,
      crates: new Set(),
    };
    for (const key of users) {
      entry.crates.add(key);
      const c = crates.get(key);
      c.used.add(license.id);
      if (standard) c.noFile = true;
    }
    texts.set(id, entry);
  }
  const missing = [...crates.keys()].filter((k) => crates.get(k).used.size === 0);
  if (missing.length > 0) {
    throw new Error(`ライセンスの文が見つからないクレート: ${missing.join(", ")}`);
  }
  const byName = (a, b) =>
    a.name.localeCompare(b.name, "en") || a.version.localeCompare(b.version, "en");
  const sortedCrates = [...crates.values()].sort(byName);
  const order = new Map(sortedCrates.map((c, i) => [crateKey(c), i]));
  const sortedTexts = [...texts.values()]
    .map((t) => ({ ...t, crates: [...t.crates].sort((a, b) => order.get(a) - order.get(b)) }))
    .sort(
      (a, b) =>
        a.id.localeCompare(b.id, "en") ||
        Number(a.standard) - Number(b.standard) ||
        order.get(a.crates[0]) - order.get(b.crates[0]),
    );
  return {
    crates: sortedCrates.map((c) => ({ ...c, used: [...c.used].sort() })),
    texts: sortedTexts,
    excluded,
  };
}

/** rustNotices で除いたクレートの数を、生成のログの 1 行にする。 */
export function excludedSummary({ excluded }) {
  return `手続きマクロのクレート ${excluded["proc-macro"]} 個と、手続きマクロ・ビルドスクリプトだけが使うクレート ${excluded["compile-time"]} 個を除いた`;
}

/** クレートの一覧と文を、表記の行の列にする (節の見出しは呼ぶ側が付ける)。 */
export function renderRustNotices({ crates, texts }) {
  const byKey = new Map(crates.map((c) => [crateKey(c), c]));
  const lines = [];
  for (const c of crates) {
    const used = c.used.join(" AND ");
    const license = used === c.license ? c.license : `${c.license} (used under ${used})`;
    lines.push("", crateKey(c), `  License: ${license}`, `  Source: ${c.source}`);
  }
  for (const t of texts) {
    lines.push("", ENTRY_RULE, "", `${t.name} (${t.id})`);
    if (t.standard) {
      lines.push(
        "Standard license text, for the following crates that do not include a license",
        "file of their own. The copyright belongs to the authors of each crate:",
      );
      for (const key of t.crates) {
        const authors = byKey.get(key).authors;
        lines.push(`  ${key}${authors.length > 0 ? ` (${authors.join(", ")})` : ""}`);
      }
    } else {
      lines.push("Used by:");
      for (const key of t.crates) lines.push(`  ${key}`);
    }
    lines.push("", t.text);
  }
  return lines;
}
