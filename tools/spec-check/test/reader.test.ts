// 読み込みの規則 (spec/bundle.md#reading, spec/model.md#data-conventions) の細かい場合。
// コンテナは assets/compat/dev/single-quad.ugoe のペイロードを並べ直して作る。

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import {
  ReadError,
  type ReadErrorKind,
  readBundle,
  readChunkTable,
  readModelJson,
} from "../src/bundle.ts";
import * as D from "../src/decode.ts";
import { decode, encode, MAX_DEPTH, type MsgValue, peekKey } from "../src/msgpack.ts";
import { validateExpression, validateMotion } from "../src/validate.ts";
import { COMPAT, SPEC_EXAMPLES } from "./paths.ts";

interface RawChunk {
  tag: string;
  flags?: number;
  payload: Uint8Array;
  /** 置く位置を変える (既定は前のペイロードの後ろ、16 の倍数)。 */
  offset?: number;
}

/** チャンクの並びからコンテナを組み立てる (spec/bundle.md#container-layout)。 */
function container(
  chunks: RawChunk[],
  header: { version?: number; flags?: number } = {},
): Uint8Array {
  const tableEnd = 16 + 24 * chunks.length;
  const align = (n: number) => Math.ceil(n / 16) * 16;
  let pos = align(tableEnd);
  const placed = chunks.map((c) => {
    const offset = c.offset ?? pos;
    pos = Math.max(pos, align(offset + c.payload.length));
    return { ...c, offset };
  });
  const out = new Uint8Array(pos);
  const view = new DataView(out.buffer);
  out.set([0x55, 0x47, 0x4f, 0x45]);
  view.setUint32(4, header.version ?? 0, true);
  view.setUint32(8, header.flags ?? 0, true);
  view.setUint32(12, chunks.length, true);
  placed.forEach((c, i) => {
    const at = 16 + 24 * i;
    out.set(new TextEncoder().encode(c.tag), at);
    view.setUint32(at + 4, c.flags ?? 0, true);
    view.setBigUint64(at + 8, BigInt(c.offset), true);
    view.setBigUint64(at + 16, BigInt(c.payload.length), true);
    out.set(c.payload, c.offset);
  });
  return out;
}

const singleQuad = new Uint8Array(readFileSync(join(COMPAT, "dev/single-quad.ugoe")));
const parts = (() => {
  const table = readChunkTable(singleQuad);
  const get = (tag: string) => {
    const c = table.find((x) => x.tag === tag);
    assert.ok(c && c.flags === 0, `${tag} is stored uncompressed`);
    return singleQuad.slice(Number(c.offset), Number(c.offset + c.length));
  };
  return { modl: get("MODL"), texr: get("TEXR") };
})();

const rejects = (bytes: Uint8Array, kind: ReadErrorKind) =>
  assert.throws(
    () => readBundle(bytes),
    (e) => e instanceof ReadError && e.kind === kind,
  );

test("a rebuilt container reads like the original", () => {
  const b = readBundle(
    container([
      { tag: "MODL", payload: parts.modl },
      { tag: "TEXR", payload: parts.texr },
    ]),
  );
  assert.equal(b.model.meshes.length, 1);
  assert.equal(b.textures[0].format, "png");
});

test("header errors", () => {
  rejects(singleQuad.slice(0, 15), "header");
  const bad = singleQuad.slice();
  bad[0] = 0x58;
  rejects(bad, "header");
  const base = [
    { tag: "MODL", payload: parts.modl },
    { tag: "TEXR", payload: parts.texr },
  ];
  rejects(container(base, { version: 1 }), "container_version");
  rejects(container(base, { flags: 1 }), "header");
});

test("chunk table errors", () => {
  const texr = { tag: "TEXR", payload: parts.texr };
  rejects(container([{ tag: "MODL", payload: parts.modl, flags: 2 }, texr]), "chunk_table");
  rejects(container([{ tag: "MODL", payload: parts.modl, offset: 72 }, texr]), "chunk_table");
  // ペイロードが重なる
  rejects(
    container([
      { tag: "MODL", payload: parts.modl },
      { ...texr, offset: 64 },
    ]),
    "chunk_table",
  );
  // 表と重なる
  rejects(container([{ tag: "MODL", payload: parts.modl, offset: 48 }, texr]), "chunk_table");
  // 表が収まらない
  const cut = container([{ tag: "MODL", payload: parts.modl }, texr]);
  new DataView(cut.buffer).setUint32(12, 1000, true);
  rejects(cut, "chunk_table");
});

