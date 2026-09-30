// `.ugoe` と `.ugoemod` の書き出し (spec/bundle.md#writing)。
//
// 型の決まったデータ (types.ts) から、MessagePack のペイロードとコンテナを作る。
// - 整数の型のフィールドは MessagePack の整数、`f32` は float 32 で書く (spec/model.md#types)。
// - 既定値のフィールドと値の無い nullable のフィールドは、書き方 (`Style`) に従って省くか書く
//   (spec/model.md#presence-and-default-values。読み込み器はどちらも受け付けなければならない)。
//   作るときのデータ (spec/model.md#authoring-data) は、既定値なら常に省く (SHOULD)。
// - 知らない値は書けない (spec/model.md#unknown-values)。型の上で「知らない値」として残るものがあれば
//   `WriteError` にする。フォールバックに置き換えて読んだデータ (読み込みの警告があるもの) を書かないのは
//   呼び出し側の責任 (`fromJson` は警告があれば拒否する)。
// - コンテナは表の順 FEAT, MODL/MODU, PHYS, MOTN…, EXPR…, META, TEXR…, NRML… で書く (SHOULD)。

import { isDeepStrictEqual } from "node:util";
import { zstdCompressSync } from "node:zlib";
import { KNOWN_FEATURES } from "./bundle.ts";
import { DEFAULT_AVATAR_META } from "./decode.ts";
import { encode, type MsgValue } from "./msgpack.ts";
import type * as M from "./types.ts";

export class WriteError extends Error {}

/** 既定値と値の無いフィールドの書き方。 */
export interface Style {
  /** `omit`: 既定値のフィールドを省く。`write`: 書く。 */
  defaults: "omit" | "write";
  /** `omit`: 値の無い nullable のフィールドを省く。`null`: `null` を書く。 */
  nulls: "omit" | "null";
}

export const MINIMAL: Style = { defaults: "omit", nulls: "omit" };
export const EXPLICIT: Style = { defaults: "write", nulls: "null" };

type Obj = { [key: string]: MsgValue };

/** オブジェクトを、フィールドの表の順に組み立てる。 */
class Fields {
  readonly o: Obj = {};
  readonly style: Style;
  constructor(style: Style) {
    this.style = style;
  }

  req(key: string, value: MsgValue): this {
    this.o[key] = value;
    return this;
  }

  /** 既定値 `def` のあるフィールド。 */
  opt<T>(key: string, value: T, def: T, enc: (v: T) => MsgValue): this {
    if (this.style.defaults === "write" || !isDeepStrictEqual(value, def)) this.o[key] = enc(value);
    return this;
  }

  /** nullable のフィールド。 */
  nul<T>(key: string, value: T | null, enc: (v: T) => MsgValue): this {
    if (value !== null) this.o[key] = enc(value);
    else if (this.style.nulls === "null") this.o[key] = null;
    return this;
  }

  /** 作るときのデータ。既定値 (空・値なし) なら常に省く。 */
  auth<T>(key: string, value: T | null, isDefault: boolean, enc: (v: T) => MsgValue): this {
    if (!isDefault && value !== null) this.o[key] = enc(value);
    return this;
  }
}

// ---- 基本の型 ----

const f32 = (v: number): MsgValue => Math.fround(v);
const int = (v: number | bigint): MsgValue => BigInt(v);
const str = (v: string): MsgValue => v;
const bool = (v: boolean): MsgValue => v;
const vec2 = (v: M.Vec2): MsgValue => [f32(v[0]), f32(v[1])];
const rgb = (v: M.Rgb): MsgValue => ({ r: f32(v.r), g: f32(v.g), b: f32(v.b) });
const arr =
  <T>(enc: (v: T) => MsgValue) =>
  (v: T[]): MsgValue =>
    v.map(enc);

function unknown(where: string): never {
  throw new WriteError(
    `${where}: an unknown value cannot be written (spec/model.md#unknown-values)`,
  );
}

// ---- モデル (spec/model.md#model) ----

class Encoder {
  readonly s: Style;
  constructor(s: Style) {
    this.s = s;
  }

  f(): Fields {
    return new Fields(this.s);
  }

