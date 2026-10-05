#!/usr/bin/env node
// ugoe の GDExtension をビルドし、見本のプロジェクト (example/) に置く。
//
//   node bindings/godot/build.mjs            # デバッグビルド
//   node bindings/godot/build.mjs --release  # リリースビルド
//   node bindings/godot/build.mjs --check    # ビルドして、依存のライセンスの検査 (scripts/check-licenses.mjs --godot) と、
//                                            # wgpu・naga に依存しないことの検査 (T0166) と、
//                                            # ヘッドレスの Godot での example/tests/smoke.gd を実行する
//   node bindings/godot/build.mjs --import   # ビルドして、見本のプロジェクトを Godot で読み込む (.godot/ の作成と拡張の登録)
//   node bindings/godot/build.mjs --conformance [--only <ケース>]
//       # ビルドして、配布物の適合テスト (tests/conformance/README.md) を Godot で実行する
//       # (= node tests/conformance/run.mjs --only godot。実行器は example/tests/conformance.gd)。
//       # 描画に画面が要る。DISPLAY が無ければ xvfb-run を使う。`--only` で計画のケースを絞る (複数可)。
//
// ライブラリを example/addons/ugoe/bin/ に、見本のモデル (assets/samples/ の .ugoe と着せ替えの .ugoemod) を example/models/ に写す。
// Godot の実行ファイルは環境変数 GODOT (既定は `godot`) で指定する。
// このクレートはルートのワークスペースの外にある (docs/adr/0026-godot-binding.md)。

