#!/usr/bin/env node
// ugoe の GDExtension をビルドし、見本のプロジェクト (example/) に置く。
//
//   node bindings/godot/build.mjs            # デバッグビルド
//   node bindings/godot/build.mjs --release  # リリースビルド
//   node bindings/godot/build.mjs --check    # ビルドして、依存のライセンスの検査 (scripts/check-licenses.mjs --godot) と、
//                                            # ヘッドレスの Godot での example/tests/smoke.gd を実行する
//   node bindings/godot/build.mjs --conformance [--only <名前>]
//       # ビルドして、`ugoe render` の画像と Godot で描いた画像を比べる (example/tests/conformance.gd)。
//       # 描画に画面が要る。DISPLAY が無ければ xvfb-run を使う。`--only` で比べるモデルを絞る (複数可)。
//
// ライブラリを example/addons/ugoe/bin/ に、見本のモデル (assets/samples/ の .ugoe と着せ替えの .ugoemod) を example/models/ に写す。
// Godot の実行ファイルは環境変数 GODOT (既定は `godot`) で指定する。
// このクレートはルートのワークスペースの外にある (docs/adr/0026-godot-binding.md)。

import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const args = process.argv.slice(2);
const release = args.includes("--release");
const check = args.includes("--check");
const conformance = args.includes("--conformance");
const only = args.flatMap((a, i) => (args[i - 1] === "--only" ? [a] : []));

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

if (check || conformance) {
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
  const dressUp = (n) => join(samples, "dress-up", n);
  // 比べるモデルと条件。`before` があれば、その値で描いてから `params` に変えた直後のフレームを比べる。
  const cases = [
    { name: "avatar", model: join(samples, "avatar.ugoe") },
    ...["nested-masks", "mixed-masks", "intersection-masks", "blend-modes", "many-masks"].map(
      (n) => ({ name: n, model: join(work, `${n}.ugoe`) }),
    ),
    {
      name: "moving-masks",
      model: join(work, "moving-masks.ugoe"),
      before: { Move: -1 },
      params: { Move: 1 },
    },
    {
      name: "avatar-moving",
      model: join(samples, "avatar.ugoe"),
      before: { AngleX: -30, AngleY: 20 },
      params: { AngleX: 30, AngleY: -20 },
    },
    {
      name: "avatar-lit",
      model: join(samples, "avatar-lit.ugoe"),
      ambient: [0.35, 0.35, 0.4],
      lights: [
        { type: "directional", direction: [1, 1, -1], color: [0.8, 0.75, 0.7] },
        { type: "point", position: [700, 300, 250], color: [0.6, 0.6, 0.9], range: 900 },
      ],
    },
    {
      name: "dress-up",
      model: dressUp("dress-up.ugoe"),
      modules: ["long-hair.ugoemod", "sailor.ugoemod", "cat-ears.ugoemod"].map(dressUp),
    },
  ].filter((c) => only.length === 0 || only.includes(c.name));
  let failed = 0;
  for (const c of cases) {
    const reference = join(work, `${c.name}.reference.png`);
    const actual = join(work, `${c.name}.godot.png`);
    const caseFile = join(work, `${c.name}.json`);
    writeFileSync(caseFile, JSON.stringify(c));
    // Godot の側は読み込みの後に必ず `Puppet::update(0)` を通す (物理も初期化される) ので、`--time 0` で揃える。
    const renderArgs = [
      "render",
      c.model,
      "-o",
      reference,
      "--background",
      "#808080",
      "--time",
      "0",
    ];
    for (const [id, value] of Object.entries(c.params ?? {})) {
      renderArgs.push("--param", `${id}=${value}`);
    }
    for (const m of c.modules ?? []) renderArgs.push("--module", m);
    if (c.ambient) renderArgs.push("--ambient", c.ambient.join(","));
    for (const l of c.lights ?? []) {
      const color = l.color.join(",");
      renderArgs.push(
        "--light",
        l.type === "directional"
          ? `directional:${l.direction.join(",")}:${color}`
          : `point:${l.position.join(",")}:${color}:${l.range ?? "inf"}`,
      );
    }
    execFileSync(ugoe, renderArgs, { stdio: "ignore" });
    console.log(`== ${c.name}`);
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
          caseFile,
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