  keyformGrid<T>(g: M.KeyformGrid<T>, keyform: (k: T) => MsgValue): MsgValue {
    return this.f()
      .opt(
        "bindings",
        g.bindings,
        [],
        arr((b) => ({ parameter: b.parameter, keys: b.keys.map(f32) })),
      )
      .req("keyforms", g.keyforms.map(keyform))
      .opt("interpolation", g.interpolation, "linear", str)
      .opt(
        "blend_shapes",
        g.blend_shapes,
        [],
        arr((b) => this.keyformGrid(b, keyform)),
      )
      .auth("origins", g.origins, g.origins.length === 0, arr(str))
      .auth("inference", g.inference, g.inference === null, (i) => this.inference(i))
      .auth(
        "mirror_source",
        g.mirror_source,
        g.mirror_source === null,
        (m) =>
          this.f().req("id", m.id).nul("axis", m.axis, f32).nul("tolerance", m.tolerance, f32).o,
      ).o;
  }

  inference(i: M.KeyformInference | null): MsgValue {
    if (i === null) return null;
    switch (i.method) {
      case "additive":
        return { method: "additive" };
      case "depth_rotation":
        return this.f()
          .req("method", i.method)
          .opt("angle_scale", i.angle_scale, 1, f32)
          .req("pivot", vec2(i.pivot))
          .opt("pivot_depth", i.pivot_depth, 0, f32)
          .nul("yaw", i.yaw, str)
          .nul("pitch", i.pitch, str).o;
      default:
        return unknown("KeyformGrid.inference");
    }
  }

  parameter(p: M.Parameter): MsgValue {
    if (p.kind === "unknown") unknown(`parameter ${p.id}: kind`);
    return this.f()
      .req("id", p.id)
      .req("name", p.name)
      .req("min", f32(p.min))
      .req("max", f32(p.max))
      .req("default", f32(p.default))
      .opt("kind", p.kind, "normal", str)
      .nul("mirror", p.mirror, str)
      .nul("group", p.group, str)
      .opt("internal", p.internal, false, bool).o;
  }

  part(p: M.Part): MsgValue {
    return this.f()
      .req("id", p.id)
      .req("name", p.name)
      .nul("parent", p.parent, str)
      .req(
        "keyforms",
        this.keyformGrid(p.keyforms, (k) => ({ opacity: f32(k.opacity) })),
      ).o;
  }

  deformer(d: M.Deformer): MsgValue {
    return this.f()
      .req("id", d.id)
      .req("name", d.name)
      .req("part", d.part)
      .nul("parent", d.parent, str)
      .req("kind", this.deformerKind(d))
      .auth("mirror", d.mirror, d.mirror === null, str)
      .auth("mirror_axis", d.mirror_axis, d.mirror_axis === null, f32).o;
  }

  deformerKind(d: M.Deformer): MsgValue {
    const k = d.kind;
    switch (k.type) {
      case "warp":
        return this.f()
          .req("type", "warp")
          .req("rows", int(k.rows))
          .req("cols", int(k.cols))
          .req(
            "keyforms",
            this.keyformGrid(k.keyforms, (w) => ({
              points: w.points.map(vec2),
              opacity: f32(w.opacity),
            })),
          )
          .auth("depth", k.depth, k.depth === null, arr(f32)).o;
      case "rotation":
        return {
          type: "rotation",
          keyforms: this.keyformGrid(
            k.keyforms,
            (r) =>
              this.f()
                .req("origin", vec2(r.origin))
                .req("angle", f32(r.angle))
                .req("scale", f32(r.scale))
                .opt("reflect_x", r.reflect_x, false, bool)
                .opt("reflect_y", r.reflect_y, false, bool)
                .req("opacity", f32(r.opacity)).o,
          ),
        };
      default:
        return unknown(`deformer ${d.id}: kind`);
    }
  }

  mesh(m: M.Mesh): MsgValue {
    return this.f()
      .req("id", m.id)
      .req("name", m.name)
      .req("part", m.part)
      .nul("parent", m.parent, str)
      .req("texture", int(m.texture))
      .req("uvs", m.uvs.map(vec2))
      .req("indices", m.indices.map(int))
      .opt("blend", m.blend, "normal", str)
      .opt("culling", m.culling, false, bool)
      .opt(
        "masks",
        m.masks,
        [],
        arr((r) => this.f().req("mesh", r.mesh).opt("invert", r.invert, false, bool).o),
      )
      .opt("mask_mode", m.mask_mode, "union", str)
      .opt("mask_only", m.mask_only, false, bool)
      .req(
        "keyforms",
        this.keyformGrid(
          m.keyforms,
          (k) =>
            this.f()
              .req("positions", k.positions.map(vec2))
              .req("opacity", f32(k.opacity))
              .req("draw_order", f32(k.draw_order))
              .opt("multiply_color", k.multiply_color, { r: 1, g: 1, b: 1 }, rgb)
              .opt("screen_color", k.screen_color, { r: 0, g: 0, b: 0 }, rgb)
              .opt("mask_strength", k.mask_strength, 1, f32).o,
        ),
      )
      .auth("mirror", m.mirror, m.mirror === null, str)
      .auth("mirror_axis", m.mirror_axis, m.mirror_axis === null, f32)
      .auth("depth", m.depth, m.depth === null, arr(f32))
      .opt("lit", m.lit, true, bool).o;
  }

