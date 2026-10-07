// 適合テストの `queries` の実行器 (tests/conformance/README.md、T0463)。
//
// run.mjs のサーバーが配るページで動く。本物の wasm (npm パッケージ @ugoe/runtime) の `WebPuppet` の `*_json()` と
// `sample_motion_json`・`MotionSampler.sample` の出力のキーを、パッケージの型 (bindings/web/src/queries.ts) のキーの一覧
// (`QUERY_KEYS`、bindings/web/src/query-keys.ts) と比べる。一覧と型が同じことは bindings/web の `pnpm check` が確かめる。
//
// - 見本のモデル (/samples/。assets/samples) を読み、着せ替えのモジュールも付けて、すべての形を出させる (ポーズと
//   パラメータのグループも見本が持つ。T0468)。
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
// 直前の update のイベント (`motion_events_json`。T0462)。最初の update で発火するイベントを持つモーションを見本で再生する
items["motion-events"] = () =>
  withPuppet(async (puppet) => {
    loadModel(puppet, await bytes("/samples/avatar.ugoe"));
    const motion = {
      name: "events",
      duration: 1,
      looping: false,
      fade_in: 0,
      fade_out: 0,
      curves: [],
      events: [{ t: 0, value: "start" }],
    };
    if (!puppet.start_motion_json(JSON.stringify(motion)))
      throw new Error("the motion did not start");
    puppet.update(1 / 60);
    const events = JSON.parse(puppet.motion_events_json());
    if (events.length !== 1) throw new Error(`motion_events_json gave ${events.length} events`);
    check.puppet(puppet, "motion-events");
  });
// ホストから来る範囲の外の番号・知らない ID・極端な数で、wasm が trap せず (`WebAssembly.RuntimeError` にならず)、
// 既定の値か JS の例外を返し、続けて使えること (docs/portability.md#パニックしない。T0493)
items["host-input"] = () =>
  withPuppet(async (puppet) => {
    loadModel(puppet, await bytes("/samples/avatar.ugoe"));
    const NONE = "__no_such_id__";
    const BIG = 0xffffffff;
    const ANY = Symbol("any");
    /** `call` を呼ぶ。投げたら trap でないこと、返したら `expected` (関数なら判定) に合うことを確かめる。 */
    const probe = (name, call, expected = ANY) => {
      let value;
      try {
        value = call();
      } catch (e) {
        if (e instanceof WebAssembly.RuntimeError)
          check.errors.push(`host-input: ${name} trapped: ${e}`);
        return;
      }
      if (expected === ANY) return;
      const ok = typeof expected === "function" ? expected(value) : Object.is(value, expected);
      if (!ok) check.errors.push(`host-input: ${name} returned ${JSON.stringify(value)}`);
    };
    const empty = (v) => v.length === 0;
    probe("drawable_positions", () => puppet.drawable_positions(BIG), undefined);
    probe("physics_particles", () => puppet.physics_particles(BIG), empty);
    probe("set_parameter", () => puppet.set_parameter(NONE, 1), false);
    probe("set_parameter NaN", () => puppet.set_parameter("AngleX", Number.NaN));
    probe("parameter", () => puppet.parameter(NONE), undefined);
    probe("part_evaluated_opacity", () => puppet.part_evaluated_opacity(NONE), undefined);
    probe("deformer_point", () => puppet.deformer_point(NONE, 0, 0), undefined);
    probe("deformer_jacobian", () => puppet.deformer_jacobian(NONE, 0, 0), undefined);
    probe("warp_points", () => puppet.warp_points(NONE), undefined);
    probe("rotation_frame", () => puppet.rotation_frame(NONE), undefined);
    probe("rotation_local_keyform", () => puppet.rotation_local_keyform(NONE), undefined);
    probe("hit_area_test", () => puppet.hit_area_test(NONE, 0, 0), false);
    probe("hit_test NaN", () => puppet.hit_test(Number.NaN, Number.NaN));
    probe("hit_test_standard", () => puppet.hit_test_standard(NONE, 0, 0), false);
    probe("show_pose", () => puppet.show_pose(NONE), false);
    probe("show_pose_immediately", () => puppet.show_pose_immediately(NONE), false);
    probe("start_motion", () => puppet.start_motion(NONE, undefined));
    probe("start_motion priority", () => puppet.start_motion(NONE, NONE));
    probe("set_expression", () => puppet.set_expression(NONE));
    probe("add_expression", () => puppet.add_expression(NONE));
    probe("stop_expression", () => puppet.stop_expression(NONE));
    probe("set_enabled", () => puppet.set_enabled(NONE, true));
    probe("is_enabled", () => puppet.is_enabled(NONE));
    probe("attach_module", () => puppet.attach_module(NONE));
    probe("detach_module", () => puppet.detach_module(NONE));
    probe("unload_module", () => puppet.unload_module(NONE));
    probe("load_module", () => puppet.load_module(new Uint8Array([0, 1, 2, 3])));
    probe("set_tracking_kept", () => puppet.set_tracking_kept([NONE]));
    probe("set_tracking", () =>
      puppet.set_tracking(
        [NONE, "eyeBlinkLeft"],
        new Float32Array([Number.NaN]),
        1e30,
        -1e30,
        Number.NaN,
      ),
    );
    probe("set_base_values", () => puppet.set_base_values(new Float32Array(1)));
    const values = puppet.values_before_physics();
    probe("follow", () =>
      puppet.follow(
        values,
        1 / 60,
        Number.NaN,
        Infinity,
        new Uint32Array([BIG]),
        new Float32Array([1, 1]),
      ),
    );
    probe("follow pairs", () =>
      puppet.follow(
        values,
        1 / 60,
        undefined,
        undefined,
        new Uint32Array([0]),
        new Float32Array(1),
      ),
    );
    probe("set_effects_json", () => puppet.set_effects_json("{"));
    probe("set_physics_json", () => puppet.set_physics_json("[]"));
    probe("set_lighting_json", () => puppet.set_lighting_json("null"));
    probe("set_expression_json", () => puppet.set_expression_json("{}"));
    probe("start_motion_json", () => puppet.start_motion_json("{}"));
    probe("set_lip_sync_level", () => puppet.set_lip_sync_level(Infinity));
    for (const dt of [Number.NaN, Infinity, -1, 1e30])
      probe(`update ${dt}`, () => puppet.update(dt));
    probe("set_view", () => puppet.set_view(Number.NaN, Infinity, -Infinity));
    probe("zoom_at", () => puppet.zoom_at(0, Number.NaN, Number.NaN));
    // 続けて使える (trap の後の `WebPuppet` は Rust の状態が途中のまま残る)
    puppet.fit();
    puppet.update(1 / 60);
    puppet.render();
    check.puppet(puppet, "host-input");
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
