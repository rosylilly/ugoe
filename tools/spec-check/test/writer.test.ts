// 書き出し (spec/bundle.md#writing) の試験: MessagePack の符号化、フィールドの書き方、知らない値の拒否、
// コンテナの配置、この実装の読み込みとの往復。参照実装との往復は src/roundtrip.ts (手で実行する)。

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { readBundle, readChunkTable, readModelJson } from "../src/bundle.ts";
import { decode, encode, type MsgValue } from "../src/msgpack.ts";
import { solidPng } from "../src/png.ts";
import {
  bundleDifferences,
  bundleExampleSources,
  compatSources,
  exampleSources,
} from "../src/roundtrip.ts";
import type * as M from "../src/types.ts";
import {
  EXPLICIT,
  encodeValue,
  MINIMAL,
  WriteError,
  writeBundle,
  writeContainer,
} from "../src/write.ts";
import { SPEC_EXAMPLES } from "./paths.ts";

const singleQuad = (): M.Model =>
  readModelJson(readFileSync(join(SPEC_EXAMPLES, "single-quad/model.json"), "utf8")).model;
const white = solidPng(1, 1, [255, 255, 255, 255]);

/** 復号した map (プロトタイプの無いオブジェクト) を、比べられるように普通のオブジェクトにする。 */
function plain(v: MsgValue): MsgValue {
  if (Array.isArray(v)) return v.map(plain);
  if (v === null || typeof v !== "object" || v instanceof Uint8Array) return v;
  return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, plain(x)]));
}

const decodeObject = (bytes: Uint8Array) => plain(decode(bytes)) as Record<string, MsgValue>;

test("MessagePack: 整数は最も短い形で書き、読み戻せる", () => {
  const cases: [bigint, number][] = [
    [0n, 1],
    [127n, 1],
    [128n, 2],
    [255n, 2],
    [256n, 3],
    [65535n, 3],
    [65536n, 5],
    [2n ** 32n - 1n, 5],
    [2n ** 32n, 9],
    [2n ** 64n - 1n, 9],
    [-1n, 1],
    [-32n, 1],
    [-33n, 2],
    [-128n, 2],
    [-129n, 3],
    [-32768n, 3],
    [-32769n, 5],
    [-(2n ** 31n), 5],
    [-(2n ** 31n) - 1n, 9],
    [-(2n ** 63n), 9],
  ];
  for (const [v, length] of cases) {
    const bytes = encode(v);
    assert.equal(bytes.length, length, `${v}`);
    assert.equal(decode(bytes), v);
  }
  assert.throws(() => encode(2n ** 64n));
});

test("MessagePack: 単精度で表せる数は float 32、ほかは float 64", () => {
  assert.equal(encode(0.5)[0], 0xca);
  assert.equal(encode(Math.fround(0.1))[0], 0xca);
  assert.equal(encode(0.1)[0], 0xcb);
  assert.equal(decode(encode(Math.fround(0.1))), Math.fround(0.1));
  assert.equal(decode(encode(0.1)), 0.1);
});

test("MessagePack: 文字列・配列・map・bin の長さの境目", () => {
  const values: MsgValue[] = [
    "",
    "a".repeat(31),
    "a".repeat(32),
    "a".repeat(256),
    "a".repeat(65536),
    "日本語",
    Array.from({ length: 15 }, () => null),
    Array.from({ length: 16 }, () => true),
    Array.from({ length: 65536 }, () => 1n),
    Object.fromEntries(Array.from({ length: 15 }, (_, i) => [`k${i}`, BigInt(i)])),
    Object.fromEntries(Array.from({ length: 16 }, (_, i) => [`k${i}`, [false]])),
    new Uint8Array([1, 2, 3]),
    { nested: { deeper: [1.5, -2n, "x", null] } },
  ];
  for (const v of values) {
    assert.deepEqual(plain(decode(encode(v))), v);
  }
});

test("整数のフィールドは整数、f32 は float 32 で書く", () => {
  const value = decodeObject(encode(encodeValue("model", singleQuad())));
  const mesh = (value.meshes as Record<string, MsgValue>[])[0];
  assert.equal(value.schema_version, 0n);
  assert.equal(mesh.texture, 0n);
  assert.deepEqual((mesh.indices as MsgValue[]).slice(0, 3), [0n, 1n, 2n]);
  const texture = (value.textures as Record<string, MsgValue>[])[0];
  assert.equal(texture.width, 1n);
  const grid = mesh.keyforms as Record<string, MsgValue>;
  const keyform = (grid.keyforms as Record<string, MsgValue>[])[0];
  assert.equal(typeof keyform.opacity, "number");
  assert.equal(typeof keyform.draw_order, "number");
  // f32 は整数の値でも float 32
  const bytes = encode(keyform.opacity as number);
  assert.equal(bytes[0], 0xca);
});