  glue(g: M.Glue): MsgValue {
    return {
      id: g.id,
      mesh_a: g.mesh_a,
      mesh_b: g.mesh_b,
      pairs: g.pairs.map((p) => ({ a: int(p.a), b: int(p.b), weight: f32(p.weight) })),
      keyforms: this.keyformGrid(g.keyforms, (k) => ({ intensity: f32(k.intensity) })),
    };
  }

  texture(t: M.TextureRef): MsgValue {
    return this.f()
      .req("name", t.name)
      .req("width", int(t.width))
      .req("height", int(t.height))
      .nul("normal_map", t.normal_map, str).o;
  }

  poseEntry(e: M.PoseEntry): MsgValue {
    return this.f().req("part", e.part).opt("links", e.links, [], arr(str)).o;
  }

  poseGroup(g: M.PoseGroup): MsgValue {
    return { entries: g.entries.map((e) => this.poseEntry(e)) };
  }

  effects(e: M.Effects): MsgValue {
    const b = e.eye_blink;
    const eyeBlink = this.f()
      .opt("parameters", b.parameters, [], arr(str))
      .opt("interval", b.interval, 4, f32)
      .opt("closing", b.closing, Math.fround(0.1), f32)
      .opt("closed", b.closed, Math.fround(0.05), f32)
      .opt("opening", b.opening, Math.fround(0.15), f32)
      .opt("seed", b.seed, 0n, (v) => v).o;
    const breath = this.f().opt(
      "parameters",
      e.breath.parameters,
      [],
      arr((p) => ({
        parameter: p.parameter,
        offset: f32(p.offset),
        peak: f32(p.peak),
        cycle: f32(p.cycle),
        weight: f32(p.weight),
      })),
    ).o;
    const lipSync = this.f().opt("parameters", e.lip_sync.parameters, [], arr(str)).o;
    return this.f()
      .opt("eye_blink", eyeBlink, {}, (v) => v)
      .opt("breath", breath, {}, (v) => v)
      .opt("lip_sync", lipSync, {}, (v) => v).o;
  }

  avatar(a: M.AvatarProfile): MsgValue {
    const m = a.meta;
    const d = DEFAULT_AVATAR_META;
    const meta = this.f()
      .opt("name", m.name, d.name, str)
      .nul("version", m.version, str)
      .opt("authors", m.authors, [], arr(str))
      .nul("copyright", m.copyright, str)
      .nul("contact", m.contact, str)
      .opt("references", m.references, [], arr(str))
      .nul("thumbnail", m.thumbnail, int)
      .nul("license_url", m.license_url, str)
      .opt("avatar_permission", m.avatar_permission, d.avatar_permission, str)
      .opt("commercial_usage", m.commercial_usage, d.commercial_usage, str)
      .opt("allow_violent_usage", m.allow_violent_usage, false, bool)
      .opt("allow_sexual_usage", m.allow_sexual_usage, false, bool)
      .opt("allow_political_or_religious_usage", m.allow_political_or_religious_usage, false, bool)
      .opt("allow_antisocial_or_hate_usage", m.allow_antisocial_or_hate_usage, false, bool)
      .opt("credit_notation", m.credit_notation, d.credit_notation, str)
      .opt("allow_redistribution", m.allow_redistribution, false, bool)
      .opt("modification", m.modification, d.modification, str)
      .nul("third_party_licenses", m.third_party_licenses, str).o;
    return this.f()
      .opt("profile_version", a.profile_version, 0, int)
      .opt("level", a.level, 1, int)
      .opt("meta", meta, {}, (v) => v)
      .opt(
        "tracking",
        a.tracking,
        [],
        arr(
          (t) =>
            this.f()
              .req("source", t.source)
              .req("input", t.input)
              .nul("parameter", t.parameter, str)
              .opt("scale", t.scale, 1, f32)
              .opt("offset", t.offset, 0, f32).o,
        ),
      )
      .nul(
        "look_at",
        a.look_at,
        (l) =>
          this.f()
            .opt("x", l.x, "EyeBallX", str)
            .opt("y", l.y, "EyeBallY", str)
            .opt("range_x", l.range_x, 30, f32)
            .opt("range_y", l.range_y, 30, f32).o,
      )
      .opt(
        "hit_areas",
        a.hit_areas,
        [],
        arr((h) => ({ name: h.name, hit_area: h.hit_area })),
      )
      .opt("slots", a.slots, [], arr(str)).o;
  }

