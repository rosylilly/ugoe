// 使い方: node tools/spec-check/src/render-reference.ts <ugoe の実行ファイル> [作業ディレクトリ]
//
// 描画の例 (spec/examples/ の render.json のあるもの) を、参照実装に黒箱として描かせて比べる (T0127)。
// コマンドだけを使う: 例のモデルとテクスチャをこの実装で .ugoe に書き、`ugoe render <bundle> --size WxH
// --fit stretch [--ambient] [--light]` で PNG に描かせ、調べる画素を期待値と許容誤差で比べる。
//
// PNG は非乗算なので、読んだ色に alpha を掛けて乗算済みに戻す。そのため次は比べられない (飛ばして数える):
// alpha が 0 の画素の色、色が alpha を超える画素の色 (乗算済みの色が alpha を超えることは PNG で表せない)。
// CLI が受け付けない光源 (長さ 0 の向きなど) は渡さない。そのケースは光源を省いた形で比べ、そう書く。
// 参照実装の側の照合テスト (just test-gpu) とは別の、手で実行する確かめ。

import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { decodeRgbaPng } from "./png.ts";
import {
  loadRenderExample,
  PIXEL_TOLERANCE,
  type RenderCase,
  renderExampleBundle,
} from "./render-examples.ts";

const REPO = join(import.meta.dirname, "../../..");
const EXAMPLES = join(REPO, "spec/examples");

const [ugoe, work = mkdtempSync(join(tmpdir(), "spec-check-render-"))] = process.argv.slice(2);
if (!ugoe) {
  console.error("usage: node tools/spec-check/src/render-reference.ts <ugoe> [work dir]");
  process.exit(2);
}

/** CLI の光源の引数。受け付けられない光源は落とし、落としたかを返す。 */
function lightArgs(c: RenderCase): { args: string[]; dropped: boolean } {
  const L = c.lights;
  if (!L) return { args: [], dropped: false };
  const args = ["--ambient", (L.ambient ?? [1, 1, 1]).join(",")];
  let dropped = false;
  for (const d of L.directional ?? []) {
    if (!(Math.hypot(...d.direction) > 0)) {
      dropped = true;
      continue;
    }
    args.push("--light", `directional:${d.direction.join(",")}:${d.color.join(",")}`);
  }
  for (const p of L.point ?? []) {
    if (p.range !== undefined && !(p.range > 0)) {
      dropped = true;
      continue;
    }
    const range = p.range === undefined ? "" : `:${p.range}`;
    args.push("--light", `point:${p.position.join(",")}:${p.color.join(",")}${range}`);
  }
  return { args, dropped };
}

let failures = 0;
let skipped = 0;
const names = readdirSync(EXAMPLES)
  .filter((n) => existsSync(join(EXAMPLES, n, "render.json")))
  .sort();
for (const name of names) {
  const example = loadRenderExample(join(EXAMPLES, name));
  const bundle = join(work, `${name}.ugoe`);
  writeFileSync(bundle, renderExampleBundle(example));
  example.doc.cases.forEach((c, i) => {
    const png = join(work, `${name}-${i}.png`);
    const { args, dropped } = lightArgs(c);
    const params = Object.entries(c.parameters ?? {}).flatMap(([k, v]) => ["--param", `${k}=${v}`]);
    const size = `${c.target[0]}x${c.target[1]}`;
    execFileSync(
      ugoe,
      ["render", bundle, "-o", png, "--size", size, "--fit", "stretch", ...params, ...args],
      { stdio: ["ignore", "ignore", "inherit"] },
    );
    if (dropped) console.log(`${name} case ${i}: lights the CLI refuses were left out`);
    const image = decodeRgbaPng(new Uint8Array(readFileSync(png)));
    for (const p of c.pixels) {
      const at = 4 * (p.at[1] * image.width + p.at[0]);
      const a = image.pixels[at + 3];
      const got = [0, 1, 2].map((q) => (image.pixels[at + q] * a) / 255).concat([a]);
      for (let q = 0; q < 4; q++) {
        if (q < 3 && (p.rgba[3] === 0 || p.rgba[q] > p.rgba[3])) {
          skipped++;
          continue;
        }
        if (!(Math.abs(p.rgba[q] - got[q]) <= PIXEL_TOLERANCE)) {
          failures++;
          console.log(
            `${name} case ${i} pixel ${p.at}: expected ${p.rgba}, got ${got.map((x) => x.toFixed(1))}`,
          );
          break;
        }
      }
    }
  });
  console.log(`${name}: ${example.doc.cases.length} cases`);
}
console.log(
  `${failures} mismatches, ${skipped} channels not comparable through PNG (work dir ${work})`,
);
process.exit(failures > 0 ? 1 : 0);
