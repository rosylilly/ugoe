// 復号した JSON / MessagePack の値を、型の決まったデータにする (spec/model.md)。
//
// - 既定値のあるフィールドが無ければ既定値を入れる (spec/model.md#presence-and-default-values)。
// - 知らないフィールドは無視する (spec/model.md#objects)。
// - 拡張できる列挙の知らない値は、フォールバックを当てて警告を残す (spec/model.md#unknown-values)。
//   この実装は書き出しをしないので、元の値は残さない。フォールバックが検証に影響する
//   `Parameter.kind` とデフォーマの種類だけは「知らない値」として残す。
// - 型の合わない値は `DecodeError` (ペイロードが不正)。モデルの規則の検査は validate.ts。

import type { MsgValue } from "./msgpack.ts";
import type * as M from "./types.ts";

export class DecodeError extends Error {}

type Obj = { [key: string]: MsgValue };

/**
 * 復号の文脈: 今の位置 (エラーと警告の場所) と警告の集まり。
 * 入力の値は、整数を `bigint`、浮動小数点数を `number` で表す (JSON は `parseJson` で読む)。
 */
export class Ctx {
  readonly warnings: string[];
  readonly path: string;

  constructor(warnings: string[] = [], path = "") {
    this.warnings = warnings;
    this.path = path;
  }

  at(key: string | number): Ctx {
    return new Ctx(this.warnings, `${this.path}/${key}`);
  }

  fail(message: string): never {
    throw new DecodeError(`${this.path || "/"}: ${message}`);
  }

  warnUnknown(what: string, value: unknown): void {
    this.warnings.push(`${this.path || "/"}: unknown ${what} ${JSON.stringify(String(value))}`);
  }
}

// ---- 基本の型 (spec/model.md#types) ----

function isObj(v: MsgValue | undefined): v is Obj {
  return typeof v === "object" && v !== null && !Array.isArray(v) && !(v instanceof Uint8Array);
}

/**
 * JSON の文書を読む。小数部も指数部もない数を `bigint` (整数)、ほかを `number` にする
 * (spec/model.md#types: `1.0` は整数のフィールドに書けない)。
 */
export function parseJson(text: string): MsgValue {
  return JSON.parse(text, (_key, value, context?: { source?: string }) => {
    if (typeof value === "number" && context?.source && /^-?\d+$/.test(context.source)) {
      return BigInt(context.source);
    }
    return value;
  }) as MsgValue;
}

export function object(c: Ctx, v: MsgValue | undefined): Obj {
  if (!isObj(v)) c.fail("expected an object");
  return v;
}

export function f32(c: Ctx, v: MsgValue | undefined): number {
  if (typeof v === "number") return Math.fround(v);
  if (typeof v === "bigint") return Math.fround(Number(v));
  return c.fail("expected a number");
}

function uint(bits: number) {
  const limit = 2n ** BigInt(bits);
  return (c: Ctx, v: MsgValue | undefined): number => {
    if (typeof v !== "bigint") return c.fail(`expected an integer (u${bits})`);
    const i = v;
    if (i < 0n || i >= limit) c.fail(`integer out of range of u${bits}`);
    return Number(i);
  };
}

/** `u64` をそのまま `bigint` で読む (2^53 を超えうる目パチの seed。spec/model.md#effects)。 */
export function u64Exact(c: Ctx, v: MsgValue | undefined): bigint {
  if (typeof v !== "bigint") return c.fail("expected an integer (u64)");
  if (v < 0n || v >= 2n ** 64n) c.fail("integer out of range of u64");
  return v;
}

export const u8 = uint(8);
export const u16 = uint(16);
export const u32 = uint(32);
export const u64 = uint(64);

export function str(c: Ctx, v: MsgValue | undefined): string {
  if (typeof v !== "string") c.fail("expected a string");
  return v;
}

export function bool(c: Ctx, v: MsgValue | undefined): boolean {
  if (typeof v !== "boolean") c.fail("expected a boolean");
  return v;
}