  slot(s: M.Slot): MsgValue {
    return this.f()
      .req("id", s.id)
      .nul("anchor", s.anchor, str)
      .req("draw_order", { min: f32(s.draw_order.min), max: f32(s.draw_order.max) })
      .opt("replaces", s.replaces, [], arr(str))
      .opt("parameters", s.parameters, [], arr(str))
      .opt("mask_sources", s.mask_sources, [], arr(str))
      .nul("part", s.part, str)
      .opt("colliders", s.colliders, [], arr(str))
      .opt("pose_parts", s.pose_parts, [], arr(str)).o;
  }

  model(m: M.Model): MsgValue {
    return this.f()
      .req("schema_version", int(m.schema_version))
      .req("meta", {
        name: m.meta.name,
        canvas: {
          width: f32(m.meta.canvas.width),
          height: f32(m.meta.canvas.height),
          origin: vec2(m.meta.canvas.origin),
        },
      })
      .req(
        "parameters",
        m.parameters.map((p) => this.parameter(p)),
      )
      .opt(
        "parameter_groups",
        m.parameter_groups,
        [],
        arr((g) => ({ id: g.id, name: g.name })),
      )
      .req(
        "parts",
        m.parts.map((p) => this.part(p)),
      )
      .opt(
        "deformers",
        m.deformers,
        [],
        arr((d) => this.deformer(d)),
      )
      .req(
        "meshes",
        m.meshes.map((x) => this.mesh(x)),
      )
      .opt(
        "glues",
        m.glues,
        [],
        arr((g) => this.glue(g)),
      )
      .req(
        "textures",
        m.textures.map((t) => this.texture(t)),
      )
      .opt(
        "hit_areas",
        m.hit_areas,
        [],
        arr((h) => ({ id: h.id, mesh: h.mesh })),
      )
      .opt(
        "pose",
        m.pose,
        { fade_duration: 0.5, groups: [] },
        (p) =>
          this.f()
            .opt("fade_duration", p.fade_duration, 0.5, f32)
            .opt(
              "groups",
              p.groups,
              [],
              arr((g) => this.poseGroup(g)),
            ).o,
      )
      .opt("effects", this.effects(m.effects), {}, (v) => v)
      .nul("avatar", m.avatar, (a) => this.avatar(a))
      .opt(
        "slots",
        m.slots,
        [],
        arr((s) => this.slot(s)),
      )
      .opt("lighting", m.lighting, false, bool).o;
  }

  // ---- 物理 (spec/model.md#physics-settings) ----

  source(s: M.PhysicsSource, where: string): MsgValue {
    return s === "unknown" ? unknown(where) : s;
  }

  target(t: M.PhysicsTarget, where: string): MsgValue {
    switch (t.type) {
      case "parameter":
        return { type: t.type, id: t.id };
      case "deformer_angle":
      case "deformer_offset":
        return { type: t.type, deformer: t.deformer };
      case "vertices":
        return {
          type: t.type,
          owner: { [t.owner.kind]: t.owner.id },
          weights: t.weights.map(f32),
        };
      default:
        return unknown(where);
    }
  }

