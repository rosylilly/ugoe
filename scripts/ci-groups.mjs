#!/usr/bin/env node
// ワークスペースのクレートを、CI でジョブを分けるためのまとまり (グループ) に分ける。定義の正はこのファイルの GROUPS。
// 分け方と、クレートを足したときの手順は docs/testing.md#ci-のジョブ。
//
//   node scripts/ci-groups.mjs check          すべてのクレートがちょうど 1 つのグループに入っているか、Rust のテストが読む `.md` が
//                                            文書だけの変更に入っていないかなどを検査する (`just groups`。`just ci` に含まれる)
//   node scripts/ci-groups.mjs args <group> [--all-features]
//                                            `cargo clippy` / `cargo test` に渡す引数 (`-p <crate>` の並びと --features)。
//                                            `just lint-group <group>` / `just test-group <group>` が使う。--all-features を付けると
//                                            グループのクレートのすべての feature (`gpu-tests` など) を有効にする (lint-group の 2 回目の clippy)
//   node scripts/ci-groups.mjs crate-args <crate>
//                                            1 つのクレートの `-p <crate>` と、グループの features のうちそのクレートのもの
//                                            (`just crate <name>` が使う。CI と同じく `required-features` のテストも流す。T0186)
//   node scripts/ci-groups.mjs os <clippy|test> [<cargo の引数>...]
//                                            OS_CRATES の clippy (すべての feature) / テストを流す (`just lint-os` / `just test-os`。
//                                            CI の rust (windows)・rust (macos)。Windows の PowerShell からも動くよう cargo を直接呼ぶ)
//   node scripts/ci-groups.mjs changed <base> [<head>]
//                                            base から head (既定 HEAD) で変わったファイルから、動かすグループの GitHub Actions の matrix と
//                                            Rust・フロントエンドの検査を動かすか (`matrix=…`・`rust=…`・`web=…` の行。
//                                            .github/workflows/ci.yml の rust-groups が $GITHUB_OUTPUT に書く。affected)
//   node scripts/ci-groups.mjs list           グループとクレートの一覧
//
// `cargo test --workspace` と違い、`-p` で選んだクレートの間でだけ feature が統合される。ワークスペース全体では
// 別のクレートが有効にしていた feature の裏のテストを落とさないよう、グループの features に書く
// (`required-features` を持つテスト・例は、ここに書かないと check が失敗する。`gpu-tests` は `just test-gpu` の担当なので除く)。
// clippy は既定の feature と、すべての feature (`--all-features`) の 2 回流す。feature の下のコード (`gpu-tests` のテストなど) も
// 型検査と lint をするため (T0013)。

import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { exportMbxPath } from "./lib/mbx.mjs";

/**
 * グループの定義。`tauri` が true のグループだけ、CI で Tauri のシステムの依存 (webkit など) を入れる。
 * `covers` は、グループのジョブがほかのグループのクレートも検査するときのそのクレート (変わったら、このグループも動かす)。
 * @type {Record<string, { crates: string[], features?: string[], tauri?: boolean, covers?: string[] }>}
 */
