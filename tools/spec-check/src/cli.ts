// 使い方: node tools/spec-check/src/cli.ts <model.json | .ugoe> [パラメータ=値 ...]
// ファイルを読み、与えたパラメータで評価した結果を JSON で標準出力に書く。警告は標準エラーに書く。
// 時間の例のディレクトリ (timeline.json のあるもの) を与えると、フレームごとの食い違いを書く。

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { readBundle, readModelJson } from "./bundle.ts";
import { evaluate, parameterValues } from "./evaluate.ts";
import { loadTimeline, runTimeline } from "./timeline.ts";

const [path, ...assignments] = process.argv.slice(2);
if (!path) {
  console.error(
    "usage: node tools/spec-check/src/cli.ts <model.json | .ugoe | timeline example dir> [Parameter=value ...]",
  );
  process.exit(2);
}

if (existsSync(join(path, "timeline.json"))) {
  const failures = runTimeline(loadTimeline(path));
  for (const f of failures) console.log(f);
  process.exit(failures.length > 0 ? 1 : 0);
}

const { model, warnings } = path.endsWith(".json")
  ? readModelJson(readFileSync(path, "utf8"))
  : readBundle(new Uint8Array(readFileSync(path)));
for (const w of warnings) console.error(`warning: ${w}`);

const set = Object.fromEntries(
  assignments.map((a) => {
    const [id, value] = a.split("=");
    return [id, Number(value)];
  }),
);
const result = evaluate(model, parameterValues(model, set));
console.log(
  JSON.stringify(
    {
      meshes: Object.fromEntries(result.meshes.map(({ id, ...rest }) => [id, rest])),
      render_order: result.render_order.map((i) => result.meshes[i].id),
    },
    null,
    2,
  ),
);