  rig(r: M.PhysicsRig): MsgValue {
    const range = (n: M.NormalizationRange) => ({
      min: f32(n.min),
      default: f32(n.default),
      max: f32(n.max),
    });
    return this.f()
      .req("id", r.id)
      .req("name", r.name)
      .req(
        "inputs",
        r.inputs.map(
          (i) =>
            this.f()
              .req("parameter", i.parameter)
              .req("source", this.source(i.source, `rig ${r.id}: input source`))
              .req("weight", f32(i.weight))
              .opt("reflect", i.reflect, false, bool).o,
        ),
      )
      .req(
        "outputs",
        r.outputs.map(
          (o) =>
            this.f()
              .req("target", this.target(o.target, `rig ${r.id}: output target`))
              .req("particle", int(o.particle))
              .req("source", this.source(o.source, `rig ${r.id}: output source`))
              .req("scale", f32(o.scale))
              .req("weight", f32(o.weight))
              .opt("reflect", o.reflect, false, bool)
              .opt("blend", o.blend, "overwrite", str).o,
        ),
      )
      .req(
        "particles",
        r.particles.map(
          (p) =>
            this.f()
              .req("position", vec2(p.position))
              .req("mobility", f32(p.mobility))
              .req("delay", f32(p.delay))
              .req("acceleration", f32(p.acceleration))
              .req("radius", f32(p.radius))
              .opt("collision_radius", p.collision_radius, 0, f32).o,
        ),
      )
      .req("normalization", {
        position: range(r.normalization.position),
        angle: range(r.normalization.angle),
      })
      .opt("colliders", r.colliders, [], arr(str))
      .nul("frame", r.frame, str).o;
  }

  collider(c: M.PhysicsCollider): MsgValue {
    const s = c.shape;
    let shape: MsgValue;
    switch (s.type) {
      case "circle":
        shape = { type: s.type, center: vec2(s.center), radius: f32(s.radius) };
        break;
      case "capsule":
        shape = { type: s.type, a: vec2(s.a), b: vec2(s.b), radius: f32(s.radius) };
        break;
      default:
        return unknown(`collider ${c.id}: shape`);
    }
    return this.f()
      .req("id", c.id)
      .req("name", c.name)
      .nul("attach", c.attach, str)
      .req("shape", shape).o;
  }

  physics(p: M.PhysicsSettings): MsgValue {
    return this.f()
      .req("gravity", vec2(p.gravity))
      .req("wind", vec2(p.wind))
      .req("fps", f32(p.fps))
      .req(
        "rigs",
        p.rigs.map((r) => this.rig(r)),
      )
      .opt(
        "colliders",
        p.colliders,
        [],
        arr((c) => this.collider(c)),
      ).o;
  }

  // ---- モーションと表情 (spec/model.md#motions, #expressions) ----

  motion(m: M.Motion): MsgValue {
    const point = (p: M.Point) => ({ t: f32(p.t), v: f32(p.v) });
    const segment = (s: M.Segment): MsgValue => {
      switch (s.type) {
        case "start":
          return { type: s.type, p: point(s.p) };
        case "bezier":
          return { type: s.type, c1: point(s.c1), c2: point(s.c2), to: point(s.to) };
        default:
          return { type: s.type, to: point(s.to) };
      }
    };
    return this.f()
      .req("name", m.name)
      .req("duration", f32(m.duration))
      .opt("looping", m.looping, false, bool)
      .opt("fade_in", m.fade_in, 0, f32)
      .opt("fade_out", m.fade_out, 0, f32)
      .req(
        "curves",
        m.curves.map(
          (c) =>
            this.f()
              .req("target", { kind: c.target.kind, id: c.target.id })
              .req("segments", c.segments.map(segment))
              .nul("fade_in", c.fade_in, f32)
              .nul("fade_out", c.fade_out, f32).o,
        ),
      )
      .opt(
        "events",
        m.events,
        [],
        arr((e) => ({ t: f32(e.t), value: e.value })),
      ).o;
  }

  expression(e: M.Expression): MsgValue {
    return this.f()
      .req("name", e.name)
      .opt("fade_in", e.fade_in, 0, f32)
      .opt("fade_out", e.fade_out, 0, f32)
      .req(
        "parameters",
        e.parameters.map((p) => ({ parameter: p.parameter, value: f32(p.value), op: p.op })),
      ).o;
  }

  // ---- モジュール (spec/model.md#modules) ----

