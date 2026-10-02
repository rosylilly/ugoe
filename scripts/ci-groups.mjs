#!/usr/bin/env node
// ワークスペースのクレートを、CI でジョブを分けるためのまとまり (グループ) に分ける。定義の正はこのファイルの GROUPS。
// 分け方と、クレートを足したときの手順は docs/testing.md#ci-のジョブ。
//
//   node scripts/ci-groups.mjs check          すべてのクレートがちょうど 1 つのグループに入っているかなどを検査する
//                                            (`just groups`。`just ci` に含まれる)
//   node scripts/ci-groups.mjs args <group>   `cargo clippy` / `cargo test` に渡す引数 (`-p <crate>` の並びと --features)。
//                                            `just lint-group <group>` / `just test-group <group>` が使う
//   node scripts/ci-groups.mjs matrix         GitHub Actions の matrix (JSON の 1 行。.github/workflows/ci.yml の rust-groups)
//   node scripts/ci-groups.mjs list           グループとクレートの一覧
//
// `cargo test --workspace` と違い、`-p` で選んだクレートの間でだけ feature が統合される。ワークスペース全体では
// 別のクレートが有効にしていた feature の裏のテストを落とさないよう、グループの features に書く
// (`required-features` を持つテスト・例は、ここに書かないと check が失敗する。`gpu-tests` は `just test-gpu` の担当なので除く)。

import { execFileSync } from "node:child_process";

/**
 * グループの定義。`tauri` が true のグループだけ、CI で Tauri のシステムの依存 (webkit など) を入れる。
 * @type {Record<string, { crates: string[], features?: string[], tauri?: boolean }>}
 */
export const GROUPS = {
  // ランタイム層 (crates.io に公開するクレート)。ワークスペースでは ugoe-editor-core が schema と ts を有効にする
  runtime: {
    crates: [
      "ugoe-core",
      "ugoe-model",
      "ugoe-runtime",
      "ugoe-format",
      "ugoe-physics",
      "ugoe-motion",
      "ugoe-render",
      "ugoe",
    ],
    features: ["ugoe-model/schema", "ugoe-model/ts"],
  },
  // バインディングと配布物の適合テスト。CI では wasm32 と C ABI の検査もこのジョブで行う
  bindings: {
    crates: ["ugoe-ffi", "ugoe-wasm", "ugoe-conformance"],
  },
  // 素材の取り込みとリギング・ヘッドレス描画
  assets: {
    crates: ["ugoe-psd", "ugoe-mesher", "ugoe-rigging", "ugoe-headless"],
  },
  // エディタの中核・AI・CLI・MCP サーバ
  tools: {
    crates: ["ugoe-editor-core", "ugoe-ai", "ugoe-cli", "ugoe-mcp"],
  },
  // Tauri のアプリ (エディタ・ビューア・スタジオ)。CI では生成物の検査 (`just gen`) もこのジョブで行う
  apps: {
    crates: ["ugoe-editor", "ugoe-viewer", "ugoe-studio"],
    tauri: true,
  },
};

/** 検査から除く `required-features` (別のレシピが担当する)。 */
const IGNORED_FEATURES = new Set(["gpu-tests"]);

/**
 * グループの定義を `cargo metadata --no-deps` の結果と比べ、問題を文字列の配列で返す (空なら問題なし)。
 * @param {Record<string, { crates: string[], features?: string[] }>} groups
 * @param {{ packages: Array<{ name: string, features: Record<string, string[]>,
 *   targets: Array<{ name: string, kind: string[], "required-features"?: string[] }> }> }} metadata
 */
export function validate(groups, metadata) {
  const errors = [];
  const packages = new Map(metadata.packages.map((p) => [p.name, p]));
  const owner = new Map();
  for (const [group, def] of Object.entries(groups)) {
    if (def.crates.length === 0) errors.push(`${group}: クレートが 1 つもない`);
    for (const name of def.crates) {
      if (!packages.has(name)) {
        errors.push(`${group}: ${name} はワークスペースのクレートではない`);
      } else if (owner.has(name)) {
        errors.push(`${name}: ${owner.get(name)} と ${group} の 2 つのグループに入っている`);
      } else {
        owner.set(name, group);
      }
    }
    const crates = new Set(def.crates);
    const features = new Set(def.features ?? []);
    for (const feature of features) {
      const [name, flag] = feature.split("/");
      if (flag === undefined || !crates.has(name)) {
        errors.push(`${group}: feature ${feature} は "<グループのクレート>/<feature>" の形で書く`);
      } else if (packages.has(name) && !(flag in packages.get(name).features)) {
        errors.push(`${group}: ${name} に feature ${flag} がない`);
      }
    }
    for (const name of def.crates) {
      for (const target of packages.get(name)?.targets ?? []) {
        for (const flag of target["required-features"] ?? []) {
          if (IGNORED_FEATURES.has(flag) || features.has(`${name}/${flag}`)) continue;
          errors.push(
            `${group}: ${name} の ${target.kind.join(",")} ${target.name} は feature ${flag} が要る。` +
              `features に ${name}/${flag} を足す`,
          );
        }
      }
    }
  }
  for (const name of packages.keys()) {
    if (!owner.has(name)) {
      errors.push(`${name}: どのグループにも入っていない (scripts/ci-groups.mjs の GROUPS に足す)`);
    }
  }
  return errors;
}

/** `cargo clippy` / `cargo test` に渡す引数。 */
export function cargoArgs(def) {
  const args = def.crates.flatMap((name) => ["-p", name]);
  if (def.features?.length) args.push("--features", def.features.join(","));
  return args;
}

function metadata() {
  return JSON.parse(
    execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
}

function main([command, group]) {
  switch (command) {
    case "check": {
      const errors = validate(GROUPS, metadata());
      for (const e of errors) console.error(`error: ${e}`);
      if (errors.length === 0) console.log(`groups ok (${Object.keys(GROUPS).length} groups)`);
      return errors.length > 0 ? 1 : 0;
    }
    case "args": {
      const def = GROUPS[group];
      if (def === undefined) {
        console.error(`unknown group: ${group} (${Object.keys(GROUPS).join(", ")})`);
        return 1;
      }
      console.log(cargoArgs(def).join(" "));
      return 0;
    }
    case "matrix": {
      const include = Object.entries(GROUPS).map(([name, def]) => ({
        group: name,
        tauri: def.tauri ?? false,
      }));
      console.log(JSON.stringify({ include }));
      return 0;
    }
    case "list": {
      for (const [name, def] of Object.entries(GROUPS))
        console.log(`${name}: ${def.crates.join(" ")}`);
      return 0;
    }
    default:
      console.error("usage: node scripts/ci-groups.mjs <check|args <group>|matrix|list>");
      return 1;
  }
}

if (import.meta.main) process.exitCode = main(process.argv.slice(2));