export function vec2(c: Ctx, v: MsgValue | undefined): M.Vec2 {
  if (!Array.isArray(v) || v.length !== 2) c.fail("expected an array of two numbers");
  return [f32(c.at(0), v[0]), f32(c.at(1), v[1])];
}

function rgb(c: Ctx, v: MsgValue | undefined): M.Rgb {
  const o = object(c, v);
  return { r: req(c, o, "r", f32), g: req(c, o, "g", f32), b: req(c, o, "b", f32) };
}

export function array<T>(conv: (c: Ctx, v: MsgValue | undefined) => T) {
  return (c: Ctx, v: MsgValue | undefined): T[] => {
    if (!Array.isArray(v)) c.fail("expected an array");
    return v.map((x, i) => conv(c.at(i), x));
  };
}

type Conv<T> = (c: Ctx, v: MsgValue | undefined) => T;

/** 必須のフィールド。 */
export function req<T>(c: Ctx, o: Obj, key: string, conv: Conv<T>): T {
  if (!Object.hasOwn(o, key)) c.fail(`missing required field "${key}"`);
  return conv(c.at(key), o[key]);
}

/** 既定値のあるフィールド。`null` は型に合わないので誤り。 */
export function opt<T>(c: Ctx, o: Obj, key: string, conv: Conv<T>, def: T): T {
  if (!Object.hasOwn(o, key)) return def;
  if (o[key] === null) c.at(key).fail("null is not allowed for a non-nullable field");
  return conv(c.at(key), o[key]);
}

/** null を許すフィールド (`T?`)。無いことと `null` はどちらも「値なし」。 */
export function nul<T>(c: Ctx, o: Obj, key: string, conv: Conv<T>): T | null {
  if (!Object.hasOwn(o, key) || o[key] === null) return null;
  return conv(c.at(key), o[key]);
}

/** 文字列の列挙。知らない値は `fallback` にして警告する。 */
function stringEnum<K extends string, F extends string>(
  what: string,
  known: readonly K[],
  fallback: F,
): Conv<K | F> {
  return (c, v) => {
    const s = str(c, v);
    if ((known as readonly string[]).includes(s)) return s as K;
    c.warnUnknown(what, s);
    return fallback;
  };
}

// ---- モデル (spec/model.md#model) ----

const parameterKind = stringEnum("ParameterKind", ["normal", "blend_shape"], "unknown");
const interpolation = stringEnum("KeyformInterpolation", ["linear", "smooth"], "linear");
const keyformOrigin = stringEnum(
  "KeyformOrigin",
  ["authored", "derived", "additive", "depth_rotation", "mirror"],
  "authored",
);
const blendMode = stringEnum("BlendMode", ["normal", "additive", "multiply"], "normal");
const maskMode = stringEnum("MaskMode", ["union", "intersection"], "union");

function parameter(c: Ctx, v: MsgValue | undefined): M.Parameter {
  const o = object(c, v);
  return {
    id: req(c, o, "id", str),
    name: req(c, o, "name", str),
    min: req(c, o, "min", f32),
    max: req(c, o, "max", f32),
    default: req(c, o, "default", f32),
    kind: opt(c, o, "kind", parameterKind, "normal"),
    mirror: nul(c, o, "mirror", str),
    group: nul(c, o, "group", str),
    internal: opt(c, o, "internal", bool, false),
  };
}

function parameterGroup(c: Ctx, v: MsgValue | undefined): M.ParameterGroup {
  const o = object(c, v);
  return { id: req(c, o, "id", str), name: req(c, o, "name", str) };
}

function binding(c: Ctx, v: MsgValue | undefined): M.KeyformBinding {
  const o = object(c, v);
  return { parameter: req(c, o, "parameter", str), keys: req(c, o, "keys", array(f32)) };
}