import { execFileSync } from "node:child_process";
import { copyFileSync, lstatSync, mkdirSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { cargoEnvWithMbx } from "../../scripts/lib/mbx.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const args = process.argv.slice(2);
const release = args.includes("--release");
const check = args.includes("--check");
const conformance = args.includes("--conformance");
const importProject = args.includes("--import");
const only = args.flatMap((a, i) => (args[i - 1] === "--only" ? [a] : []));

// mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
// (just を通らずに呼ばれても PATH に shim を足す)。
const cargo = cargoEnvWithMbx(process.env);
// target/ はリポジトリのルートと共有する (ugoe のクレートのビルド済みの成果物を使い回す)。
// ただし mbx を通さない (`MBX_DISABLE`) のに mbx が入っているか、ルートの target/ が mbx の managed target (シンボリックリンク)
// なら、共有の target/ には mbx の置き場からの読み取り専用のハードリンクがあり、素の cargo は書き直せずに失敗する
// (`output file ... is not writeable`)。そのときは bindings/godot/target/ に分けてビルドする。
const sharedTarget = join(root, "target");
let targetDir = process.env.CARGO_TARGET_DIR ?? sharedTarget;
if (
  process.env.CARGO_TARGET_DIR === undefined &&
  !cargo.viaMbx &&
  (cargo.installed || isSymlink(sharedTarget))
) {
  targetDir = join(here, "target");
  console.log(`mbx を通さないので、mbx と共有しない ${targetDir} でビルドする`);
}
const cargoArgs = ["build", "--locked", "--manifest-path", join(here, "Cargo.toml")];
if (release) cargoArgs.push("--release");
const cargoEnv = { ...cargo.env, CARGO_TARGET_DIR: targetDir };
execFileSync("cargo", cargoArgs, { stdio: "inherit", env: cargoEnv });

function isSymlink(path) {
  try {
    return lstatSync(path).isSymbolicLink();
  } catch {
    return false;
  }
}

const lib = {
  linux: "libugoe_godot.so",
  darwin: "libugoe_godot.dylib",
  win32: "ugoe_godot.dll",
}[process.platform];
if (!lib) throw new Error(`unsupported platform: ${process.platform}`);
const example = join(here, "example");
const addon = join(example, "addons", "ugoe");
mkdirSync(join(addon, "bin"), { recursive: true });
mkdirSync(join(example, "models"), { recursive: true });
copyFileSync(join(targetDir, release ? "release" : "debug", lib), join(addon, "bin", lib));
const samples = join(root, "assets", "samples");
const models = [
  join(samples, "avatar.ugoe"),
  join(samples, "avatar-lit.ugoe"),
  ...["dress-up.ugoe", "long-hair.ugoemod", "sailor.ugoemod", "cat-ears.ugoemod"].map((n) =>
    join(samples, "dress-up", n),
  ),
];
for (const model of models) copyFileSync(model, join(example, "models", basename(model)));
console.log(`copied ${lib} and ${models.length} sample models into ${example}`);

const godot = process.env.GODOT ?? "godot";

/** Godot (と、画面が要るなら xvfb-run) があるかを確かめ、無ければ入れ方を示して止める。 */
function requireGodot({ display = false } = {}) {
  try {
    const version = execFileSync(godot, ["--version"], { encoding: "utf8" }).trim();
    console.log(`godot: ${version}`);
  } catch {
    console.error(
      [
        `error: Godot が見つからない (GODOT=${godot})。`,
        "  Godot 4.4 以降を https://godotengine.org/download から入れて PATH に置くか、",
        "  環境変数 GODOT に実行ファイルのパスを指定する (CI の版は .github/workflows/ci.yml の godot のジョブ)。",
      ].join("\n"),
    );
    process.exit(1);
  }
  if (display && !process.env.DISPLAY && !process.env.WAYLAND_DISPLAY) {
    try {
      execFileSync("sh", ["-c", "command -v xvfb-run"], { stdio: "ignore" });
    } catch {
      console.error(
        "error: 画面が無く、xvfb-run も無い。`sudo apt-get install xvfb` などで入れる (--conformance は描画に画面が要る)。",
      );
      process.exit(1);
    }
  }
}

/** Godot を実行する。失敗したら例外を投げる。スクリプトのエラーで終わらないときのために、10 分で打ち切る。 */
function runGodot(godotArgs, { display = false } = {}) {
  const [cmd, cmdArgs] =
    display && !process.env.DISPLAY && !process.env.WAYLAND_DISPLAY
      ? ["xvfb-run", ["-a", "-s", "-screen 0 1280x1024x24", godot, ...godotArgs]]
      : [godot, godotArgs];
  execFileSync(cmd, cmdArgs, { stdio: "inherit", timeout: 10 * 60 * 1000 });
}

if (check || conformance || importProject) {
  requireGodot({ display: conformance });
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
  execFileSync("node", [join(root, "scripts", "check-licenses.mjs"), "--godot"], {
    stdio: "inherit",
  });
  // GDExtension は参照レンダラを使わない (ugoe を default-features = false で使う) ので、wgpu・naga を含めない (T0166)。
  const tree = execFileSync(
    "cargo",
    [
      "tree",
      "--locked",
      "--manifest-path",
      join(here, "Cargo.toml"),
      "-e",
      "normal,build",
      "--prefix",
      "none",
    ],
    { encoding: "utf8", env: cargoEnv },
  );
  const gpu = tree.split("\n").filter((line) => /^(wgpu|naga) /.test(line));
  if (gpu.length > 0) {
    throw new Error(`the GDExtension depends on wgpu or naga:\n${[...new Set(gpu)].join("\n")}`);
  }
  runGodot(["--headless", "--path", example, "--script", "res://tests/smoke.gd"]);
}

if (conformance) {
  // 比べ合わせは適合テストの計画と `ugoe-conformance compare` で行う (Godot と xvfb-run は run.mjs も調べる)。
  const cases = only.length > 0 ? ["--cases", only.join(",")] : [];
  try {
    execFileSync(
      "node",
      [
        join(root, "tests", "conformance", "run.mjs"),
        "--only",
        "godot",
        "--no-godot-build",
        ...cases,
      ],
      // 参照を描く ugoe のクレートも、上の cargo と同じ環境と target でビルドする。
      { stdio: "inherit", env: cargoEnv },
    );
  } catch {
    process.exitCode = 1;
  }
}