test("unknown chunks are skipped, even with zero length or junk payloads", () => {
  const b = readBundle(
    container([
      { tag: "XZER", payload: new Uint8Array() },
      { tag: "MODL", payload: parts.modl },
      { tag: "XJNK", flags: 1, payload: new Uint8Array([1, 2, 3]) },
      { tag: "TEXR", payload: parts.texr },
    ]),
  );
  assert.deepEqual(b.warnings, []);
});

test("chunks not allowed, duplicated or missing", () => {
  const texr = { tag: "TEXR", payload: parts.texr };
  const modl = { tag: "MODL", payload: parts.modl };
  rejects(container([modl, { tag: "MODU", payload: parts.modl }, texr]), "chunks");
  rejects(container([modl, modl, texr]), "chunks");
  rejects(container([texr]), "counts");
  rejects(container([modl]), "counts");
  rejects(container([modl, texr, texr]), "counts");
  rejects(container([modl, { tag: "TEXR", payload: new Uint8Array([1, 2, 3]) }]), "counts");
});

test("feature declarations", () => {
  // { "features": [ { "id": "test.x", "required": false } ] } を手で符号化する
  const feat = (required: boolean) =>
    new Uint8Array([
      0x81,
      0xa8,
      ...new TextEncoder().encode("features"),
      0x91,
      0x82,
      0xa2,
      ...new TextEncoder().encode("id"),
      0xa6,
      ...new TextEncoder().encode("test.x"),
      0xa8,
      ...new TextEncoder().encode("required"),
      required ? 0xc3 : 0xc2,
    ]);
  const rest = [
    { tag: "MODL", payload: parts.modl },
    { tag: "TEXR", payload: parts.texr },
  ];
  const optional = readBundle(container([{ tag: "FEAT", payload: feat(false) }, ...rest]));
  assert.deepEqual(optional.warnings, ['unknown optional feature "test.x"']);
  rejects(container([{ tag: "FEAT", payload: feat(true) }, ...rest]), "unsupported_features");
  // FEAT は 1 つまで
  rejects(
    container([
      { tag: "FEAT", payload: feat(false) },
      { tag: "FEAT", payload: feat(true) },
      ...rest,
    ]),
    "features",
  );
  // 必須の宣言は、本体が壊れていても先に拒否する
  rejects(
    container([
      { tag: "FEAT", payload: feat(true) },
      { tag: "MODL", payload: new Uint8Array([0xc1]) },
      { tag: "TEXR", payload: parts.texr },
    ]),
    "unsupported_features",
  );
});

test("MessagePack payloads with trailing bytes are rejected (allowed by spec/bundle.md)", () => {
  const modl = new Uint8Array([...parts.modl, 0xc0]);
  rejects(
    container([
      { tag: "MODL", payload: modl },
      { tag: "TEXR", payload: parts.texr },
    ]),
    "decode",
  );
});

test("MessagePack decoding", () => {
  assert.equal(
    decode(new Uint8Array([0xcf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])),
    2n ** 64n - 1n,
  );
  assert.equal(decode(new Uint8Array([0xca, 0x3f, 0xc0, 0x00, 0x00])), 1.5);
  assert.equal(decode(new Uint8Array([0xff])), -1n);
  const deep = new Uint8Array(MAX_DEPTH + 2).fill(0x91);
  deep[deep.length - 1] = 0xc0;
  assert.throws(() => decode(deep));
  // { "a": 1, "schema_version": 7 } から schema_version だけを読む
  const map = new Uint8Array([
    0x82,
    0xa1,
    0x61,
    0x92,
    0x01,
    0x02,
    0xae,
    ...new TextEncoder().encode("schema_version"),
    0x07,
  ]);
  assert.equal(peekKey(map, "schema_version"), 7n);
  assert.throws(
    () => decode(new Uint8Array([0x82, 0xa1, 0x61, 0x01, 0xa1, 0x61, 0x02])),
    /duplicate/,
  );
});

// ---- JSON の文書 ----

const quadJson = readFileSync(join(SPEC_EXAMPLES, "single-quad/model.json"), "utf8");

const jsonRejects = (text: string, pattern: RegExp) =>
  assert.throws(
    () => readModelJson(text),
    (e) => e instanceof ReadError && pattern.test(e.message),
  );

test("JSON: integers written as 1.0 are rejected", () => {
  jsonRejects(quadJson.replace('"width": 1,', '"width": 1.0,'), /integer/);
  jsonRejects(quadJson.replace('"width": 1,', '"width": 1.5,'), /integer/);
});

test("JSON: null for a field that is not nullable is rejected", () => {
  jsonRejects(quadJson.replace('"blend": "normal"', '"blend": null'), /null/);
});