function inference(c: Ctx, v: MsgValue | undefined): M.KeyformInference {
  const o = object(c, v);
  const method = req(c, o, "method", str);
  switch (method) {
    case "additive":
      return { method };
    case "depth_rotation":
      return {
        method,
        angle_scale: opt(c, o, "angle_scale", f32, 1),
        pivot: req(c, o, "pivot", vec2),
        pivot_depth: opt(c, o, "pivot_depth", f32, 0),
        yaw: nul(c, o, "yaw", str),
        pitch: nul(c, o, "pitch", str),
      };
    default:
      c.at("method").warnUnknown("KeyformInference", method);
      return { method: "unknown" };
  }
}

function mirrorSource(c: Ctx, v: MsgValue | undefined): M.MirrorSource {
  const o = object(c, v);
  return {
    id: req(c, o, "id", str),
    axis: nul(c, o, "axis", f32),
    tolerance: nul(c, o, "tolerance", f32),
  };
}

function keyformGrid<T>(keyform: Conv<T>): Conv<M.KeyformGrid<T>> {
  const grid: Conv<M.KeyformGrid<T>> = (c, v) => {
    const o = object(c, v);
    return {
      bindings: opt(c, o, "bindings", array(binding), []),
      keyforms: req(c, o, "keyforms", array(keyform)),
      interpolation: opt(c, o, "interpolation", interpolation, "linear"),
      blend_shapes: opt(c, o, "blend_shapes", array(grid), []),
      origins: opt(c, o, "origins", array(keyformOrigin), []),
      inference: nul(c, o, "inference", inference),
      mirror_source: nul(c, o, "mirror_source", mirrorSource),
    };
  };
  return grid;
}

function partKeyform(c: Ctx, v: MsgValue | undefined): M.PartKeyform {
  const o = object(c, v);
  return { opacity: req(c, o, "opacity", f32) };
}

function part(c: Ctx, v: MsgValue | undefined): M.Part {
  const o = object(c, v);
  return {
    id: req(c, o, "id", str),
    name: req(c, o, "name", str),
    parent: nul(c, o, "parent", str),
    keyforms: req(c, o, "keyforms", keyformGrid(partKeyform)),
  };
}

function warpKeyform(c: Ctx, v: MsgValue | undefined): M.WarpKeyform {
  const o = object(c, v);
  return { points: req(c, o, "points", array(vec2)), opacity: req(c, o, "opacity", f32) };
}

function rotationKeyform(c: Ctx, v: MsgValue | undefined): M.RotationKeyform {
  const o = object(c, v);
  return {
    origin: req(c, o, "origin", vec2),
    angle: req(c, o, "angle", f32),
    scale: req(c, o, "scale", f32),
    reflect_x: opt(c, o, "reflect_x", bool, false),
    reflect_y: opt(c, o, "reflect_y", bool, false),
    opacity: req(c, o, "opacity", f32),
  };
}

function deformerKind(c: Ctx, v: MsgValue | undefined): M.DeformerKind {
  const o = object(c, v);
  const type = req(c, o, "type", str);
  switch (type) {
    case "warp":
      return {
        type,
        rows: req(c, o, "rows", u16),
        cols: req(c, o, "cols", u16),
        keyforms: req(c, o, "keyforms", keyformGrid(warpKeyform)),
        depth: nul(c, o, "depth", array(f32)),
      };
    case "rotation":
      return { type, keyforms: req(c, o, "keyforms", keyformGrid(rotationKeyform)) };
    default:
      c.at("type").warnUnknown("DeformerKind", type);
      return { type: "unknown" };
  }
}

function deformer(c: Ctx, v: MsgValue | undefined): M.Deformer {
  const o = object(c, v);
  return {
    id: req(c, o, "id", str),
    name: req(c, o, "name", str),
    part: req(c, o, "part", str),
    parent: nul(c, o, "parent", str),
    kind: req(c, o, "kind", deformerKind),
    mirror: nul(c, o, "mirror", str),
    mirror_axis: nul(c, o, "mirror_axis", f32),
  };
}