test("minimal は既定値と値の無いフィールドを省き、explicit は書く", () => {
  const m = singleQuad();
  const minimal = decodeObject(encode(encodeValue("model", m, MINIMAL)));
  const explicit = decodeObject(encode(encodeValue("model", m, EXPLICIT)));
  for (const key of ["deformers", "glues", "hit_areas", "pose", "effects", "slots", "lighting"]) {
    assert.ok(!(key in minimal), key);
    assert.ok(key in explicit, key);
  }
  assert.ok(!("avatar" in minimal));
  assert.equal(explicit.avatar, null);
  const meshMin = (minimal.meshes as Record<string, MsgValue>[])[0];
  const meshExp = (explicit.meshes as Record<string, MsgValue>[])[0];
  assert.ok(!("parent" in meshMin) && !("blend" in meshMin) && !("lit" in meshMin));
  assert.equal(meshExp.parent, null);
  assert.equal(meshExp.blend, "normal");
  // 作るときのデータは explicit でも既定値なら書かない
  assert.ok(!("mirror" in meshExp) && !("depth" in meshExp));
  const grid = meshExp.keyforms as Record<string, MsgValue>;
  assert.ok(!("origins" in grid) && !("inference" in grid) && !("mirror_source" in grid));
});

test("知らない値は書けない", () => {
  const m = singleQuad();
  m.parameters[0].kind = "unknown";
  assert.throws(() => encodeValue("model", m), WriteError);

  const d = singleQuad();
  d.deformers.push({
    id: "D",
    name: "D",
    part: "Root",
    parent: null,
    kind: { type: "unknown" },
    mirror: null,
    mirror_axis: null,
  });
  assert.throws(() => encodeValue("model", d), WriteError);

  const physics: M.PhysicsSettings = {
    gravity: [0, 1],
    wind: [0, 0],
    fps: 30,
    colliders: [{ id: "C", name: "C", attach: null, shape: { type: "unknown" } }],
    rigs: [],
  };
  assert.throws(() => encodeValue("physics", physics), WriteError);
});

test("基準の機能は宣言できない", () => {
  assert.throws(
    () =>
      writeBundle(
        { model: singleQuad(), textures: [white] },
        { features: [{ id: "modules", required: false }] },
      ),
    WriteError,
  );
});

test("画像の数がテクスチャと合わなければ書かない", () => {
  assert.throws(() => writeBundle({ model: singleQuad(), textures: [] }), WriteError);
  assert.throws(
    () => writeBundle({ model: singleQuad(), textures: [white], normalMaps: [white] }),
    WriteError,
  );
});

test("コンテナ: 16 バイト境界、詰め物は 0、ペイロードの順は指定どおり", () => {
  const chunks = [
    { tag: "AAAA", data: new Uint8Array([1, 2, 3]) },
    { tag: "BBBB", data: new Uint8Array(0) },
    { tag: "CCCC", data: new Uint8Array(17).fill(9) },
  ];
  const bytes = writeContainer(chunks, { payloadOrder: [2, 0, 1] });
  const table = readChunkTable(bytes);
  assert.deepEqual(
    table.map((c) => [c.tag, Number(c.offset), Number(c.length)]),
    [
      ["AAAA", 128, 3],
      ["BBBB", 144, 0],
      ["CCCC", 96, 17],
    ],
  );
  assert.equal(bytes.length, 144);
  // 表の後ろ (88..96)、CCCC の後ろ (113..128)、AAAA の後ろ (131..144) は 0
  for (const [from, to] of [
    [88, 96],
    [113, 128],
    [131, 144],
  ]) {
    assert.ok(bytes.subarray(from, to).every((b) => b === 0));
  }
});

test("コンテナ: 長さ 0 のチャンクの offset はそのまま書き (AAAA の中でもよい)、ほかの置き場所を変えない", () => {
  const bytes = writeContainer([
    { tag: "AAAA", data: new Uint8Array(40).fill(1) },
    { tag: "BBBB", data: new Uint8Array(0), offset: 112 },
    { tag: "CCCC", data: new Uint8Array([2]) },
  ]);
  assert.deepEqual(
    readChunkTable(bytes).map((c) => [c.tag, Number(c.offset), Number(c.length)]),
    [
      ["AAAA", 96, 40],
      ["BBBB", 112, 0],
      ["CCCC", 144, 1],
    ],
  );
  // 長さのあるペイロードには付けられない
  assert.throws(
    () => writeContainer([{ tag: "AAAA", data: new Uint8Array([1]), offset: 96 }]),
    WriteError,
  );
});

test("u64 の seed を正確に往復させる", () => {
  const m = singleQuad();
  m.effects.eye_blink.seed = 2n ** 64n - 1n;
  const back = readBundle(writeBundle({ model: m, textures: [white] }));
  assert.equal(back.model.effects.eye_blink.seed, 2n ** 64n - 1n);
});

const sources = [...exampleSources(), ...bundleExampleSources(), ...compatSources()];
for (const src of sources) {
  test(`この実装で書いて読み戻す: ${src.name}`, () => {
    const b = src.bundle;
    for (const [style, compress] of [
      [MINIMAL, []],
      [EXPLICIT, ["MODL", "PHYS", "MOTN", "EXPR", "META"]],
    ] as const) {
      const bytes = writeBundle(
        {
          ...b,
          textures: b.textures.map((i) => i.bytes),
          normalMaps: b.normalMaps.map((i) => i.bytes),
        },
        { style, compress: [...compress] },
      );
      const back = readBundle(bytes);
      assert.deepEqual(back.warnings, []);
      assert.deepEqual(bundleDifferences(b, back), []);
      assert.deepEqual(back.meta, b.meta);
    }
  });
}
