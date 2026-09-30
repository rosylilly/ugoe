// 使い方: node tools/spec-check/src/roundtrip.ts <ugoe の実行ファイル> [作業ディレクトリ]
//
// 独立の書き出しと参照実装の往復の試験 (T0128)。参照実装は黒箱として、コマンドだけを使う:
//
// 1. 元のデータ (spec/examples/ の JSON の文書と .ugoe の例、assets/compat/dev/ の .ugoe) をこの実装で読む。
// 2. この実装で .ugoe に書く。書き方は 2 通り (既定値と null を省き圧縮しない / すべて書きペイロードを圧縮する)。
// 3. 参照実装の `ugoe validate` が受け付けることを確かめる。
// 4. 参照実装の `ugoe build <in> -o <out>` で読んで書き直させる。
// 5. 書き直されたファイルをこの実装で読み、元のデータと同じであることを確かめる (f32 に丸めた値で完全一致)。
//
// 食い違いを標準出力に書き、あれば終了コード 1 で終わる。モジュール (.ugoemod) は CLI が読めないので対象外。

import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { type Bundle, readBundle, readJsonDocument, readModelJson } from "./bundle.ts";
import { loadManifest } from "./bundles.ts";
import * as D from "./decode.ts";
import type { MsgValue } from "./msgpack.ts";
import { solidPng } from "./png.ts";
import { EXPLICIT, MINIMAL, type Style, type WriteOptions, writeBundle } from "./write.ts";

const REPO = join(import.meta.dirname, "../../..");

/** 往復させる元のデータ。 */
export interface Source {
  name: string;
  bundle: Omit<Bundle, "warnings">;
}

/** spec/examples/ の各ディレクトリの JSON の文書を 1 つのバンドルの中身にする。 */
export function exampleSources(): Source[] {
  const dir = join(REPO, "spec/examples");
  const out: Source[] = [];
  for (const name of readdirSync(dir).sort()) {
    const d = join(dir, name);
    if (!existsSync(join(d, "model.json"))) continue;
    const files = readdirSync(d).sort();
    const read = <T>(file: string, conv: (c: D.Ctx, v: MsgValue) => T): T => {
      const { value, warnings } = readJsonDocument(readFileSync(join(d, file), "utf8"), conv);
      if (warnings.length > 0) throw new Error(`${name}/${file}: ${warnings.join("; ")}`);
      return value;
    };
    const { model, warnings } = readModelJson(readFileSync(join(d, "model.json"), "utf8"));
    if (warnings.length > 0) throw new Error(`${name}/model.json: ${warnings.join("; ")}`);
    const physicsFile = files.find((f) => f.endsWith(".physics.json"));
    out.push({
      name,
      bundle: {
        model,
        physics: physicsFile ? read(physicsFile, D.physicsSettings) : null,
        motions: files.filter((f) => f.endsWith(".motion.json")).map((f) => read(f, D.motion)),
        expressions: files
          .filter((f) => f.endsWith(".expression.json"))
          .map((f) => read(f, D.expression)),
        meta: { example: name },
        textures: model.textures.map(() => ({
          format: "png" as const,
          bytes: solidPng(1, 1, [255, 255, 255, 255]),
        })),
        normalMaps: [],
      },
    });
  }
  return out;
}

/** assets/compat/dev/ の .ugoe (参照実装が書いたもの) をこの実装で読む。警告のあるものは書けないので除く。 */
export function compatSources(): Source[] {
  const dir = join(REPO, "assets/compat/dev");
  return readdirSync(dir)
    .filter((f) => f.endsWith(".ugoe"))
    .sort()
    .map((f) => {
      const { warnings, ...bundle } = readBundle(new Uint8Array(readFileSync(join(dir, f))));
      if (warnings.length > 0) throw new Error(`${f}: ${warnings.join("; ")}`);
      return { name: `compat/${f}`, bundle };
    });
}