  module(m: M.Module): MsgValue {
    const r = m.requires;
    return this.f()
      .req("schema_version", int(m.schema_version))
      .req("name", m.name)
      .req("slot", m.slot)
      .opt(
        "requires",
        this.f()
          .nul("avatar_level", r.avatar_level, int)
          .opt("slots", r.slots, [], arr(str))
          .opt("parameters", r.parameters, [], arr(str)).o,
        {},
        (v) => v,
      )
      .opt(
        "parts",
        m.parts,
        [],
        arr((p) => this.part(p)),
      )
      .opt(
        "deformers",
        m.deformers,
        [],
        arr((d) => this.deformer(d)),
      )
      .opt(
        "meshes",
        m.meshes,
        [],
        arr((x) => this.mesh(x)),
      )
      .opt(
        "glues",
        m.glues,
        [],
        arr((g) => this.glue(g)),
      )
      .opt(
        "textures",
        m.textures,
        [],
        arr((t) => this.texture(t)),
      )
      .opt(
        "physics",
        this.f()
          .opt(
            "rigs",
            m.physics.rigs,
            [],
            arr((x) => this.rig(x)),
          )
          .opt(
            "colliders",
            m.physics.colliders,
            [],
            arr((c) => this.collider(c)),
          ).o,
        {},
        (v) => v,
      )
      .opt(
        "pose",
        this.f()
          .opt(
            "groups",
            m.pose.groups,
            [],
            arr((g) => this.poseGroup(g)),
          )
          .opt(
            "links",
            m.pose.links,
            [],
            arr((e) => this.poseEntry(e)),
          ).o,
        {},
        (v) => v,
      ).o;
  }
}

// ---- コンテナ (spec/bundle.md#container-layout) ----

/** コンテナの 1 つのチャンク。 */
export interface ChunkSpec {
  tag: string;
  /** 格納する前の (展開した) ペイロード。 */
  data: Uint8Array;
  /** zstd で圧縮する (flags のビット 0)。 */
  compress?: boolean;
  /** そのまま書く flags (試験用。`compress` より優先する)。 */
  flags?: number;
}

export interface ContainerOptions {
  /** ペイロードを置く順 (表の添字の並び)。省けば表の順。 */
  payloadOrder?: number[];
}

const align16 = (n: number) => Math.ceil(n / 16) * 16;

/** ヘッダ・チャンクの表・16 バイト境界に揃えたペイロードを書く。詰め物は 0。 */
export function writeContainer(chunks: ChunkSpec[], options: ContainerOptions = {}): Uint8Array {
  for (const c of chunks) {
    if (c.tag.length !== 4)
      throw new WriteError(`chunk tag ${JSON.stringify(c.tag)} is not 4 bytes`);
  }
  const stored = chunks.map((c) =>
    c.compress && c.flags === undefined ? new Uint8Array(zstdCompressSync(c.data)) : c.data,
  );
  const order = options.payloadOrder ?? chunks.map((_, i) => i);
  if ([...order].sort((a, b) => a - b).join() !== chunks.map((_, i) => i).join()) {
    throw new WriteError("payloadOrder is not a permutation of the chunks");
  }
  const offsets = new Array<number>(chunks.length);
  let at = align16(16 + 24 * chunks.length);
  for (const i of order) {
    offsets[i] = at;
    at = align16(at + stored[i].length);
  }
  const out = new Uint8Array(at);
  const view = new DataView(out.buffer);
  out.set([0x55, 0x47, 0x4f, 0x45]);
  view.setUint32(4, 0, true); // コンテナの版
  view.setUint32(8, 0, true); // 予約
  view.setUint32(12, chunks.length, true);
  chunks.forEach((c, i) => {
    const e = 16 + 24 * i;
    for (let k = 0; k < 4; k++) out[e + k] = c.tag.charCodeAt(k);
    view.setUint32(e + 4, c.flags ?? (c.compress ? 1 : 0), true);
    view.setBigUint64(e + 8, BigInt(offsets[i]), true);
    view.setBigUint64(e + 16, BigInt(stored[i].length), true);
    out.set(stored[i], offsets[i]);
  });
  return out;
}

// ---- バンドルとモジュールのファイル (spec/bundle.md#bundle-files, #module-files) ----

export interface FeatureDecl {
  id: string;
  required: boolean;
}

export interface WriteOptions {
  style?: Style;
  /** 圧縮するチャンクの種類 (例 `["MODL", "PHYS"]`)。画像は圧縮しない方がよい (SHOULD NOT)。 */
  compress?: string[];
  /**
   * 宣言する機能。基準の機能は宣言できない (MUST NOT)。`test.` で始まる ID は試験のためのもの
   * (spec/bundle.md#feature-declarations)。空なら FEAT を書かない (SHOULD)。
   */
  features?: FeatureDecl[];
  /** 表の後ろに足すチャンク (知らないチャンクの試験用)。 */
  extraChunks?: ChunkSpec[];
  payloadOrder?: ContainerOptions["payloadOrder"];
}

