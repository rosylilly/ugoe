// 適合テストの `queries` の実行器 (tests/conformance/README.md、T0463)。
//
// run.mjs のサーバーが配るページで動く。本物の wasm (npm パッケージ @ugoe/runtime) の `WebPuppet` の `*_json()` と
// `sample_motion_json`・`MotionSampler.sample` の出力のキーを、パッケージの型 (bindings/web/src/queries.ts) のキーの一覧
// (`QUERY_KEYS`、bindings/web/src/query-keys.ts) と比べる。一覧と型が同じことは bindings/web の `pnpm check` が確かめる。
//
// - 見本のモデル (/samples/。assets/samples) を読み、着せ替えのモジュールも付けて、すべての形を出させる。見本が持たない
//   ポーズとパラメータのグループは、フィクスチャ `pose_and_effects` (run.mjs が `ugoe fixture` で書き出す。/fixture/) に
//   グループを足した JSON のモデルで出させる。
//   どれかの形を 1 度も比べなかったら失敗にする (見本が変わって確かめられなくなったことに気づくため)。
// - 結果は /result/queries.json (項目ごとに "ok" か食い違いの一覧と、形ごとに比べた数)。

import { createPuppet, loadModel, MotionSampler, sampleMotion } from "/pkg/index.js";
import { QueryKeyCheck } from "/pkg/query-keys.js";

/** 読むモデルと、付けるモジュール (`dress-up.jpg` の組み合わせ)。 */
const MODELS = [
  { name: "avatar", url: "/samples/avatar.ugoe", modules: [] },
  { name: "avatar-lit", url: "/samples/avatar-lit.ugoe", modules: [] },
  {
    name: "dress-up",
    url: "/samples/dress-up/dress-up.ugoe",
    modules: ["long-hair", "cat-ears", "sailor"],
    // 付けずに読むだけのモジュール (`attached: false` の要素も比べる)
    loaded: ["twin-tails"],
  },
];

/** カーブの対象の 3 種類を持つモーション (`sample_motion_json`・`MotionSampler` で評価する)。 */
const MOTION = JSON.stringify({
  name: "queries",
  duration: 1,
  looping: false,
  fade_in: 0,
  fade_out: 0,
  curves: [
    ["parameter", "AngleX"],
    ["part_opacity", "Face"],
    ["effect_group", "EyeBlink"],
  ].map(([kind, id]) => ({
    target: { kind, id },
    segments: [
      { type: "start", p: { t: 0, v: 0 } },
      { type: "linear", to: { t: 1, v: 1 } },
    ],
  })),
  events: [],
});

async function post(path, body) {
  const response = await fetch(path, { method: "POST", body });
  if (!response.ok) throw new Error(`POST ${path}: ${response.status}`);
}

async function bytes(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`GET ${url}: ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

/** WebGL2 の `WebPuppet` を作って `use` に渡し、終わったら解放する。 */
async function withPuppet(use) {
  const canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  document.body.append(canvas);
  const puppet = await createPuppet(canvas, { backend: "webgl2" });
  try {
    await use(puppet);
  } finally {
    puppet.free();
    canvas.remove();
  }
}

const check = new QueryKeyCheck();

/** 項目の名前 → 項目を確かめる関数。食い違いは `check.errors` に貯まる。 */
const items = {
  /** モデルの無い `WebPuppet` (空の一覧と `pose_json()` の `undefined`)。 */
  async "no-model"() {
    await withPuppet((puppet) => check.puppet(puppet, "no-model"));
  },
  async "sample-motion"() {
    const values = sampleMotion(MOTION, 0.5);
    if (values.length !== 3) throw new Error(`sample_motion_json gave ${values.length} values`);
    check.list(values, "curve_value", "sample_motion_json");
    const sampler = new MotionSampler(MOTION);
    try {
      check.list(sampleMotion(sampler, 0.5), "curve_value", "MotionSampler.sample");
    } finally {
      sampler.free();
    }
  },
};
items["pose-and-groups"] = () =>
  withPuppet(async (puppet) => {
    const model = await (await fetch("/fixture/pose_and_effects.json")).json();
    // 見本もフィクスチャもパラメータのグループを持たないので足す (表示用で、評価には効かない)
    model.parameter_groups = [{ id: "Group", name: "Group" }];
    model.parameters[0].group = "Group";
    loadModel(puppet, JSON.stringify(model));
    puppet.update(1 / 60);
    check.puppet(puppet, "pose-and-groups");
  });
for (const model of MODELS) {
  items[model.name] = () =>
    withPuppet(async (puppet) => {
      loadModel(puppet, await bytes(model.url));
      for (const name of [...model.modules, ...(model.loaded ?? [])]) {
        const loaded = puppet.load_module(await bytes(`/samples/dress-up/${name}.ugoemod`));
        if (model.modules.includes(name)) puppet.attach_module(loaded);
      }
      // 評価の結果 (`drawable_states_json`) とポーズを今の値にする
      puppet.update(1 / 60);
      check.puppet(puppet, model.name);
    });
}

async function main() {
  const results = {};
  let failed = 0;
  for (const [name, run] of Object.entries(items)) {
    await post("/start", name);
    const before = check.errors.length;
    try {
      await run();
    } catch (e) {
      check.errors.push(`${name}: ${e?.stack ?? e}`);
    }
    const errors = check.errors.slice(before);
    results[name] = errors.length === 0 ? "ok" : errors;
    if (errors.length > 0) {
      failed += 1;
      // 同じ食い違いが要素の数だけ並ぶので、ログは先頭だけ (すべては queries.json)
      const shown = errors.slice(0, 10);
      if (errors.length > shown.length) shown.push(`... and ${errors.length - shown.length} more`);
      await post("/log", `${name}:\n  ${shown.join("\n  ")}`);
    }
  }
  // 見本がすべての形を出したか
  const unseen = check.unseen();
  results.coverage = unseen.length === 0 ? "ok" : [`shapes never seen: ${unseen.join(", ")}`];
  if (unseen.length > 0) {
    failed += 1;
    await post("/log", `coverage: shapes never seen: ${unseen.join(", ")}`);
  }
  await post("/result/queries.json", JSON.stringify({ results, seen: check.seen }, null, 2));
  await post("/done", JSON.stringify({ total: Object.keys(results).length, failed }));
}

main().catch((e) => post("/done", JSON.stringify({ error: String(e?.stack ?? e) })));
