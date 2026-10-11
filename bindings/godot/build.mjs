#!/usr/bin/env node
// ugoe の GDExtension をビルドし、見本のプロジェクト (example/) に置く。
//
//   node bindings/godot/build.mjs            # デバッグビルド
//   node bindings/godot/build.mjs --release  # リリースビルド
//   node bindings/godot/build.mjs --check    # 依存のライセンスの検査 (scripts/check-licenses.mjs --godot。ビルドの前) と、ビルドして
//                                            # wgpu・naga に依存しないことの検査 (T0166) と、
//                                            # ヘッドレスの Godot での example/tests/smoke.gd を実行する
//   node bindings/godot/build.mjs --import   # ビルドして、見本のプロジェクトを Godot で読み込む (.godot/ の作成と拡張の登録)
//   node bindings/godot/build.mjs --conformance [--only <ケース>]
//       # ビルドして、配布物の適合テスト (tests/conformance/README.md) を Godot で実行する
//       # (= node tests/conformance/run.mjs --only godot。実行器は example/tests/conformance.gd)。
//       # 描画に画面が要る。DISPLAY が無ければ xvfb-run を使う。`--only` で計画のケースを絞る (複数可)。
//   node bindings/godot/build.mjs --release --bench
//       # ビルドして、1 フレームの時間を example/tests/bench.gd で測る (検査ではない。画面が要る)。
//   --renderer forward_plus
//       # --conformance と --bench を、Compatibility (opengl3。既定) の代わりに Forward+ (RenderingDevice の Vulkan) で描く。
//       # GPU が無ければ Mesa の lavapipe を使う (VK_DRIVER_FILES で lavapipe の ICD を指す。README の「検査」)。
//
// ライブラリを example/addons/ugoe/bin/ に、見本のモデル (assets/samples/ の .ugoe と着せ替えの .ugoemod) を example/models/ に写す。
// Godot の実行ファイルは環境変数 GODOT (既定は `godot`) で指定する。
// このクレートはルートのワークスペースの外にある (docs/adr/0026-godot-binding.md)。

import { execFileSync, spawnSync } from "node:child_process";
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
const bench = args.includes("--bench");
const renderer = args.flatMap((a, i) => (args[i - 1] === "--renderer" ? [a] : [])).at(-1);
if (renderer !== undefined && !["compatibility", "forward_plus"].includes(renderer)) {
  console.error(`error: unknown --renderer ${renderer} (expected compatibility or forward_plus)`);
  process.exit(2);
}
const only = args.flatMap((a, i) => (args[i - 1] === "--only" ? [a] : []));

// mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
// (just を通らずに呼ばれても PATH に shim を足す)。
const cargo = cargoEnvWithMbx(process.env);
// target/ はリポジトリのルートと共有する (ugoe のクレートのビルド済みの成果物を使い回す)。
// ただし mbx を通さない (`MBX_DISABLE`) のに mbx が入っているか、ルートの target/ が mbx の managed target (シンボリックリンク)
// なら、共有の target/ には mbx の置き場からの読み取り専用のハードリンクがあり、素の cargo は書き直せずに失敗する
// (`output file ... is not writeable`)。そのときは参照を描く ugoe のクレートも含めて bindings/godot/target/ に分けてビルドする。
// CI (`CI` がある) では GDExtension だけを bindings/godot/target/ に分ける。rust-cache は保存の前に、ワークスペースの依存でない
// ものを target から消すので、ルートの target/ に置くと godot-rust などの依存がキャッシュに残らない (release-checks.yml の godot のジョブは
// bindings/godot/target/ を別のキャッシュにする。docs/testing.md の「CI のキャッシュ」)。参照を描く ugoe のクレートは、
// conformance のジョブのキャッシュを使えるようにルートの target/ でビルドする。
const sharedTarget = join(root, "target");
const ownTarget = join(here, "target");
let targetDir = process.env.CARGO_TARGET_DIR ?? sharedTarget;
let referenceTargetDir = targetDir;
if (process.env.CARGO_TARGET_DIR === undefined) {
  if (!cargo.viaMbx && (cargo.installed || isSymlink(sharedTarget))) {
    targetDir = referenceTargetDir = ownTarget;
    console.log(`mbx を通さないので、mbx と共有しない ${targetDir} でビルドする`);
  } else if (process.env.CI) {
    targetDir = ownTarget;
    console.log(
      `CI なので、GDExtension を ${targetDir} でビルドする (参照の画像は ${referenceTargetDir})`,
    );
  }
}
const cargoArgs = ["build", "--locked", "--manifest-path", join(here, "Cargo.toml")];
if (release) cargoArgs.push("--release");
const cargoEnv = { ...cargo.env, CARGO_TARGET_DIR: targetDir };
const referenceEnv = { ...cargo.env, CARGO_TARGET_DIR: referenceTargetDir };
if (check) {
  // 依存のライセンスの検査はビルドの前に流す。Cargo.lock が古ければ (`--locked`)、cargo のビルドの失敗とスタックではなく、
  // 古い lock のファイルと直し方を短く出して止まる (scripts/lib/cargo-locked.mjs)
  const licenses = spawnSync("node", [join(root, "scripts", "check-licenses.mjs"), "--godot"], {
    stdio: "inherit",
  });
  if (licenses.status !== 0) process.exit(licenses.status ?? 1);
}
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
        "  環境変数 GODOT に実行ファイルのパスを指定する (CI の版は .github/workflows/release-checks.yml の godot のジョブ)。",
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

if (check || conformance || importProject || bench) {
  requireGodot({ display: conformance || bench });
  // 初回はプロジェクトの読み込み (.godot/ の作成と拡張の登録) が要る。
  // Godot 4.7.1・4.7.2 は、GDExtension のクラスを持つプロジェクトの初回の --import の終わりに異常終了することがある
  // (godot-rust の最小のクラスでも起きる。2 回目からは起きない) ので、1 回だけやり直す。
  try {
    runGodot(["--headless", "--path", example, "--import"]);
  } catch {
    console.warn("godot --import failed; retrying once");
    runGodot(["--headless", "--path", example, "--import"]);
  }
}

if (check) {
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
        ...(renderer ? ["--godot-renderer", renderer] : []),
        ...cases,
      ],
      // 参照を描く ugoe のクレートは、上の cargo と同じ環境でビルドする (target は上の referenceTargetDir)。
      { stdio: "inherit", env: referenceEnv },
    );
  } catch {
    process.exitCode = 1;
  }
}

if (bench) {
  // 描画の資源に渡すところも測るので、ヘッドレスではなく画面のある環境で描く (bench.gd の冒頭)。
  const [rendering, method] =
    renderer === "forward_plus"
      ? [["--rendering-method", "forward_plus", "--rendering-driver", "vulkan"], "forward_plus"]
      : [["--rendering-driver", "opengl3"], "gl_compatibility"];
  if (!release) console.warn("warning: --bench without --release measures a debug build");
  runGodot(["--path", example, ...rendering, "--script", "res://tests/bench.gd", "--", method], {
    display: true,
  });
}