function meshKeyform(c: Ctx, v: MsgValue | undefined): M.MeshKeyform {
  const o = object(c, v);
  return {
    positions: req(c, o, "positions", array(vec2)),
    opacity: req(c, o, "opacity", f32),
    draw_order: req(c, o, "draw_order", f32),
    multiply_color: opt(c, o, "multiply_color", rgb, { r: 1, g: 1, b: 1 }),
    screen_color: opt(c, o, "screen_color", rgb, { r: 0, g: 0, b: 0 }),
    mask_strength: opt(c, o, "mask_strength", f32, 1),
  };
}

function maskRef(c: Ctx, v: MsgValue | undefined): M.MaskRef {
  const o = object(c, v);
  return { mesh: req(c, o, "mesh", str), invert: opt(c, o, "invert", bool, false) };
}

function mesh(c: Ctx, v: MsgValue | undefined): M.Mesh {
  const o = object(c, v);
  return {
    id: req(c, o, "id", str),
    name: req(c, o, "name", str),
    part: req(c, o, "part", str),
    parent: nul(c, o, "parent", str),
    texture: req(c, o, "texture", u32),
    uvs: req(c, o, "uvs", array(vec2)),
    indices: req(c, o, "indices", array(u16)),
    blend: opt(c, o, "blend", blendMode, "normal"),
    culling: opt(c, o, "culling", bool, false),
    masks: opt(c, o, "masks", array(maskRef), []),
    mask_mode: opt(c, o, "mask_mode", maskMode, "union"),
    mask_only: opt(c, o, "mask_only", bool, false),
    keyforms: req(c, o, "keyforms", keyformGrid(meshKeyform)),
    mirror: nul(c, o, "mirror", str),
    mirror_axis: nul(c, o, "mirror_axis", f32),
    depth: nul(c, o, "depth", array(f32)),
    lit: opt(c, o, "lit", bool, true),
  };
}

function gluePair(c: Ctx, v: MsgValue | undefined): M.GluePair {
  const o = object(c, v);
  return { a: req(c, o, "a", u16), b: req(c, o, "b", u16), weight: req(c, o, "weight", f32) };
}

function glueKeyform(c: Ctx, v: MsgValue | undefined): M.GlueKeyform {
  const o = object(c, v);
  return { intensity: req(c, o, "intensity", f32) };
}

function glue(c: Ctx, v: MsgValue | undefined): M.Glue {
  const o = object(c, v);
  return {
    id: req(c, o, "id", str),
    mesh_a: req(c, o, "mesh_a", str),
    mesh_b: req(c, o, "mesh_b", str),
    pairs: req(c, o, "pairs", array(gluePair)),
    keyforms: req(c, o, "keyforms", keyformGrid(glueKeyform)),
  };
}

function textureRef(c: Ctx, v: MsgValue | undefined): M.TextureRef {
  const o = object(c, v);
  return {
    name: req(c, o, "name", str),
    width: req(c, o, "width", u32),
    height: req(c, o, "height", u32),
    normal_map: nul(c, o, "normal_map", str),
  };
}

function hitArea(c: Ctx, v: MsgValue | undefined): M.HitArea {
  const o = object(c, v);
  return { id: req(c, o, "id", str), mesh: req(c, o, "mesh", str) };
}

function poseEntry(c: Ctx, v: MsgValue | undefined): M.PoseEntry {
  const o = object(c, v);
  return { part: req(c, o, "part", str), links: opt(c, o, "links", array(str), []) };
}

function poseGroup(c: Ctx, v: MsgValue | undefined): M.PoseGroup {
  const o = object(c, v);
  return { entries: req(c, o, "entries", array(poseEntry)) };
}

function pose(c: Ctx, v: MsgValue | undefined): M.Pose {
  const o = object(c, v);
  return {
    fade_duration: opt(c, o, "fade_duration", f32, 0.5),
    groups: opt(c, o, "groups", array(poseGroup), []),
  };
}

