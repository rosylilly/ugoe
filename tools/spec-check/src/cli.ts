// 使い方: node tools/spec-check/src/cli.ts <model.json | .ugoe> [パラメータ=値 ...]
// ファイルを読み、与えたパラメータで評価した結果を JSON で標準出力に書く。警告は標準エラーに書く。

import { readFileSync } from "node:fs";
import { readBundle, readModelJson } from "./bundle.ts";
import { evaluate, parameterValues } from "./evaluate.ts";

const [path, ...assignments] = process.argv.slice(2);
if (!path) {
  console.error(
    "usage: node tools/spec-check/src/cli.ts <model.json | .ugoe> [Parameter=value ...]",
  );
  process.exit(2);
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
