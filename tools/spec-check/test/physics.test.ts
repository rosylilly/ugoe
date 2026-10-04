// 物理の時間の進め方 (spec/physics.md#advancing-time) のうち、時間の例では確かめにくい場合。
// 例のファイルは spec/ (CC BY 4.0) から実行時に読み、このディレクトリには写さない。

import assert from "node:assert/strict";
import { join } from "node:path";
import { test } from "node:test";
import { Runtime } from "../src/frame.ts";
import { loadTimeline } from "../src/timeline.ts";
import { SPEC_EXAMPLES } from "./paths.ts";

/** 有限の f32 の最大値。 */
const F32_MAX = 3.4028234663852886e38;
const OUTPUTS = ["SwayAngle", "Tip", "SwayX"];

test("8 刻みを超える dt の後、残りの時間は割り算の余りになり、物理は進み続ける", () => {
  const ex = loadTimeline(join(SPEC_EXAMPLES, "physics-pendulum"));
  const physics = ex.physics;
  assert.ok(physics);
  // fps 75 (step = 1/75) では、F32_MAX − whole × step は倍精度でも負の大きな値になる
  // (余りでなく引き算なら、補間の係数が 0 に切り詰められ、残りの時間が失われる)。
  const fps = 75;
  const step = 1 / fps;
  assert.ok(F32_MAX - Math.floor(F32_MAX / step) * step < 0, "引き算が壊れる fps と dt の組");
  const remainder = F32_MAX % step;
  assert.ok(remainder >= 0 && remainder < step);
  assert.ok(remainder / step > 0.5, "補間の係数が 0 と区別できること");

  // 巨大な dt で進めるものと、同じ 8 刻みと同じ余りを持つ dt で進めるもの
  const run = (big: number) => {
    const rt = new Runtime(ex.model, { ...physics, fps });
    // 目パチは巨大な dt で相を数え切れないので切る (相の数の上限は任意。spec/motion.md#eye-blink)
    rt.setStage("eye_blink", false);
    rt.frame(step); // 静止の姿勢に置く
    rt.set("X", 6);
    const out = [rt.frame(big).values];
    for (let i = 0; i < 4; i++) out.push(rt.frame(step).values);
    return out.map((v) => OUTPUTS.map((id) => v.get(id) ?? Number.NaN));
  };
  const huge = run(F32_MAX);
  const exact = run(8 * step + remainder);
  huge.forEach((values, f) => {
    values.forEach((v, k) => {
      assert.ok(Number.isFinite(v), `frame ${f} ${OUTPUTS[k]}: ${v}`);
      assert.ok(
        Math.abs(v - exact[f][k]) < 1e-6,
        `frame ${f} ${OUTPUTS[k]}: ${v} ≠ ${exact[f][k]}`,
      );
    });
  });
  // 物理が進み続けている (根を動かした後、出力が変わり続ける)
  for (let f = 1; f < huge.length; f++) assert.notDeepEqual(huge[f], huge[f - 1]);
});
