#!/usr/bin/env node
// ワークスペースのクレートを、CI でジョブを分けるためのまとまり (グループ) に分ける。定義の正はこのファイルの GROUPS。
// 分け方と、クレートを足したときの手順は docs/testing.md#ci-のジョブ。
//
//   node scripts/ci-groups.mjs check          すべてのクレートがちょうど 1 つのグループに入っているかなどを検査する
//                                            (`just groups`。`just ci` に含まれる)
//   node scripts/ci-groups.mjs args <group> [--all-features]
//                                            `cargo clippy` / `cargo test` に渡す引数 (`-p <crate>` の並びと --features)。
//                                            `just lint-group <group>` / `just test-group <group>` が使う。--all-features を付けると
//                                            グループのクレートのすべての feature (`gpu-tests` など) を有効にする (lint-group の 2 回目の clippy)
//   node scripts/ci-groups.mjs os <clippy|test> [<cargo の引数>...]
//                                            OS_CRATES の clippy (すべての feature) / テストを流す (`just lint-os` / `just test-os`。
//                                            CI の rust (windows)・rust (macos)。Windows の PowerShell からも動くよう cargo を直接呼ぶ)
//   node scripts/ci-groups.mjs matrix         GitHub Actions の matrix (JSON の 1 行。.github/workflows/ci.yml の rust-groups)
//   node scripts/ci-groups.mjs list           グループとクレートの一覧
//
// `cargo test --workspace` と違い、`-p` で選んだクレートの間でだけ feature が統合される。ワークスペース全体では
// 別のクレートが有効にしていた feature の裏のテストを落とさないよう、グループの features に書く
// (`required-features` を持つテスト・例は、ここに書かないと check が失敗する。`gpu-tests` は `just test-gpu` の担当なので除く)。
// clippy は既定の feature と、すべての feature (`--all-features`) の 2 回流す。feature の下のコード (`gpu-tests` のテストなど) も
// 型検査と lint をするため (T0013)。

import { execFileSync } from "node:child_process";
import { exportMbxPath } from "./lib/mbx.mjs";

/**
 * グループの定義。`tauri` が true のグループだけ、CI で Tauri のシステムの依存 (webkit など) を入れる。
 * @type {Record<string, { crates: string[], features?: string[], tauri?: boolean }>}
 */
export const GROUPS = {
  // ランタイム層 (crates.io に公開するクレート)。ワークスペースでは ugoe-editor-core が schema と ts を、
  // ugoe-headless と ugoe-wasm が ugoe-format の png を有効にする
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
    features: ["ugoe-format/png", "ugoe-model/schema", "ugoe-model/ts"],
  },
  // バインディングと配布物の適合テスト。CI では wasm32 と C ABI の検査もこのジョブで行う
  bindings: {
    crates: ["ugoe-ffi", "ugoe-wasm", "ugoe-conformance"],
  },
  // 素材の取り込みとリギング・ヘッドレス描画
  assets: {
    crates: ["ugoe-psd", "ugoe-mesher", "ugoe-rigging", "ugoe-headless"],
  },
  // エディタの中核・AI・CLI・MCP サーバ・ファイルの読み書きの補助
  tools: {
    crates: ["ugoe-fs", "ugoe-editor-core", "ugoe-ai", "ugoe-cli", "ugoe-mcp"],
  },
  // Tauri のアプリ (エディタ・ビューア・スタジオ)。CI では生成物の検査 (`just gen`) もこのジョブで行う
  apps: {
    crates: ["ugoe-editor", "ugoe-viewer", "ugoe-studio", "ugoe-studio-vcam"],
    tauri: true,
  },
};

/**
 * Windows と macOS のランナーでも clippy とテストを流すクレート (CI の rust (windows)・rust (macos)。docs/testing.md#ci-のジョブ)。
 * ファイルを書き出す・パスを扱うクレート。大文字小文字だけが違う名前やパスの区切りの違いの誤りは、Linux では見つからない (T0667)。
 * scripts/cross-check.mjs の `tools` も、この一覧を Windows のターゲットで clippy にかける (T0962)。
 */
export const OS_CRATES = ["ugoe-format", "ugoe-fs", "ugoe-editor-core", "ugoe-cli", "ugoe-mcp"];

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

/**
 * OS_CRATES がワークスペースのクレートかを検査し、問題を文字列の配列で返す (空なら問題なし)。
 * @param {string[]} crates
 * @param {{ packages: Array<{ name: string }> }} metadata
 */
export function validateOs(crates, metadata) {
  const packages = new Set(metadata.packages.map((p) => p.name));
  if (crates.length === 0) return ["OS_CRATES: クレートが 1 つもない"];
  return crates
    .filter((name) => !packages.has(name))
    .map((name) => `OS_CRATES: ${name} はワークスペースのクレートではない`);
}

/**
 * `cargo clippy` / `cargo test` に渡す引数。`allFeatures` ならグループの features の代わりに --all-features を付ける
 * (選んだクレートのすべての feature。依存先のクレートの feature は有効にしない)。
 */
export function cargoArgs(def, { allFeatures = false } = {}) {
  const args = def.crates.flatMap((name) => ["-p", name]);
  if (allFeatures) args.push("--all-features");
  else if (def.features?.length) args.push("--features", def.features.join(","));
  return args;
}

/** OS_CRATES の `cargo` の引数。clippy はすべての feature で、警告をエラーにする。 */
export function osCargoArgs(command, crates, extra = []) {
  const packages = crates.flatMap((name) => ["-p", name]);
  switch (command) {
    case "clippy":
      return [
        "clippy",
        ...packages,
        "--all-features",
        "--all-targets",
        ...extra,
        "--",
        "-D",
        "warnings",
      ];
    case "test":
      return ["test", ...packages, ...extra];
    default:
      throw new Error(`unknown command: ${command} (clippy, test)`);
  }
}

function metadata() {
  return JSON.parse(
    execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
}

function main([command, group, ...rest]) {
  // mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
  exportMbxPath();
  switch (command) {
    case "check": {
      const meta = metadata();
      const errors = [...validate(GROUPS, meta), ...validateOs(OS_CRATES, meta)];
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
      const allFeatures = rest.includes("--all-features");
      console.log(cargoArgs(def, { allFeatures }).join(" "));
      return 0;
    }
    case "os": {
      let args;
      try {
        args = osCargoArgs(group, OS_CRATES, rest);
      } catch (e) {
        console.error(`error: ${e.message}`);
        return 1;
      }
      console.log(`== cargo ${args.join(" ")}`);
      try {
        execFileSync("cargo", args, { stdio: "inherit" });
      } catch {
        return 1;
      }
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
      console.log(`(windows, macos): ${OS_CRATES.join(" ")}`);
      return 0;
    }
    default:
      console.error(
        "usage: node scripts/ci-groups.mjs <check|args <group> [--all-features]|os <clippy|test>|matrix|list>",
      );
      return 1;
  }
}

if (import.meta.main) process.exitCode = main(process.argv.slice(2));