const DEFAULT_EYE_BLINK: M.EyeBlink = {
  parameters: [],
  interval: 4.0,
  closing: Math.fround(0.1),
  closed: Math.fround(0.05),
  opening: Math.fround(0.15),
  seed: 0n,
};

function eyeBlink(c: Ctx, v: MsgValue | undefined): M.EyeBlink {
  const o = object(c, v);
  const d = DEFAULT_EYE_BLINK;
  return {
    parameters: opt(c, o, "parameters", array(str), []),
    interval: opt(c, o, "interval", f32, d.interval),
    closing: opt(c, o, "closing", f32, d.closing),
    closed: opt(c, o, "closed", f32, d.closed),
    opening: opt(c, o, "opening", f32, d.opening),
    seed: opt(c, o, "seed", u64Exact, d.seed),
  };
}

function breathParameter(c: Ctx, v: MsgValue | undefined): M.BreathParameter {
  const o = object(c, v);
  return {
    parameter: req(c, o, "parameter", str),
    offset: req(c, o, "offset", f32),
    peak: req(c, o, "peak", f32),
    cycle: req(c, o, "cycle", f32),
    weight: req(c, o, "weight", f32),
  };
}

function effects(c: Ctx, v: MsgValue | undefined): M.Effects {
  const o = object(c, v);
  const params = (key: string): { parameters: string[] } => {
    const inner = opt(c, o, key, object, {});
    return { parameters: opt(c.at(key), inner, "parameters", array(str), []) };
  };
  const breath = opt(c, o, "breath", object, {});
  return {
    eye_blink: opt(c, o, "eye_blink", eyeBlink, DEFAULT_EYE_BLINK),
    breath: { parameters: opt(c.at("breath"), breath, "parameters", array(breathParameter), []) },
    lip_sync: params("lip_sync"),
  };
}

function drawOrderRange(c: Ctx, v: MsgValue | undefined): M.DrawOrderRange {
  const o = object(c, v);
  return { min: req(c, o, "min", f32), max: req(c, o, "max", f32) };
}

function slot(c: Ctx, v: MsgValue | undefined): M.Slot {
  const o = object(c, v);
  return {
    id: req(c, o, "id", str),
    anchor: nul(c, o, "anchor", str),
    draw_order: req(c, o, "draw_order", drawOrderRange),
    replaces: opt(c, o, "replaces", array(str), []),
    parameters: opt(c, o, "parameters", array(str), []),
    mask_sources: opt(c, o, "mask_sources", array(str), []),
    part: nul(c, o, "part", str),
    colliders: opt(c, o, "colliders", array(str), []),
    pose_parts: opt(c, o, "pose_parts", array(str), []),
  };
}

function avatarLevel(c: Ctx, v: MsgValue | undefined): number | null {
  const o = object(c, v);
  return opt(c, o, "level", u8, 1);
}

/** `schema_version` を読んで、この実装が読める版か確かめる。 */
export const SUPPORTED_SCHEMA_VERSION = 0;

export class UnsupportedSchemaVersion extends Error {
  readonly version: number;
  constructor(version: number) {
    super(`schema_version ${version} is newer than supported (${SUPPORTED_SCHEMA_VERSION})`);
    this.version = version;
  }
}

function schemaVersion(c: Ctx, o: Obj): number {
  const version = req(c, o, "schema_version", u32);
  if (version > SUPPORTED_SCHEMA_VERSION) throw new UnsupportedSchemaVersion(version);
  return version;
}

