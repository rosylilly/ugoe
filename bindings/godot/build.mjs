#!/usr/bin/env node
// ugoe の GDExtension をビルドし、見本のプロジェクト (example/) に置く。
//
//   node bindings/godot/build.mjs            # デバッグビルド
//   node bindings/godot/build.mjs --release  # リリースビルド
//   node bindings/godot/build.mjs --check    # ビルドして、依存のライセンスの検査 (check-licenses.mjs) と、
//                                            # ヘッドレスの Godot での example/tests/smoke.gd を実行する
//   node bindings/godot/build.mjs --conformance
//       # ビルドして、`ugoe render` の画像と Godot で描いた画像を比べる (example/tests/conformance.gd)。
//       # 描画に画面が要る。DISPLAY が無ければ xvfb-run を使う。
//
// ライブラリを example/bin/ に、見本のモデル (assets/samples/avatar.ugoe) を example/models/ に写す。
// Godot の実行ファイルは環境変数 GODOT (既定は `godot`) で指定する。
// このクレートはルートのワークスペースの外にある (docs/adr/0026-godot-binding.md)。

import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const args = process.argv.slice(2);
const release = args.includes("--release");
const check = args.includes("--check");
const conformance = args.includes("--conformance");

// target/ はリポジトリのルートと共有する (ugoe のクレートのビルド済みの成果物を使い回す)。
const targetDir = process.env.CARGO_TARGET_DIR ?? join(root, "target");
const cargoArgs = ["build", "--locked", "--manifest-path", join(here, "Cargo.toml")];
if (release) cargoArgs.push("--release");
execFileSync("cargo", cargoArgs, {
  stdio: "inherit",
  env: { ...process.env, CARGO_TARGET_DIR: targetDir },
});

const lib = {
  linux: "libugoe_godot.so",
  darwin: "libugoe_godot.dylib",
  win32: "ugoe_godot.dll",
}[process.platform];
if (!lib) throw new Error(`unsupported platform: ${process.platform}`);
const example = join(here, "example");
mkdirSync(join(example, "bin"), { recursive: true });
mkdirSync(join(example, "models"), { recursive: true });
copyFileSync(join(targetDir, release ? "release" : "debug", lib), join(example, "bin", lib));
copyFileSync(
  join(root, "assets", "samples", "avatar.ugoe"),
  join(example, "models", "avatar.ugoe"),
);
console.log(`copied ${lib} and avatar.ugoe into ${example}`);

const godot = process.env.GODOT ?? "godot";

/** Godot を実行する。失敗したら例外を投げる。 */
function runGodot(godotArgs, { display = false } = {}) {
  const [cmd, cmdArgs] =
    display && !process.env.DISPLAY && !process.env.WAYLAND_DISPLAY
      ? ["xvfb-run", ["-a", "-s", "-screen 0 1280x1024x24", godot, ...godotArgs]]
      : [godot, godotArgs];
  execFileSync(cmd, cmdArgs, { stdio: "inherit" });
}

if (check || conformance) {
  // 初回はプロジェクトの読み込み (.godot/ の作成と拡張の登録) が要る。
  // Godot 4.7.1 は、GDExtension のクラスを持つプロジェクトの初回の --import の終わりに異常終了することがある
  // (godot-rust の最小のクラスでも起きる。2 回目からは起きない) ので、1 回だけやり直す。
  try {
    runGodot(["--headless", "--path", example, "--import"]);
  } catch {
    console.warn("godot --import failed; retrying once");
    runGodot(["--headless", "--path", example, "--import"]);
  }
}

if (check) {
  execFileSync("node", [join(here, "check-licenses.mjs")], { stdio: "inherit" });
  runGodot(["--headless", "--path", example, "--script", "res://tests/smoke.gd"]);
}

if (conformance) {
  const work = mkdtempSync(join(tmpdir(), "ugoe-godot-"));
  execFileSync(
    "cargo",
    [
      "run",
      "--locked",
      "--quiet",
      "--manifest-path",
      join(here, "Cargo.toml"),
      "--example",
      "conformance_bundles",
      "--",
      work,
    ],
    { stdio: "inherit", env: { ...process.env, CARGO_TARGET_DIR: targetDir } },
  );
  execFileSync("cargo", ["build", "--locked", "--quiet", "-p", "ugoe-cli"], {
    cwd: root,
    stdio: "inherit",
    env: { ...process.env, CARGO_TARGET_DIR: targetDir },
  });
  const ugoe = join(targetDir, "debug", process.platform === "win32" ? "ugoe.exe" : "ugoe");
  const models = [
    join(root, "assets", "samples", "avatar.ugoe"),
    ...["nested-masks", "mixed-masks", "intersection-masks", "blend-modes"].map((n) =>
      join(work, `${n}.ugoe`),
    ),
  ];
  let failed = 0;
  for (const model of models) {
    const name = basename(model, ".ugoe");
    const reference = join(work, `${name}.reference.png`);
    const actual = join(work, `${name}.godot.png`);
    execFileSync(ugoe, ["render", model, "-o", reference, "--background", "#808080"], {
      stdio: "ignore",
    });
    console.log(`== ${name}`);
    try {
      runGodot(
        [
          "--path",
          example,
          "--rendering-driver",
          "opengl3",
          "--script",
          "res://tests/conformance.gd",
          "--",
          model,
          reference,
          actual,
        ],
        { display: true },
      );
    } catch {
      failed += 1;
    }
  }
  console.log(`images: ${work}`);
  if (failed > 0) {
    console.error(`${failed} model(s) did not match`);
    process.exitCode = 1;
  }
}