export const GROUPS = {
  // ランタイム層 (crates.io に公開するクレート)。ワークスペースでは ugoe-editor-core が schema と ts を、
  // ugoe-headless と ugoe-wasm が ugoe-format の png を有効にする。ugoe-format の png を使う結合テストは required-features
  // (std の無い形の単体テストは just nostd-check。T0186)
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
    // CI のこのジョブの `just nostd-check` (std の無い ugoe-ffi の静的ライブラリ) と `just fuzz-lint` (ugoe_ffi::fuzzing) は
    // ugoe-ffi も検査するので、ugoe-ffi が変わったときもこのグループを動かす (下の affected)
    covers: ["ugoe-ffi"],
  },
  // バインディングと配布物の適合テスト。CI では wasm32 と C ABI の検査もこのジョブで行う。
  // ugoe-ffi の fuzzing は、fuzz の入口を決まった入力で回す結合テスト (tests/fuzzing.rs) の required-features
  bindings: {
    crates: ["ugoe-ffi", "ugoe-wasm", "ugoe-conformance"],
    features: ["ugoe-ffi/fuzzing"],
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
    crates: [
      "ugoe-editor",
      "ugoe-viewer",
      "ugoe-studio",
      "ugoe-studio-vcam",
      "ugoe-studio-gpu-priority",
    ],
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
    for (const name of def.covers ?? []) {
      if (!packages.has(name))
        errors.push(`${group}: covers の ${name} はワークスペースのクレートではない`);
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

/**
 * 1 つのクレートの `cargo clippy` / `cargo test` に渡す引数。グループの features のうち、そのクレートのものだけを付ける
 * (`ugoe-format/png` など。開発時の依存に自分を置かない代わりに、CI と同じ feature でテストする。T0186)。
 * どのグループにも無いクレートは `-p <crate>` だけ (知らない名前は cargo が誤りにする)。
 */
export function crateArgs(groups, name) {
  const def = Object.values(groups).find((d) => d.crates.includes(name));
  const features = (def?.features ?? []).filter((f) => f.startsWith(`${name}/`));
  return features.length > 0 ? ["-p", name, "--features", features.join(",")] : ["-p", name];
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

/**
 * 文書 (docs/ と、クレートの外の `.md`) のうち、Rust のテストが読むものと読むクレート。変わったら文書だけの変更とはみなさず、
 * そのクレートの変更として扱う。Rust のソースが読む文書と食い違わないかを `just groups` (validateDocsReads) が確かめる。
 * @type {Array<{ path: RegExp, crates: string[] }>}
 */
export const DOCS_READ_BY_TESTS = [
  // メッセージのコードの表 (ugoe-editor-core の message_tests.rs、エディタの error.rs)
  { path: /^docs\/editor\.md$/, crates: ["ugoe-editor-core", "ugoe-editor"] },
  // 機能とフォールバックの一覧 (ugoe-format の features.rs)
  { path: /^docs\/spec\//, crates: ["ugoe-format"] },
  // スタジオの API のスキーマ (スタジオの api/tests.rs)
  { path: /^docs\/studio-api\.schema\.json$/, crates: ["ugoe-studio"] },
];

/** フロントエンドの検査 (CI の `web`) だけを動かすパス。Rust のテストはここを読まない。 */
export const WEB_ONLY_PATHS = [
  /^packages\//,
  /^bindings\/web\//,
  /^tests\/e2e\//,
  /^tests\/ui-audit\//,
  /^tools\//,
];

/**
 * `cargo metadata --no-deps` から、ワークスペースのクレートの名前・ディレクトリ (ルートからの相対で末尾に `/`)・
 * ワークスペースの中の依存 (開発時の依存も) を取り出す。
 * @returns {Array<{ name: string, dir: string, deps: string[] }>}
 */
export function workspaceCrates(metadata) {
  const names = new Set(metadata.packages.map((p) => p.name));
  return metadata.packages.map((p) => ({
    name: p.name,
    dir: `${relative(metadata.workspace_root, dirname(p.manifest_path)).split(sep).join("/")}/`,
    deps: [
      ...new Set(
        p.dependencies.filter((d) => d.path !== undefined && names.has(d.name)).map((d) => d.name),
      ),
    ],
  }));
}

/** ワークスペースのクレートのディレクトリの中のファイルなら、そのクレート (いちばん深いもの)。 */
function crateOf(file, crates) {
  let found;
  for (const c of crates) {
    if (file.startsWith(c.dir) && (found === undefined || c.dir.length > found.dir.length))
      found = c;
  }
  return found;
}

/**
 * 文書だけの変更か (CI の Rust の検査もフロントエンドの検査も動かさない)。docs/ のファイルと、クレートの外の `.md` で、
 * テストが読むもの (DOCS_READ_BY_TESTS) でないもの。クレートの中の README などはテストが読む (ugoe-mcp) ので
 * クレートの変更として扱う。
 */
export function isDocsOnly(file, crates) {
  return (
    (file.startsWith("docs/") || file.endsWith(".md")) &&
    crateOf(file, crates) === undefined &&
    !DOCS_READ_BY_TESTS.some((e) => e.path.test(file))
  );
}

/**
 * 変わったファイル (リポジトリのルートからの相対) から、CI (ci.yml) で動かす Rust のグループとフロントエンドの検査を決める
 * (docs/testing.md#変わったパスでの絞り込み)。
 * - 文書だけ (isDocsOnly) は何も動かさない (ビルドしない検査 `rust-checks` は常に動く)。テストが読む文書は、読むクレートの変更。
 * - クレートの中のファイルは、そのクレートと、それに (開発時の依存も含めて) 依存するクレートのグループを動かす。
 *   グループの `covers` のクレートが変わったら、そのグループも動かす。
 * - `apps/<アプリ>/` の src-tauri の外 (フロントエンド・翻訳・設定) は、そのアプリのクレートの変更として扱う
 *   (src-tauri のテストが翻訳や tauri.conf.json を読む)。
 * - WEB_ONLY_PATHS はフロントエンドの検査だけ。
 * - それ以外 (Cargo.lock・rust-toolchain.toml・justfile・scripts/・.github/・assets/・spec/・fuzz/ など) はすべてのグループを動かす。
 * フロントエンドの検査は、文書だけの変更でなければ動かす (wasm と生成した型を使うため)。
 * @param {string[]} files
 * @param {Record<string, { crates: string[], covers?: string[] }>} groups
 * @param {Array<{ name: string, dir: string, deps: string[] }>} crates
 * @returns {{ groups: string[], web: boolean }}
 */
export function affected(files, groups, crates) {
  const changed = new Set();
  let all = false;
  let web = false;
  for (const file of files) {
    const read = DOCS_READ_BY_TESTS.find((e) => e.path.test(file));
    if (read !== undefined) {
      for (const name of read.crates) changed.add(name);
      continue;
    }
    if (isDocsOnly(file, crates)) continue;
    web = true;
    const crate = crateOf(file, crates);
    const app = /^apps\/[^/]+\//.exec(file)?.[0];
    if (crate !== undefined) {
      changed.add(crate.name);
    } else if (app !== undefined) {
      for (const c of crates) if (c.dir.startsWith(app)) changed.add(c.name);
    } else if (!WEB_ONLY_PATHS.some((re) => re.test(file))) {
      all = true;
    }
  }
  if (all) return { groups: Object.keys(groups), web };
  const dependents = new Map(crates.map((c) => [c.name, []]));
  for (const c of crates) for (const d of c.deps) dependents.get(d)?.push(c.name);
  const queue = [...changed];
  while (queue.length > 0) {
    for (const next of dependents.get(queue.pop()) ?? []) {
      if (!changed.has(next)) {
        changed.add(next);
        queue.push(next);
      }
    }
  }
  const selected = Object.entries(groups)
    .filter(([, def]) => [...def.crates, ...(def.covers ?? [])].some((name) => changed.has(name)))
    .map(([name]) => name);
  return { groups: selected, web };
}

/**
 * ワークスペースのクレートの Rust のソースが読む文書 (文字列の全体が `docs/…`・`spec/…` で終わるパス。`../` や
 * `CARGO_MANIFEST_DIR` からの相対も) が、文書だけの変更 (isDocsOnly) に入っていないか、DOCS_READ_BY_TESTS の
 * 読むクレートに入っているかを確かめ、問題を文字列の配列で返す。コメントの行と、文の中のパス (メッセージ) は見ない。
 * @param {Array<{ file: string, text: string }>} sources ルートからの相対のパスと中身
 */
export function validateDocsReads(sources, crates) {
  const errors = [];
  for (const { file, text } of sources) {
    const reader = crateOf(file, crates)?.name;
    text.split("\n").forEach((line, i) => {
      if (line.trimStart().startsWith("//")) return;
      for (const [, path] of line.matchAll(/"(?:\/?(?:\.\.\/)*)((?:docs|spec)\/[\w./-]+)"/g)) {
        const read = DOCS_READ_BY_TESTS.find((e) => e.path.test(path));
        if (isDocsOnly(path, crates)) {
          errors.push(
            `${file}:${i + 1}: ${path} を読んでいるが、CI では文書だけの変更とみなしてテストを動かさない ` +
              "(scripts/ci-groups.mjs の DOCS_READ_BY_TESTS に足す)",
          );
        } else if (read !== undefined && !read.crates.includes(reader)) {
          errors.push(
            `${file}:${i + 1}: ${path} を読んでいるが、DOCS_READ_BY_TESTS の読むクレートに ${reader} が無い`,
          );
        }
      }
    });
  }
  return errors;
}

/** クレートのディレクトリの `.rs` (target/ を除く)。 */
function rustSources(root, crates) {
  const sources = [];
  const walk = (dir) => {
    for (const entry of readdirSync(join(root, dir), { withFileTypes: true })) {
      const path = `${dir}${entry.name}`;
      if (entry.isDirectory()) {
        if (entry.name !== "target" && entry.name !== "node_modules") walk(`${path}/`);
      } else if (entry.name.endsWith(".rs")) {
        sources.push({ file: path, text: readFileSync(join(root, path), "utf8") });
      }
    }
  };
  for (const c of crates) walk(c.dir);
  return sources;
}

/** `git diff` で変わったファイル。base が無い (新しいブランチ・全 0) か取れなければ undefined (すべてを動かす)。 */
function changedFiles(base, head) {
  if (!base || /^0+$/.test(base)) return undefined;
  try {
    const out = execFileSync("git", ["diff", "--name-only", "--no-renames", base, head], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    });
    return out.split("\n").filter((line) => line !== "");
  } catch {
    return undefined;
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
      const crates = workspaceCrates(meta);
      const errors = [
        ...validate(GROUPS, meta),
        ...validateOs(OS_CRATES, meta),
        ...validateDocsReads(rustSources(meta.workspace_root, crates), crates),
      ];
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
    case "crate-args": {
      if (group === undefined) {
        console.error("usage: node scripts/ci-groups.mjs crate-args <crate>");
        return 1;
      }
      console.log(crateArgs(GROUPS, group).join(" "));
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
    case "changed": {
      const files = changedFiles(group, rest[0] ?? "HEAD");
      const result =
        files === undefined
          ? { groups: Object.keys(GROUPS), web: true }
          : affected(files, GROUPS, workspaceCrates(metadata()));
      const include = result.groups.map((name) => ({
        group: name,
        tauri: GROUPS[name].tauri ?? false,
      }));
      console.error(
        files === undefined
          ? `base ${group || "(none)"} からの差分が取れないので、すべてを動かす`
          : `${files.length} 個のファイルが変わった`,
      );
      console.error(`groups: ${result.groups.join(" ") || "(none)"} / web: ${result.web}`);
      console.log(`matrix=${JSON.stringify({ include })}`);
      console.log(`rust=${result.groups.length > 0}`);
      console.log(`web=${result.web}`);
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
        "usage: node scripts/ci-groups.mjs <check|args <group> [--all-features]|crate-args <crate>|os <clippy|test>|changed <base> [<head>]|list>",
      );
      return 1;
  }
}

if (import.meta.main) process.exitCode = main(process.argv.slice(2));
