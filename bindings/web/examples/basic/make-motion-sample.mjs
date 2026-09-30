#!/usr/bin/env node
// モーションと表情を含む見本のバンドル (generated/avatar-motion.ugoe) を作る。`npm run sample:motion`。
//
// リポジトリの見本 (assets/samples/avatar.ugoe) はモーションと表情を持たないので、`motion` / `expression` 属性を
// 試すために作る。ugoe-cli で見本のアバターを .ugoeproj として作り直し、このディレクトリの motions/ と expressions/
// (手で書いたもの。MIT) を足して `ugoe build` する。Rust のツールチェインが要る (リポジトリの中で実行する)。

import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";

const here = import.meta.dirname;
const root = join(here, "..", "..", "..", "..");
const out = join(here, "generated");
const work = join(out, "work");
const project = join(work, "avatar.ugoeproj");

/** ugoe-cli を実行する (失敗したらその終了コードで終える)。 */
function ugoe(...args) {
  const r = spawnSync("cargo", ["run", "-q", "-p", "ugoe-cli", "--", ...args], {
    cwd: root,
    stdio: "inherit",
  });
  if (r.status !== 0) process.exit(r.status ?? 1);
}

rmSync(work, { recursive: true, force: true });
mkdirSync(work, { recursive: true });
ugoe("sample", "avatar", "-o", join(work, "avatar.ugoe"), "--project", project);
for (const dir of ["motions", "expressions"]) {
  mkdirSync(join(project, dir), { recursive: true });
  for (const name of readdirSync(join(here, dir))) {
    cpSync(join(here, dir, name), join(project, dir, name));
  }
}
ugoe("build", project, "-o", join(out, "avatar-motion.ugoe"));
rmSync(work, { recursive: true, force: true });