export function model(c: Ctx, v: MsgValue | undefined): M.Model {
  const o = object(c, v);
  const schema_version = schemaVersion(c, o);
  const metaCtx = c.at("meta");
  const meta = req(c, o, "meta", object);
  const canvas = req(metaCtx, meta, "canvas", object);
  const canvasCtx = metaCtx.at("canvas");
  return {
    schema_version,
    meta: {
      name: req(metaCtx, meta, "name", str),
      canvas: {
        width: req(canvasCtx, canvas, "width", f32),
        height: req(canvasCtx, canvas, "height", f32),
        origin: req(canvasCtx, canvas, "origin", vec2),
      },
    },
    parameters: req(c, o, "parameters", array(parameter)),
    parameter_groups: opt(c, o, "parameter_groups", array(parameterGroup), []),
    parts: req(c, o, "parts", array(part)),
    deformers: opt(c, o, "deformers", array(deformer), []),
    meshes: req(c, o, "meshes", array(mesh)),
    glues: opt(c, o, "glues", array(glue), []),
    textures: req(c, o, "textures", array(textureRef)),
    hit_areas: opt(c, o, "hit_areas", array(hitArea), []),
    pose: opt(c, o, "pose", pose, { fade_duration: 0.5, groups: [] }),
    effects: opt(c, o, "effects", effects, {
      eye_blink: DEFAULT_EYE_BLINK,
      breath: { parameters: [] },
      lip_sync: { parameters: [] },
    }),
    // アバタープロファイルは評価に使わないので、モジュールの条件に要る level だけを読む
    // (spec/model.md#avatar-profile: 使わない読み込み器は無視してよい)
    avatar_level: nul(c, o, "avatar", avatarLevel),
    slots: opt(c, o, "slots", array(slot), []),
    lighting: opt(c, o, "lighting", bool, false),
  };
}

// ---- 物理 (spec/model.md#physics-settings) ----

const physicsSource = stringEnum("PhysicsSource", ["x", "y", "angle"], "unknown");
const physicsBlend = stringEnum("PhysicsBlend", ["overwrite", "add"], "overwrite");

function physicsInput(c: Ctx, v: MsgValue | undefined): M.PhysicsInput {
  const o = object(c, v);
  return {
    parameter: req(c, o, "parameter", str),
    source: req(c, o, "source", physicsSource),
    weight: req(c, o, "weight", f32),
    reflect: opt(c, o, "reflect", bool, false),
  };
}

function vertexOwner(c: Ctx, v: MsgValue | undefined): M.VertexOwner {
  const o = object(c, v);
  const keys = Object.keys(o);
  if (keys.length !== 1) c.fail("VertexOwner must have exactly one field");
  const [key] = keys;
  if (key !== "mesh" && key !== "warp") c.fail(`unknown VertexOwner "${key}" (not extensible)`);
  return { kind: key, id: str(c.at(key), o[key]) };
}

function physicsTarget(c: Ctx, v: MsgValue | undefined): M.PhysicsTarget {
  const o = object(c, v);
  const type = req(c, o, "type", str);
  switch (type) {
    case "parameter":
      return { type, id: req(c, o, "id", str) };
    case "deformer_angle":
    case "deformer_offset":
      return { type, deformer: req(c, o, "deformer", str) };
    case "vertices":
      return {
        type,
        owner: req(c, o, "owner", vertexOwner),
        weights: req(c, o, "weights", array(f32)),
      };
    default:
      c.at("type").warnUnknown("PhysicsTarget", type);
      return { type: "unknown" };
  }
}

function physicsOutput(c: Ctx, v: MsgValue | undefined): M.PhysicsOutput {
  const o = object(c, v);
  let target: M.PhysicsTarget;
  if (Object.hasOwn(o, "target")) {
    target = req(c, o, "target", physicsTarget);
  } else if (Object.hasOwn(o, "parameter")) {
    // 古い形 (spec/model.md#physics-settings の Legacy form)
    target = { type: "parameter", id: req(c, o, "parameter", str) };
  } else {
    c.fail('missing required field "target"');
  }
  return {
    target,
    particle: req(c, o, "particle", u32),
    source: req(c, o, "source", physicsSource),
    scale: req(c, o, "scale", f32),
    weight: req(c, o, "weight", f32),
    reflect: opt(c, o, "reflect", bool, false),
    blend: opt(c, o, "blend", physicsBlend, "overwrite"),
  };
}