/** spec/examples/bundles/ のバンドルのうち、読めるもの (知らない機能の警告は除いて書き直す)。 */
export function bundleExampleSources(): Source[] {
  const dir = join(REPO, "spec/examples/bundles");
  return loadManifest(dir)
    .files.filter((e) => e.result === "ok" && e.file.endsWith(".ugoe"))
    .map((e) => {
      const { warnings: _, ...bundle } = readBundle(
        new Uint8Array(readFileSync(join(dir, e.file))),
      );
      return { name: `bundles/${e.file}`, bundle };
    });
}

const VARIANTS: { name: string; style: Style; options: WriteOptions }[] = [
  { name: "minimal", style: MINIMAL, options: {} },
  {
    name: "explicit-zstd",
    style: EXPLICIT,
    options: { compress: ["MODL", "PHYS", "MOTN", "EXPR", "META"] },
  },
];

/** 2 つのバンドルの中身の食い違い (META は比べない。参照実装が書き直すことがある)。 */
export function bundleDifferences(a: Omit<Bundle, "warnings">, b: Omit<Bundle, "warnings">) {
  const diffs: string[] = [];
  for (const key of ["model", "physics", "motions", "expressions"] as const) {
    if (!isDeepStrictEqual(a[key], b[key])) diffs.push(`${key} differs`);
  }
  const images = (x: Omit<Bundle, "warnings">) => ({
    textures: x.textures.map((i) => Buffer.from(i.bytes).toString("hex")),
    normalMaps: x.normalMaps.map((i) => Buffer.from(i.bytes).toString("hex")),
  });
  if (!isDeepStrictEqual(images(a), images(b))) diffs.push("images differ");
  return diffs;
}

function run(ugoe: string, work: string, sources: Source[]): string[] {
  const failures: string[] = [];
  for (const src of sources) {
    for (const v of VARIANTS) {
      const label = `${src.name} (${v.name})`;
      const input = join(work, `${basename(src.name).replace(/\.ugoe$/, "")}.${v.name}.ugoe`);
      const output = input.replace(/\.ugoe$/, ".rebuilt.ugoe");
      const b = src.bundle;
      const bytes = writeBundle(
        {
          ...b,
          textures: b.textures.map((i) => i.bytes),
          normalMaps: b.normalMaps.map((i) => i.bytes),
        },
        { ...v.options, style: v.style },
      );
      writeFileSync(input, bytes);
      // この実装で読み戻す
      const own = readBundle(bytes);
      for (const d of bundleDifferences(b, own)) failures.push(`${label}: own reader: ${d}`);
      // 参照実装
      try {
        execFileSync(ugoe, ["validate", input], { stdio: "pipe" });
      } catch (e) {
        failures.push(`${label}: ugoe validate rejected: ${(e as { stderr: Buffer }).stderr}`);
        continue;
      }
      try {
        execFileSync(ugoe, ["build", input, "-o", output], { stdio: "pipe" });
      } catch (e) {
        failures.push(`${label}: ugoe build failed: ${(e as { stderr: Buffer }).stderr}`);
        continue;
      }
      const rebuilt = readBundle(new Uint8Array(readFileSync(output)));
      for (const w of rebuilt.warnings) failures.push(`${label}: warning after rebuild: ${w}`);
      for (const d of bundleDifferences(b, rebuilt)) failures.push(`${label}: after rebuild: ${d}`);
    }
  }
  return failures;
}

if (import.meta.main) {
  const [ugoe, workArg] = process.argv.slice(2);
  if (!ugoe) {
    console.error("usage: node tools/spec-check/src/roundtrip.ts <path to ugoe> [work dir]");
    process.exit(2);
  }
  const work = workArg ?? mkdtempSync(join(tmpdir(), "spec-check-roundtrip-"));
  mkdirSync(work, { recursive: true });
  const sources = [...exampleSources(), ...bundleExampleSources(), ...compatSources()];
  const failures = run(ugoe, work, sources);
  for (const f of failures) console.log(f);
  console.log(
    `${sources.length} sources × ${VARIANTS.length} variants, ${failures.length} failures (files in ${work})`,
  );
  process.exit(failures.length > 0 ? 1 : 0);
}