test("JSON: optional fields may be absent and unknown fields are ignored", () => {
  const m = JSON.parse(quadJson);
  delete m.meshes[0].blend;
  delete m.pose;
  delete m.effects;
  m.meshes[0].future_field = { anything: [1, 2, 3] };
  const { model, warnings } = readModelJson(JSON.stringify(m));
  assert.equal(model.meshes[0].blend, "normal");
  assert.equal(model.pose.fade_duration, 0.5);
  assert.deepEqual(warnings, []);
});

test("JSON: unknown values get their fallback and a warning", () => {
  const m = JSON.parse(quadJson);
  m.meshes[0].blend = "overlay";
  m.meshes[0].keyforms.interpolation = "spline";
  const { model, warnings } = readModelJson(JSON.stringify(m));
  assert.equal(model.meshes[0].blend, "normal");
  assert.equal(model.meshes[0].keyforms.interpolation, "linear");
  assert.equal(warnings.length, 2);
});

test("an unknown VertexOwner makes the output unknown, with a warning", () => {
  const output = (owner: unknown) => ({
    target: { type: "vertices", owner, weights: [1] },
    particle: 1,
    source: "x",
    scale: 1,
    weight: 100,
  });
  const settings = (owner: unknown) => ({
    gravity: [0, 1],
    wind: [0, 0],
    fps: 60,
    rigs: [
      {
        id: "R",
        name: "R",
        inputs: [],
        outputs: [output({ warp: "W" }), output(owner)],
        particles: [],
        normalization: {
          position: { min: -1, default: 0, max: 1 },
          angle: { min: -1, default: 0, max: 1 },
        },
      },
    ],
    colliders: [],
  });
  const read = (owner: unknown) => {
    const warnings: string[] = [];
    const physics = D.physicsSettings(
      new D.Ctx(warnings),
      D.parseJson(JSON.stringify(settings(owner))),
    );
    return { targets: physics.rigs[0].outputs.map((o) => o.target), warnings };
  };
  for (const owner of [{ bone: { id: "B", segment: 2 } }, "all"]) {
    const { targets, warnings } = read(owner);
    assert.deepEqual(targets[0], {
      type: "vertices",
      owner: { kind: "warp", id: "W" },
      weights: [1],
    });
    assert.deepEqual(targets[1], { type: "unknown" });
    assert.equal(warnings.length, 1);
  }
  for (const owner of [{}, { mesh: "A", warp: "B" }, { mesh: 1 }, "mesh"]) {
    assert.throws(() => read(owner), JSON.stringify(owner));
  }
});

test("JSON: a newer schema_version is refused", () => {
  jsonRejects(quadJson.replace('"schema_version": 0', '"schema_version": 1'), /schema_version/);
});

test("validation reports every violation", () => {
  const m = JSON.parse(quadJson);
  m.meshes[0].texture = 5;
  m.meshes[0].indices.push(9);
  m.parts[0].parent = "Nowhere";
  assert.throws(
    () => readModelJson(JSON.stringify(m)),
    (e) => e instanceof ReadError && e.kind === "validation" && e.details.length === 4,
  );
});

// ---- モーションと表情の検証 (spec/model.md#motion-and-expression-validation) ----

const motionDoc = (): Record<string, unknown> => ({
  name: "m",
  duration: 1.0,
  fade_in: -0.5, // 負のフェードは違反ではない
  curves: [
    {
      target: { kind: "parameter", id: "X" },
      segments: [
        { type: "start", p: { t: 0.0, v: 0.0 } },
        { type: "bezier", c1: { t: -1.0, v: 2.0 }, c2: { t: 3.0, v: 2.0 }, to: { t: 0.5, v: 1.0 } },
      ],
      fade_in: 0.25,
    },
    // 知らない対象の曲線 (再生では無視する)
    {
      target: { kind: "future", id: 7 },
      segments: [{ type: "start", p: { t: 0.0, v: 0.0 } }],
    },
  ],
  events: [{ t: 5.0, value: "after the end" }],
});
const expressionDoc = (): Record<string, unknown> => ({
  name: "e",
  fade_in: 0.5,
  parameters: [
    { parameter: "X", value: 1.0, op: "add" },
    { parameter: "Y", value: 2.0, op: "future" },
  ],
});
const readMotion = (doc: unknown) =>
  D.motion(new D.Ctx([]), D.parseJson(JSON.stringify(doc).replaceAll('"INF"', "1e39")));
const readExpression = (doc: unknown) =>
  D.expression(new D.Ctx([]), D.parseJson(JSON.stringify(doc).replaceAll('"INF"', "1e39")));