function particle(c: Ctx, v: MsgValue | undefined): M.PhysicsParticle {
  const o = object(c, v);
  return {
    position: req(c, o, "position", vec2),
    mobility: req(c, o, "mobility", f32),
    delay: req(c, o, "delay", f32),
    acceleration: req(c, o, "acceleration", f32),
    radius: req(c, o, "radius", f32),
    collision_radius: opt(c, o, "collision_radius", f32, 0),
  };
}

function normalizationRange(c: Ctx, v: MsgValue | undefined): M.NormalizationRange {
  const o = object(c, v);
  return {
    min: req(c, o, "min", f32),
    default: req(c, o, "default", f32),
    max: req(c, o, "max", f32),
  };
}

function rig(c: Ctx, v: MsgValue | undefined): M.PhysicsRig {
  const o = object(c, v);
  const n = req(c, o, "normalization", object);
  const nc = c.at("normalization");
  return {
    id: req(c, o, "id", str),
    name: req(c, o, "name", str),
    inputs: req(c, o, "inputs", array(physicsInput)),
    outputs: req(c, o, "outputs", array(physicsOutput)),
    particles: req(c, o, "particles", array(particle)),
    normalization: {
      position: req(nc, n, "position", normalizationRange),
      angle: req(nc, n, "angle", normalizationRange),
    },
    colliders: opt(c, o, "colliders", array(str), []),
    frame: nul(c, o, "frame", str),
  };
}

function colliderShape(c: Ctx, v: MsgValue | undefined): M.ColliderShape {
  const o = object(c, v);
  const type = req(c, o, "type", str);
  switch (type) {
    case "circle":
      return { type, center: req(c, o, "center", vec2), radius: req(c, o, "radius", f32) };
    case "capsule":
      return {
        type,
        a: req(c, o, "a", vec2),
        b: req(c, o, "b", vec2),
        radius: req(c, o, "radius", f32),
      };
    default:
      c.at("type").warnUnknown("ColliderShape", type);
      return { type: "unknown" };
  }
}

function collider(c: Ctx, v: MsgValue | undefined): M.PhysicsCollider {
  const o = object(c, v);
  return {
    id: req(c, o, "id", str),
    name: req(c, o, "name", str),
    attach: nul(c, o, "attach", str),
    shape: req(c, o, "shape", colliderShape),
  };
}

export function physicsSettings(c: Ctx, v: MsgValue | undefined): M.PhysicsSettings {
  const o = object(c, v);
  return {
    gravity: req(c, o, "gravity", vec2),
    wind: req(c, o, "wind", vec2),
    fps: req(c, o, "fps", f32),
    rigs: req(c, o, "rigs", array(rig)),
    colliders: opt(c, o, "colliders", array(collider), []),
  };
}

// ---- モーションと表情 (spec/model.md#motions, #expressions) ----

function point(c: Ctx, v: MsgValue | undefined): M.Point {
  const o = object(c, v);
  return { t: req(c, o, "t", f32), v: req(c, o, "v", f32) };
}

/** セグメント。知らない種類は `null` (曲線ごと無視する)。 */
function segment(c: Ctx, v: MsgValue | undefined): M.Segment | null {
  const o = object(c, v);
  const type = req(c, o, "type", str);
  switch (type) {
    case "start":
      return { type, p: req(c, o, "p", point) };
    case "linear":
    case "stepped":
    case "inverse_stepped":
      return { type, to: req(c, o, "to", point) };
    case "bezier":
      return {
        type,
        c1: req(c, o, "c1", point),
        c2: req(c, o, "c2", point),
        to: req(c, o, "to", point),
      };
    default:
      c.at("type").warnUnknown("Segment", type);
      return null;
  }
}

