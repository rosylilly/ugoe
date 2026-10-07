#!/usr/bin/env node
// ワークスペースの外のクレート (fuzz/・bindings/godot/) の Cargo.lock の依存の版が、ルートの Cargo.lock とそろっているかを検査する
// (docs/testing.md#fuzz。判定は scripts/lib/lock-sync.mjs)。`just groups` と `node fuzz/run.mjs --check`・`--lint` が実行する。
//
//   node scripts/check-lock-sync.mjs            # すべての外の lock
//   node scripts/check-lock-sync.mjs fuzz       # 1 つだけ (fuzz・godot)

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { lockDrift, lockDriftMessage } from "./lib/lock-sync.mjs";

const REPO_ROOT = fileURLToPath(new URL("..", import.meta.url));
// 外のクレートの置き場所 (ルートからの相対)
const OUTSIDE = { fuzz: "fuzz", godot: "bindings/godot" };

const names = process.argv.slice(2);
const unknown = names.filter((n) => !(n in OUTSIDE));
if (unknown.length > 0) {
  console.error(`usage: node scripts/check-lock-sync.mjs [${Object.keys(OUTSIDE).join("|")}]...`);
  process.exit(2);
}

const rootLock = readFileSync(join(REPO_ROOT, "Cargo.lock"), "utf8");
let failed = false;
const checked = names.length > 0 ? names : Object.keys(OUTSIDE);
for (const name of checked) {
  const dir = OUTSIDE[name];
  const drift = lockDrift(rootLock, readFileSync(join(REPO_ROOT, dir, "Cargo.lock"), "utf8"));
  if (drift.length > 0) {
    console.error(lockDriftMessage(`${dir}/Cargo.lock`, `${dir}/Cargo.toml`, drift));
    failed = true;
  }
}
if (failed) process.exit(1);
console.log(`lock versions ok (${checked.join(", ")} match the root Cargo.lock)`);
