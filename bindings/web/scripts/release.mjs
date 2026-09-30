#!/usr/bin/env node
// @ugoe/runtime のリリースの補助 (bindings/web/README.md#リリース)。npm への公開は CI
// (.github/workflows/web-runtime-release.yml) か、利用者が手元で行う。
//
//   node scripts/release.mjs check [<tag>]     版がクレートの版と揃っているか (タグ `v<版>` を渡すとタグとも比べる)
//   node scripts/release.mjs sync              package.json の版をクレートの版 (Cargo.toml の workspace.package.version) にする
//   node scripts/release.mjs publish [--dry-run]
//                                              dist/ を npm に公開する。`private: true` の間は公開せず、入る内容だけを表示する
//
// 版の正はルートの Cargo.toml の `[workspace.package] version` (すべての配布物で揃える。docs/portability.md)。
// プレリリースの版 (`0.2.0-alpha.1` など) は npm の dist-tag `next`、それ以外は `latest` で公開する。

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const pkgDir = join(import.meta.dirname, "..");
const root = join(pkgDir, "..", "..");
const pkgPath = join(pkgDir, "package.json");

/** ルートの Cargo.toml の `[workspace.package] version`。 */
export function crateVersion(toml = readFileSync(join(root, "Cargo.toml"), "utf8")) {
  const section = toml.match(/^\[workspace\.package\]\r?\n([\s\S]*?)(?=^\[|(?![\s\S]))/m)?.[1];
  const version = section?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) throw new Error("Cargo.toml has no [workspace.package] version");
  return version;
}

/** npm の dist-tag。プレリリース (`-` を含む版) は `next`。 */
export function distTag(version) {
  return version.includes("-") ? "next" : "latest";
}

/**
 * 版の食い違いを文の配列で返す (揃っていれば空)。`tag` は Git のタグ (`v0.1.0`。省略可)。
 */
export function versionProblems({ packageVersion, crate, tag }) {
  const problems = [];
  if (packageVersion !== crate) {
    problems.push(
      `package.json version ${packageVersion} differs from the crates' version ${crate} (run: node scripts/release.mjs sync)`,
    );
  }
  if (tag && tag !== `v${crate}`) {
    problems.push(`tag ${tag} does not match the crates' version ${crate} (expected v${crate})`);
  }
  return problems;
}

function readPackage() {
  return JSON.parse(readFileSync(pkgPath, "utf8"));
}

function run(cmd, args) {
  const r = spawnSync(cmd, args, {
    cwd: pkgDir,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  if (r.error) throw r.error;
  if (r.status !== 0) process.exit(r.status ?? 1);
}

function check(tag) {
  const pkg = readPackage();
  const problems = versionProblems({ packageVersion: pkg.version, crate: crateVersion(), tag });
  for (const p of problems) console.error(`error: ${p}`);
  if (problems.length > 0) process.exit(1);
  console.log(
    `@ugoe/runtime ${pkg.version} (dist-tag ${distTag(pkg.version)}${pkg.private ? ", private: not publishable" : ""})`,
  );
}

function sync() {
  const text = readFileSync(pkgPath, "utf8");
  const version = crateVersion();
  // 整形 (キーの順・インデント) を保つため、version の行だけを書き換える
  const next = text.replace(/^(\s*"version":\s*)"[^"]*"/m, `$1"${version}"`);
  writeFileSync(pkgPath, next);
  console.log(`package.json version: ${version}`);
}

function publish(dryRun) {
  const pkg = readPackage();
  if (!existsSync(join(pkgDir, "dist", "wasm", "ugoe_wasm_bg.wasm"))) {
    console.error("error: dist/ is not built (run: pnpm --filter @ugoe/runtime build)");
    process.exit(1);
  }
  if (pkg.private) {
    // npm は private のパッケージを公開しない (--dry-run でも失敗する)。入る内容だけを確かめる
    const msg =
      "@ugoe/runtime is private; skipping npm publish (remove `private: true` to publish. bindings/web/README.md#リリース)";
    console.log(process.env.GITHUB_ACTIONS ? `::notice::${msg}` : `note: ${msg}`);
    run("npm", ["pack", "--dry-run"]);
    return;
  }
  const args = ["publish", "--access", "public", "--tag", distTag(pkg.version)];
  // CI (GitHub Actions の OIDC) では、どのコミットとワークフローから作ったかの来歴 (provenance) を付ける
  if (process.env.GITHUB_ACTIONS) args.push("--provenance");
  if (dryRun) args.push("--dry-run");
  run("npm", args);
}

if (import.meta.main) {
  const [command, arg] = process.argv.slice(2);
  if (command === "check") check(arg);
  else if (command === "sync") sync();
  else if (command === "publish") publish(arg === "--dry-run");
  else {
    console.error("usage: node scripts/release.mjs check [<tag>] | sync | publish [--dry-run]");
    process.exit(2);
  }
}