export interface BundleInput {
  model: M.Model;
  physics?: M.PhysicsSettings | null;
  motions?: M.Motion[];
  expressions?: M.Expression[];
  /** META の JSON の値。 */
  meta?: unknown;
  textures: Uint8Array[];
  normalMaps?: Uint8Array[];
}

export interface ModuleInput {
  module: M.Module;
  meta?: unknown;
  textures: Uint8Array[];
  normalMaps?: Uint8Array[];
}

function featureChunk(features: FeatureDecl[]): ChunkSpec[] {
  if (features.length === 0) return [];
  for (const f of features) {
    if (KNOWN_FEATURES.has(f.id))
      throw new WriteError(`baseline feature ${f.id} must not be declared`);
  }
  const value: MsgValue = {
    features: features.map((f) => ({ id: f.id, required: f.required })),
  };
  return [{ tag: "FEAT", data: encode(value) }];
}

function imageChunks(
  textures: M.TextureRef[],
  images: Uint8Array[],
  normalMaps: Uint8Array[],
): ChunkSpec[] {
  if (images.length !== textures.length) {
    throw new WriteError(`${images.length} images for ${textures.length} textures`);
  }
  const withNormal = textures.filter((t) => t.normal_map !== null).length;
  if (normalMaps.length !== withNormal) {
    throw new WriteError(`${normalMaps.length} normal maps for ${withNormal} textures with one`);
  }
  return [
    ...images.map((data) => ({ tag: "TEXR", data })),
    ...normalMaps.map((data) => ({ tag: "NRML", data })),
  ];
}

function metaChunk(meta: unknown): ChunkSpec[] {
  if (meta === undefined) return [];
  return [{ tag: "META", data: new TextEncoder().encode(JSON.stringify(meta)) }];
}

function finish(chunks: ChunkSpec[], options: WriteOptions): Uint8Array {
  const compress = new Set(options.compress ?? []);
  const all = [...chunks, ...(options.extraChunks ?? [])].map((c) => ({
    ...c,
    compress: c.compress ?? compress.has(c.tag),
  }));
  return writeContainer(all, { payloadOrder: options.payloadOrder });
}

/** `.ugoe` を書く。 */
export function writeBundle(input: BundleInput, options: WriteOptions = {}): Uint8Array {
  const e = new Encoder(options.style ?? MINIMAL);
  const chunks: ChunkSpec[] = [
    ...featureChunk(options.features ?? []),
    { tag: "MODL", data: encode(e.model(input.model)) },
  ];
  if (input.physics) chunks.push({ tag: "PHYS", data: encode(e.physics(input.physics)) });
  for (const m of input.motions ?? []) chunks.push({ tag: "MOTN", data: encode(e.motion(m)) });
  for (const x of input.expressions ?? []) {
    chunks.push({ tag: "EXPR", data: encode(e.expression(x)) });
  }
  chunks.push(...metaChunk(input.meta));
  chunks.push(...imageChunks(input.model.textures, input.textures, input.normalMaps ?? []));
  return finish(chunks, options);
}

/** `.ugoemod` を書く。 */
export function writeModuleFile(input: ModuleInput, options: WriteOptions = {}): Uint8Array {
  const e = new Encoder(options.style ?? MINIMAL);
  const chunks: ChunkSpec[] = [
    ...featureChunk(options.features ?? []),
    { tag: "MODU", data: encode(e.module(input.module)) },
    ...metaChunk(input.meta),
    ...imageChunks(input.module.textures, input.textures, input.normalMaps ?? []),
  ];
  return finish(chunks, options);
}

/** 型の決まったデータを MessagePack の値にする (試験で使う)。 */
export function encodeValue(
  kind: "model" | "physics" | "motion" | "expression" | "module",
  value: unknown,
  style: Style = MINIMAL,
): MsgValue {
  const e = new Encoder(style);
  switch (kind) {
    case "model":
      return e.model(value as M.Model);
    case "physics":
      return e.physics(value as M.PhysicsSettings);
    case "motion":
      return e.motion(value as M.Motion);
    case "expression":
      return e.expression(value as M.Expression);
    case "module":
      return e.module(value as M.Module);
  }
}
