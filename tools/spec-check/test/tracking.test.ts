// 追従の対応 (spec/avatar-profile.md#computing-the-parameter-values) の細かい場合。
// 時間の例 (spec/examples/tracking) で確かめられないもの: 入力に値が無い上書き・値が有限でない場合・
// 知らない source の要素・順によらないこと。

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { readModelJson } from "../src/bundle.ts";
import { STANDARD_MAPPING, trackingValues } from "../src/tracking.ts";
import type * as M from "../src/types.ts";
import { SPEC_EXAMPLES } from "./paths.ts";

/** spec/examples/tracking のモデルの `avatar.tracking` を差し替えたもの。 */
function model(tracking: unknown[]): M.Model {
  const doc = JSON.parse(readFileSync(join(SPEC_EXAMPLES, "tracking", "model.json"), "utf8"));
  doc.avatar.tracking = tracking;
  return readModelJson(JSON.stringify(doc)).model;
}

const head = { yaw: 10, pitch: 0, roll: 0 };

test("標準の対応は 32 のパラメータ", () => {
  assert.equal(STANDARD_MAPPING.size, 32);
});

test("入力に値が無い要素があれば、対象は値を持たない (書かない)", () => {
  const m = model([
    { source: "standard", input: "ShoulderLY", parameter: "Cheek", offset: 0.5 },
    { source: "blend_shape", input: "cheekPuff", parameter: "Cheek" },
  ]);
  assert.equal(trackingValues(m, { head }).has("Cheek"), false);
  const v = trackingValues(m, { head, shoulders: { left: 0.1 }, blend_shapes: { cheekPuff: 0.2 } });
  assert.ok(Math.abs((v.get("Cheek") ?? Number.NaN) - 0.8) < 1e-12);
});

test("値が有限でなければ書かない", () => {
  const v = trackingValues(model([]), { head, blend_shapes: { eyeBlinkLeft: Number.NaN } });
  assert.equal(v.has("EyeLOpen"), false);
  assert.equal(v.get("EyeROpen"), 1);
});

test("対象が internal の要素も標準の対応を置き換える", () => {
  const v = trackingValues(
    model([{ source: "standard", input: "MouthOpenY", parameter: "BrowLY" }]),
    {
      head,
      blend_shapes: { jawOpen: 0.5 },
    },
  );
  assert.equal(v.has("MouthOpenY"), false);
  assert.equal(v.has("BrowLY"), false);
});

test("知らない source の要素は無視する (警告して読む)", () => {
  const m = model([{ source: "future_tracker", input: "x", parameter: "AngleX", scale: 2 }]);
  assert.equal(m.avatar?.tracking.length, 0);
  assert.equal(trackingValues(m, { head }).get("AngleX"), 10);
});

test("結果は要素の順によらない", () => {
  const items = [
    { source: "head", input: "yaw", parameter: "AngleX" },
    { source: "standard", input: "BodyAngleX", parameter: "AngleX", scale: -1 },
    { source: "standard", input: "EyeBallX", scale: -1 },
    { source: "blend_shape", input: "jawForward", parameter: "JawForward", scale: 0.5 },
  ];
  const input = {
    head: { yaw: 12, pitch: 1, roll: 2 },
    body: { yaw: 4 },
    blend_shapes: { eyeLookOutLeft: 0.4, jawForward: 0.8 },
  };
  const a = trackingValues(model(items), input);
  const b = trackingValues(model([...items].reverse()), input);
  assert.deepEqual([...a].sort(), [...b].sort());
  assert.equal(a.get("AngleX"), 8);
  assert.equal(a.has("BodyAngleX"), false);
});