/** 曲線。対象かセグメントが知らない値なら `null` (曲線を無視する)。 */
function curve(c: Ctx, v: MsgValue | undefined): M.Curve | null {
  const o = object(c, v);
  const t = req(c, o, "target", object);
  const tc = c.at("target");
  const kind = req(tc, t, "kind", str);
  const id = req(tc, t, "id", MsgAny);
  const segments = req(c, o, "segments", array(segment));
  const fade_in = nul(c, o, "fade_in", f32);
  const fade_out = nul(c, o, "fade_out", f32);
  if (kind !== "parameter" && kind !== "part_opacity" && kind !== "effect_group") {
    tc.at("kind").warnUnknown("CurveTarget", kind);
    return null;
  }
  if (segments.some((s) => s === null)) return null;
  return {
    target: { kind, id: str(tc.at("id"), id) },
    segments: segments as M.Segment[],
    fade_in,
    fade_out,
  };
}

/** 知らない種類の対象の `id` はどんな値でもよい (spec/model.md#motions)。 */
function MsgAny(_c: Ctx, v: MsgValue | undefined): MsgValue | undefined {
  return v;
}

function motionEvent(c: Ctx, v: MsgValue | undefined): M.MotionEvent {
  const o = object(c, v);
  return { t: req(c, o, "t", f32), value: req(c, o, "value", str) };
}

export function motion(c: Ctx, v: MsgValue | undefined): M.Motion {
  const o = object(c, v);
  return {
    name: req(c, o, "name", str),
    duration: req(c, o, "duration", f32),
    looping: opt(c, o, "looping", bool, false),
    fade_in: opt(c, o, "fade_in", f32, 0),
    fade_out: opt(c, o, "fade_out", f32, 0),
    curves: req(c, o, "curves", array(curve)).filter((x): x is M.Curve => x !== null),
    events: opt(c, o, "events", array(motionEvent), []),
  };
}

const expressionOp = stringEnum("ExpressionOp", ["add", "multiply", "overwrite"], "unknown");

export function expression(c: Ctx, v: MsgValue | undefined): M.Expression {
  const o = object(c, v);
  const items = req(
    c,
    o,
    "parameters",
    array((ic, iv) => {
      const io = object(ic, iv);
      return {
        parameter: req(ic, io, "parameter", str),
        value: req(ic, io, "value", f32),
        op: req(ic, io, "op", expressionOp),
      };
    }),
  );
  return {
    name: req(c, o, "name", str),
    fade_in: opt(c, o, "fade_in", f32, 0),
    fade_out: opt(c, o, "fade_out", f32, 0),
    parameters: items.filter((x): x is M.ExpressionParameter => x.op !== "unknown"),
  };
}

// ---- モジュール (spec/model.md#modules) ----

export function module(c: Ctx, v: MsgValue | undefined): M.Module {
  const o = object(c, v);
  const schema_version = schemaVersion(c, o);
  const requires = opt(c, o, "requires", object, {});
  const rc = c.at("requires");
  const physics = opt(c, o, "physics", object, {});
  const pc = c.at("physics");
  const modulePose = opt(c, o, "pose", object, {});
  const mc = c.at("pose");
  return {
    schema_version,
    name: req(c, o, "name", str),
    slot: req(c, o, "slot", str),
    requires: {
      avatar_level: nul(rc, requires, "avatar_level", u8),
      slots: opt(rc, requires, "slots", array(str), []),
      parameters: opt(rc, requires, "parameters", array(str), []),
    },
    parts: opt(c, o, "parts", array(part), []),
    deformers: opt(c, o, "deformers", array(deformer), []),
    meshes: opt(c, o, "meshes", array(mesh), []),
    glues: opt(c, o, "glues", array(glue), []),
    textures: opt(c, o, "textures", array(textureRef), []),
    physics: {
      rigs: opt(pc, physics, "rigs", array(rig), []),
      colliders: opt(pc, physics, "colliders", array(collider), []),
    },
    pose: {
      groups: opt(mc, modulePose, "groups", array(poseGroup), []),
      links: opt(mc, modulePose, "links", array(poseEntry), []),
    },
  };
}