test("f32: numbers too large for f32 convert to infinity (JSON and MessagePack)", () => {
  const c = new D.Ctx([]);
  assert.equal(D.f32(c, D.parseJson("1e39")), Number.POSITIVE_INFINITY);
  assert.equal(D.f32(c, D.parseJson("-1e39")), Number.NEGATIVE_INFINITY);
  assert.equal(D.f32(c, D.parseJson("1e400")), Number.POSITIVE_INFINITY);
  // 有限の f32 の最大値はそのまま
  assert.equal(D.f32(c, D.parseJson("3.4028234663852886e38")), 3.4028234663852886e38);
  // MessagePack の float 64 の 1e39 と、float 32 の無限大
  assert.equal(D.f32(c, decode(encode(1e39))), Number.POSITIVE_INFINITY);
  assert.equal(D.f32(c, decode(encode(Number.NEGATIVE_INFINITY))), Number.NEGATIVE_INFINITY);
  assert.ok(Number.isNaN(D.f32(c, decode(encode(Number.NaN)))));
});

test("motion validation: a valid motion (negative fades, points and events outside the duration)", () => {
  const m = readMotion(motionDoc());
  assert.equal(m.curves.length, 1);
  assert.equal(m.ignored_curves?.length, 1);
  assert.deepEqual(validateMotion(m), []);
});

test("motion validation: every rule", () => {
  const cases: [string, (d: Record<string, unknown>) => void][] = [
    ["negative duration", (d) => (d.duration = -1.0)],
    ["infinite duration", (d) => (d.duration = "INF")],
    ["fade_in", (d) => (d.fade_in = "INF")],
    ["fade_out", (d) => (d.fade_out = "INF")],
    ["curve fade_in", (d) => (curve(d, 0).fade_in = "INF")],
    ["curve fade_out", (d) => (curve(d, 0).fade_out = "INF")],
    ["start p.t", (d) => (seg(d, 0, 0).p = { t: "INF", v: 0.0 })],
    ["bezier c1.v", (d) => (seg(d, 0, 1).c1 = { t: 0.0, v: "INF" })],
    ["bezier c2.t", (d) => (seg(d, 0, 1).c2 = { t: "INF", v: 0.0 })],
    ["bezier to.v", (d) => (seg(d, 0, 1).to = { t: 0.5, v: "INF" })],
    ["event t", (d) => (d.events = [{ t: "INF", value: "x" }])],
    ["curve with an unknown target", (d) => (seg(d, 1, 0).p = { t: 0.0, v: "INF" })],
  ];
  for (const [name, edit] of cases) {
    const d = motionDoc();
    edit(d);
    assert.equal(validateMotion(readMotion(d)).length, 1, name);
  }
});

test("expression validation: every rule, including adjustments with an unknown op", () => {
  assert.deepEqual(validateExpression(readExpression(expressionDoc())), []);
  const cases: [string, (d: Record<string, unknown>) => void][] = [
    ["fade_in", (d) => (d.fade_in = "INF")],
    ["fade_out", (d) => (d.fade_out = "INF")],
    ["value", (d) => (adjustment(d, 0).value = "INF")],
    ["value with an unknown op", (d) => (adjustment(d, 1).value = "INF")],
  ];
  for (const [name, edit] of cases) {
    const d = expressionDoc();
    edit(d);
    assert.equal(validateExpression(readExpression(d)).length, 1, name);
  }
});

test("bundle: an invalid motion or expression is refused at validation", () => {
  const base = [
    { tag: "MODL", payload: parts.modl },
    { tag: "TEXR", payload: parts.texr },
  ];
  const withChunk = (tag: string, value: MsgValue) =>
    container([base[0], { tag, payload: encode(value) }, base[1]]);
  const motion = D.parseJson(JSON.stringify(motionDoc())) as Record<string, MsgValue>;
  const expression = D.parseJson(JSON.stringify(expressionDoc())) as Record<string, MsgValue>;
  // 正しいものは読める
  const ok = readBundle(withChunk("MOTN", motion));
  assert.equal(ok.motions.length, 1);
  assert.equal(readBundle(withChunk("EXPR", expression)).expressions.length, 1);
  // MessagePack の float 64 の 1e39 は無限大になる
  rejects(withChunk("MOTN", { ...motion, fade_out: 1e39 }), "validation");
  rejects(withChunk("MOTN", { ...motion, duration: -0.5 }), "validation");
  rejects(withChunk("EXPR", { ...expression, fade_in: Number.NaN }), "validation");
});

function curve(d: Record<string, unknown>, i: number): Record<string, unknown> {
  return (d.curves as Record<string, unknown>[])[i];
}
function seg(d: Record<string, unknown>, i: number, k: number): Record<string, unknown> {
  return (curve(d, i).segments as Record<string, unknown>[])[k];
}
function adjustment(d: Record<string, unknown>, i: number): Record<string, unknown> {
  return (d.parameters as Record<string, unknown>[])[i];
}
