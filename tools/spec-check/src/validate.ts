// モデル・物理・モーション・表情・モジュールの検証 (spec/model.md#validation)。
// 規則の番号は spec/model.md#model-validation、spec/model.md#physics-validation、spec/model.md#motion-and-expression-validation の番号。
// 違反をすべて集めて返す。

import type * as M from "./types.ts";

const WELL_KNOWN_PARAMETERS = new Set([
  "AngleX",
  "AngleY",
  "AngleZ",
  "BodyAngleX",
  "BodyAngleY",
  "BodyAngleZ",
  "Breath",
  "EyeLOpen",
  "EyeROpen",
  "EyeLSmile",
  "EyeRSmile",
  "EyeBallX",
  "EyeBallY",
  "BrowLY",
  "BrowRY",
  "MouthForm",
  "MouthOpenY",
  "Cheek",
  "HairFront",
  "HairSide",
  "HairBack",
  "EyeLWide",
  "EyeRWide",
  "BrowInnerUp",
  "BrowLAngle",
  "BrowRAngle",
  "MouthX",
  "JawX",
  "MouthPucker",
  "MouthFunnel",
  "MouthPress",
  "MouthShrug",
  "CheekPuff",
  "TongueOut",
  "NoseSneer",
  "ShoulderLY",
  "ShoulderRY",
  "MouthVowelA",
  "MouthVowelI",
  "MouthVowelU",
  "MouthVowelE",
  "MouthVowelO",
]);

/** well-known な パラメータ ID か (spec/model.md#well-known-parameter-ids)。 */
export const isWellKnownParameter = (id: string): boolean => WELL_KNOWN_PARAMETERS.has(id);

const finite = Number.isFinite;

function duplicates(ids: string[]): string[] {
  const seen = new Set<string>();
  const dup = new Set<string>();
  for (const id of ids) (seen.has(id) ? dup : seen).add(id);
  return [...dup];
}

/** 親の連鎖に循環があるものを返す。 */
function cyclic(items: { id: string; parent: string | null }[]): string[] {
  const parent = new Map(items.map((x) => [x.id, x.parent]));
  const out: string[] = [];
  for (const x of items) {
    const seen = new Set<string>();
    let cur: string | null = x.id;
    while (cur !== null && parent.has(cur)) {
      if (seen.has(cur)) {
        out.push(x.id);
        break;
      }
      seen.add(cur);
      cur = parent.get(cur) ?? null;
    }
  }
  return out;
}

export function validateModel(model: M.Model): string[] {
  const errors: string[] = [];
  const err = (msg: string) => errors.push(msg);

  const params = new Map(model.parameters.map((p) => [p.id, p]));
  const groups = new Set(model.parameter_groups.map((g) => g.id));
  const parts = new Set(model.parts.map((p) => p.id));
  const deformers = new Map(model.deformers.map((d) => [d.id, d]));
  const meshes = new Map(model.meshes.map((m) => [m.id, m]));

  // 1. ID の重複
  const kinds: [string, string[]][] = [
    ["parameter", model.parameters.map((x) => x.id)],
    ["parameter group", model.parameter_groups.map((x) => x.id)],
    ["part", model.parts.map((x) => x.id)],
    ["deformer", model.deformers.map((x) => x.id)],
    ["mesh", model.meshes.map((x) => x.id)],
    ["glue", model.glues.map((x) => x.id)],
    ["hit area", model.hit_areas.map((x) => x.id)],
    ["slot", model.slots.map((x) => x.id)],
  ];
  for (const [kind, ids] of kinds) {
    for (const id of duplicates(ids)) err(`1: duplicate ${kind} id "${id}"`);
  }

  // 2. パラメータ
  for (const p of model.parameters) {
    if (!(p.min <= p.default && p.default <= p.max))
      err(`2: parameter ${p.id}: min <= default <= max`);
    if (p.mirror !== null && !params.has(p.mirror)) err(`2: parameter ${p.id}: unknown mirror`);
    if (p.group !== null && !groups.has(p.group)) err(`2: parameter ${p.id}: unknown group`);
  }

  // 3. 参照
  const ref = (ok: boolean, what: string) => ok || err(`3: ${what}`);
  for (const d of model.deformers) {
    ref(parts.has(d.part), `deformer ${d.id}: unknown part ${d.part}`);
    ref(d.parent === null || deformers.has(d.parent), `deformer ${d.id}: unknown parent`);
  }
  for (const m of model.meshes) {
    ref(parts.has(m.part), `mesh ${m.id}: unknown part ${m.part}`);
    ref(m.parent === null || deformers.has(m.parent), `mesh ${m.id}: unknown parent`);
    for (const mask of m.masks) ref(meshes.has(mask.mesh), `mesh ${m.id}: unknown mask`);
  }
  for (const p of model.parts) {
    ref(p.parent === null || parts.has(p.parent), `part ${p.id}: unknown parent`);
  }
  for (const g of model.glues) {
    ref(meshes.has(g.mesh_a) && meshes.has(g.mesh_b), `glue ${g.id}: unknown mesh`);
  }
  for (const h of model.hit_areas) ref(meshes.has(h.mesh), `hit area ${h.id}: unknown mesh`);
  for (const g of model.pose.groups) {
    for (const e of g.entries) {
      for (const id of [e.part, ...e.links]) ref(parts.has(id), `pose: unknown part ${id}`);
    }
  }
  const fx = model.effects;
  for (const id of [
    ...fx.eye_blink.parameters,
    ...fx.breath.parameters.map((b) => b.parameter),
    ...fx.lip_sync.parameters,
  ]) {
    ref(params.has(id), `effects: unknown parameter ${id}`);
  }

  // 4. 親の連鎖の循環
  for (const id of cyclic(model.parts)) err(`4: part ${id}: cyclic parent chain`);
  for (const id of cyclic(model.deformers)) err(`4: deformer ${id}: cyclic parent chain`);

  // 5. マスクの循環
  {
    const state = new Map<string, "visiting" | "done">();
    const visit = (id: string): boolean => {
      const s = state.get(id);
      if (s === "done") return false;
      if (s === "visiting") return true;
      state.set(id, "visiting");
      const m = meshes.get(id);
      const cycle = m ? m.masks.some((mask) => visit(mask.mesh)) : false;
      state.set(id, "done");
      return cycle;
    };
    for (const m of model.meshes) {
      state.clear();
      if (visit(m.id)) err(`5: mesh ${m.id}: cyclic mask references`);
    }
  }

  // 6.・7. メッシュ
  for (const m of model.meshes) {
    if (m.texture >= model.textures.length) err(`6: mesh ${m.id}: texture out of range`);
    const n = m.uvs.length;
    if (m.indices.length % 3 !== 0) err(`7: mesh ${m.id}: indices not a multiple of 3`);
    if (m.indices.some((i) => i >= n)) err(`7: mesh ${m.id}: index out of range`);
    for (const k of allKeyforms(m.keyforms)) {
      if (k.positions.length !== n) err(`7: mesh ${m.id}: keyform has wrong number of positions`);
      if (!finite(k.mask_strength)) err(`7: mesh ${m.id}: mask_strength not finite`);
    }
  }

  // 8. グルー
  for (const g of model.glues) {
    const a = meshes.get(g.mesh_a);
    const b = meshes.get(g.mesh_b);
    for (const pair of g.pairs) {
      if (a && pair.a >= a.uvs.length) err(`8: glue ${g.id}: vertex a out of range`);
      if (b && pair.b >= b.uvs.length) err(`8: glue ${g.id}: vertex b out of range`);
    }
  }

  // 9. ワープデフォーマ
  for (const d of model.deformers) {
    if (d.kind.type !== "warp") continue;
    const { rows, cols } = d.kind;
    if (rows < 1 || cols < 1) err(`9: warp ${d.id}: rows and cols must be at least 1`);
    for (const k of allKeyforms(d.kind.keyforms)) {
      if (k.points.length !== (rows + 1) * (cols + 1))
        err(`9: warp ${d.id}: wrong number of points`);
    }
  }

  // 10.・11. キーフォームのグリッド
  const sameKind = {
    part: (id: string) => parts.has(id),
    deformer: (id: string) => deformers.has(id),
    mesh: (id: string) => meshes.has(id),
    glue: (id: string) => model.glues.some((g) => g.id === id),
  };
  const checkGrid = (
    where: string,
    grid: M.KeyformGrid<unknown>,
    owner: keyof typeof sameKind,
    blendAllowed: boolean,
    isBlend: boolean,
  ) => {
    const count = grid.bindings.reduce((n, b) => n * b.keys.length, 1);
    if (grid.keyforms.length !== count) err(`10: ${where}: number of keyforms is not ${count}`);
    const bound = new Set<string>();
    for (const b of grid.bindings) {
      if (b.keys.length < 1) err(`10: ${where}: binding without keys`);
      for (let i = 1; i < b.keys.length; i++) {
        if (!(b.keys[i - 1] < b.keys[i])) err(`10: ${where}: keys not strictly increasing`);
      }
      if (b.keys.some((k) => Number.isNaN(k))) err(`10: ${where}: NaN key`);
      const p = params.get(b.parameter);
      if (!p) err(`10: ${where}: unknown parameter ${b.parameter}`);
      if (bound.has(b.parameter)) err(`10: ${where}: parameter ${b.parameter} bound twice`);
      bound.add(b.parameter);
      if (p && !isBlend && p.kind === "blend_shape") {
        err(`11: ${where}: blend_shape parameter ${p.id} on a normal grid`);
      }
      if (p && isBlend && p.kind === "normal") {
        err(`11: ${where}: normal parameter ${p.id} on a blend-shape grid`);
      }
    }
    if (grid.origins.length !== 0 && grid.origins.length !== grid.keyforms.length) {
      err(`10: ${where}: origins length`);
    }
    const ms = grid.mirror_source;
    if (ms) {
      if (!sameKind[owner](ms.id)) err(`10: ${where}: unknown mirror_source ${ms.id}`);
      if (
        (ms.axis !== null && !finite(ms.axis)) ||
        (ms.tolerance !== null && !finite(ms.tolerance))
      ) {
        err(`10: ${where}: mirror_source axis or tolerance not finite`);
      }
    }
    const inf = grid.inference;
    if (inf?.method === "depth_rotation") {
      if (![inf.angle_scale, ...inf.pivot, inf.pivot_depth].every(finite)) {
        err(`10: ${where}: inference values not finite`);
      }
      for (const id of [inf.yaw, inf.pitch]) {
        if (id !== null && (!params.has(id) || !bound.has(id))) {
          err(`10: ${where}: inference parameter ${id} is not an axis of the grid`);
        }
      }
    }
    if (isBlend) {
      if (grid.bindings.length === 0) err(`11: ${where}: blend-shape grid without bindings`);
      if (grid.blend_shapes.length !== 0) err(`11: ${where}: nested blend shapes`);
    } else if (!blendAllowed && grid.blend_shapes.length !== 0) {
      err(`11: ${where}: blend shapes are not allowed here`);
    }
    if (!isBlend) {
      const seen = new Set<string>();
      grid.blend_shapes.forEach((b, i) => {
        checkGrid(`${where} blend_shapes[${i}]`, b, owner, false, true);
        for (const binding of b.bindings) {
          if (seen.has(binding.parameter)) {
            err(`11: ${where}: parameter ${binding.parameter} is an axis of two blend-shape grids`);
          }
        }
        for (const binding of b.bindings) seen.add(binding.parameter);
      });
    }
  };
  for (const p of model.parts) checkGrid(`part ${p.id}`, p.keyforms, "part", false, false);
  for (const d of model.deformers) {
    if (d.kind.type === "warp" || d.kind.type === "rotation") {
      checkGrid(`deformer ${d.id}`, d.kind.keyforms, "deformer", true, false);
    }
  }
  for (const m of model.meshes) checkGrid(`mesh ${m.id}`, m.keyforms, "mesh", true, false);
  for (const g of model.glues) checkGrid(`glue ${g.id}`, g.keyforms, "glue", false, false);

  // 12. ミラーの組
  const mirror = (
    kind: string,
    items: Map<string, { mirror: string | null; mirror_axis: number | null }>,
  ) => {
    for (const [id, x] of items) {
      if (x.mirror_axis !== null && !finite(x.mirror_axis))
        err(`12: ${kind} ${id}: mirror_axis not finite`);
      if (x.mirror === null || x.mirror === id) continue;
      const other = items.get(x.mirror);
      if (!other) err(`12: ${kind} ${id}: unknown mirror ${x.mirror}`);
      else if (other.mirror !== id) err(`12: ${kind} ${id}: mirror pair is not bidirectional`);
    }
  };
  mirror("mesh", meshes);
  mirror("deformer", deformers);

  // 13. 深さ
  for (const m of model.meshes) {
    if (m.depth && (m.depth.length !== m.uvs.length || !m.depth.every(finite))) {
      err(`13: mesh ${m.id}: depth`);
    }
  }
  for (const d of model.deformers) {
    if (d.kind.type !== "warp" || !d.kind.depth) continue;
    const n = (d.kind.rows + 1) * (d.kind.cols + 1);
    if (d.kind.depth.length !== n || !d.kind.depth.every(finite)) err(`13: warp ${d.id}: depth`);
  }

  // 14. ポーズ
  {
    const all = model.pose.groups.flatMap((g) => g.entries.flatMap((e) => [e.part, ...e.links]));
    for (const id of duplicates(all)) err(`14: pose: part ${id} appears more than once`);
    const f = model.pose.fade_duration;
    if (!(finite(f) && f >= 0)) err("14: pose: fade_duration");
  }

  // 15. 効果
  {
    const b = fx.eye_blink;
    if (![b.closing, b.closed, b.opening].every((x) => finite(x) && x >= 0)) {
      err("15: eye blink: closing, closed and opening must be finite and at least 0");
    }
    if (!(finite(b.interval) && b.interval > 0)) err("15: eye blink: interval");
    for (const p of fx.breath.parameters) {
      if (!(finite(p.cycle) && p.cycle > 0)) err(`15: breath ${p.parameter}: cycle`);
    }
  }

  // 16. スロット
  {
    const entryParts = new Set(model.pose.groups.flatMap((g) => g.entries.map((e) => e.part)));
    for (const s of model.slots) {
      if (s.anchor !== null && !deformers.has(s.anchor)) err(`16: slot ${s.id}: unknown anchor`);
      if (s.part !== null && !parts.has(s.part)) err(`16: slot ${s.id}: unknown part`);
      for (const id of s.replaces) if (!parts.has(id)) err(`16: slot ${s.id}: unknown part ${id}`);
      for (const id of s.parameters) {
        if (!params.has(id)) err(`16: slot ${s.id}: unknown parameter ${id}`);
      }
      for (const id of s.mask_sources) {
        if (!meshes.has(id)) err(`16: slot ${s.id}: unknown mesh ${id}`);
      }
      for (const id of s.pose_parts) {
        if (!entryParts.has(id)) err(`16: slot ${s.id}: ${id} is not the part of a pose entry`);
      }
      const r = s.draw_order;
      if (!(finite(r.min) && finite(r.max) && r.min <= r.max)) err(`16: slot ${s.id}: draw_order`);
    }
  }

  return errors;
}

/** グリッドのキーフォームと、ブレンドシェイプのグリッドのキーフォーム。 */
function allKeyforms<T>(grid: M.KeyformGrid<T>): T[] {
  return [...grid.keyforms, ...grid.blend_shapes.flatMap((b) => allKeyforms(b))];
}

/** 物理の検証 (spec/model.md#physics-validation)。モデルと合わせて行う。 */
export function validatePhysics(
  model: M.Model,
  physics: { rigs: M.PhysicsRig[]; colliders: M.PhysicsCollider[] },
  options: { skipModelColliders?: boolean } = {},
): string[] {
  const errors: string[] = [];
  const err = (msg: string) => errors.push(`physics ${msg}`);
  for (const id of duplicates(physics.rigs.map((r) => r.id))) err(`1: duplicate rig id ${id}`);
  for (const id of duplicates(physics.colliders.map((c) => c.id))) {
    err(`1: duplicate collider id ${id}`);
  }
  const params = new Set(model.parameters.map((p) => p.id));
  const deformers = new Map(model.deformers.map((d) => [d.id, d]));
  const meshes = new Map(model.meshes.map((m) => [m.id, m]));
  const colliders = new Set(physics.colliders.map((c) => c.id));
  for (const rig of physics.rigs) {
    for (const input of rig.inputs) {
      if (!params.has(input.parameter))
        err(`2: rig ${rig.id}: unknown parameter ${input.parameter}`);
    }
    for (const out of rig.outputs) {
      const t = out.target;
      if (t.type === "parameter" && !params.has(t.id)) {
        err(`2: rig ${rig.id}: unknown parameter ${t.id}`);
      }
      if (t.type === "deformer_angle" || t.type === "deformer_offset") {
        const d = deformers.get(t.deformer);
        if (!d) err(`2: rig ${rig.id}: unknown deformer ${t.deformer}`);
        else if (d.kind.type === "warp") err(`3: rig ${rig.id}: ${t.deformer} is not a rotation`);
      }
      if (t.type === "vertices") {
        let count: number | null = null;
        if (t.owner.kind === "mesh") {
          const m = meshes.get(t.owner.id);
          if (!m) err(`2: rig ${rig.id}: unknown mesh ${t.owner.id}`);
          else count = m.uvs.length;
        } else {
          const d = deformers.get(t.owner.id);
          if (!d) err(`2: rig ${rig.id}: unknown deformer ${t.owner.id}`);
          else if (d.kind.type === "rotation") err(`3: rig ${rig.id}: ${t.owner.id} is not a warp`);
          else if (d.kind.type === "warp") count = (d.kind.rows + 1) * (d.kind.cols + 1);
        }
        if (count !== null && t.weights.length !== count) err(`4: rig ${rig.id}: weights length`);
        if (!t.weights.every((w) => w >= 0 && w <= 1))
          err(`4: rig ${rig.id}: weight out of [0, 1]`);
      }
    }
    for (const id of rig.colliders) {
      if (!colliders.has(id)) err(`2: rig ${rig.id}: unknown collider ${id}`);
    }
    if (rig.frame !== null && !deformers.has(rig.frame)) err(`2: rig ${rig.id}: unknown frame`);
    for (const p of rig.particles) {
      if (!(finite(p.collision_radius) && p.collision_radius >= 0)) {
        err(`5: rig ${rig.id}: collision_radius`);
      }
    }
  }
  for (const c of physics.colliders) {
    if (c.attach !== null && !deformers.has(c.attach)) err(`2: collider ${c.id}: unknown attach`);
    const s = c.shape;
    if (s.type === "circle" && !(finite(s.radius) && s.radius >= 0 && s.center.every(finite))) {
      err(`5: collider ${c.id}: shape`);
    }
    if (
      s.type === "capsule" &&
      !(finite(s.radius) && s.radius >= 0 && [...s.a, ...s.b].every(finite))
    ) {
      err(`5: collider ${c.id}: shape`);
    }
  }
  if (!options.skipModelColliders) {
    for (const s of model.slots) {
      for (const id of s.colliders) {
        if (!colliders.has(id)) err(`2: slot ${s.id}: unknown collider ${id}`);
      }
    }
  }
  return errors;
}

/** セグメントの点 (`p`・`to`、`bezier` の `c1`・`c2`) を名前つきで返す。 */
function segmentPoints(s: M.Segment): [string, M.Point][] {
  switch (s.type) {
    case "start":
      return [["p", s.p]];
    case "bezier":
      return [
        ["c1", s.c1],
        ["c2", s.c2],
        ["to", s.to],
      ];
    default:
      return [["to", s.to]];
  }
}

/**
 * モーションを単独で検証する (spec/model.md#motion-and-expression-validation の 1)。
 * 再生で無視する曲線 (知らない対象・知らない種類のセグメント) も、読めた数は検査する。
 */
export function validateMotion(motion: M.Motion): string[] {
  const errors: string[] = [];
  const err = (msg: string) => errors.push(`motion 1: ${msg}`);
  if (!finite(motion.duration) || motion.duration < 0) {
    err(`duration ${motion.duration} is not finite and at least 0`);
  }
  if (!finite(motion.fade_in)) err(`fade_in ${motion.fade_in} is not finite`);
  if (!finite(motion.fade_out)) err(`fade_out ${motion.fade_out} is not finite`);
  const curves: {
    fade_in: number | null;
    fade_out: number | null;
    segments: (M.Segment | null)[];
  }[] = [...motion.curves, ...(motion.ignored_curves ?? [])];
  curves.forEach((c, i) => {
    const where =
      i < motion.curves.length ? `curve ${i}` : `ignored curve ${i - motion.curves.length}`;
    if (c.fade_in !== null && !finite(c.fade_in))
      err(`${where}: fade_in ${c.fade_in} is not finite`);
    if (c.fade_out !== null && !finite(c.fade_out)) {
      err(`${where}: fade_out ${c.fade_out} is not finite`);
    }
    c.segments.forEach((s, k) => {
      if (s === null) return;
      for (const [name, pt] of segmentPoints(s)) {
        if (!finite(pt.t) || !finite(pt.v)) {
          err(`${where}: segment ${k}: ${name} (${pt.t}, ${pt.v}) is not finite`);
        }
      }
    });
  });
  motion.events.forEach((e, i) => {
    if (!finite(e.t)) err(`event ${i}: t ${e.t} is not finite`);
  });
  return errors;
}

/**
 * 表情を単独で検証する (spec/model.md#motion-and-expression-validation の 2)。
 * 知らない演算の項目の値も検査する。
 */
export function validateExpression(expression: M.Expression): string[] {
  const errors: string[] = [];
  const err = (msg: string) => errors.push(`expression 2: ${msg}`);
  if (!finite(expression.fade_in)) err(`fade_in ${expression.fade_in} is not finite`);
  if (!finite(expression.fade_out)) err(`fade_out ${expression.fade_out} is not finite`);
  expression.parameters.forEach((p, i) => {
    if (!finite(p.value)) err(`parameter ${i} (${p.parameter}): value ${p.value} is not finite`);
  });
  (expression.ignored_values ?? []).forEach((v, i) => {
    if (!finite(v)) err(`adjustment ${i} with an unknown op: value ${v} is not finite`);
  });
  return errors;
}

/** モジュールが使うパラメータ (spec/model.md#ids-and-references-inside-a-module)。 */
export function moduleParameters(mod: M.Module): Set<string> {
  const out = new Set<string>();
  const grid = (g: M.KeyformGrid<unknown>) => {
    for (const b of g.bindings) out.add(b.parameter);
    if (g.inference?.method === "depth_rotation") {
      if (g.inference.yaw !== null) out.add(g.inference.yaw);
      if (g.inference.pitch !== null) out.add(g.inference.pitch);
    }
    for (const b of g.blend_shapes) grid(b);
  };
  for (const p of mod.parts) grid(p.keyforms);
  for (const d of mod.deformers) if (d.kind.type !== "unknown") grid(d.kind.keyforms);
  for (const m of mod.meshes) grid(m.keyforms);
  for (const g of mod.glues) grid(g.keyforms);
  for (const r of mod.physics.rigs) {
    for (const i of r.inputs) out.add(i.parameter);
    for (const o of r.outputs) if (o.target.type === "parameter") out.add(o.target.id);
  }
  return out;
}

/** 名前の規則 (空でない・`/` を含まない・`#` で始まらない)。 */
export const validModuleName = (name: string): boolean =>
  name.length > 0 && !name.includes("/") && !name.startsWith("#");

/** モジュールを単独で検証する (spec/model.md#module-validation)。 */
export function validateModule(mod: M.Module): string[] {
  const parameters: M.Parameter[] = [...moduleParameters(mod)].map((id) => ({
    id,
    name: id,
    min: Number.NEGATIVE_INFINITY,
    max: Number.POSITIVE_INFINITY,
    default: 0,
    // どの種類のグリッドにも置ける種類
    kind: "unknown",
    mirror: null,
    group: null,
    internal: false,
  }));
  // モジュールのメッシュでないマスクは本体のメッシュを指すので、ここでは検査しない
  const ownMeshes = new Set(mod.meshes.map((m) => m.id));
  const asModel: M.Model = {
    schema_version: mod.schema_version,
    meta: { name: mod.name, canvas: { width: 0, height: 0, origin: [0, 0] } },
    parameters,
    parameter_groups: [],
    parts: mod.parts,
    deformers: mod.deformers,
    meshes: mod.meshes.map((m) => ({ ...m, masks: m.masks.filter((k) => ownMeshes.has(k.mesh)) })),
    glues: mod.glues,
    textures: mod.textures,
    hit_areas: [],
    pose: { fade_duration: 0.5, groups: mod.pose.groups },
    effects: {
      eye_blink: { parameters: [], interval: 4, closing: 0, closed: 0, opening: 0, seed: 0n },
      breath: { parameters: [] },
      lip_sync: { parameters: [] },
    },
    avatar: null,
    slots: [],
    lighting: false,
  };
  const errors = validateModel(asModel);
  const ownColliders = new Set(mod.physics.colliders.map((c) => c.id));
  errors.push(
    ...validatePhysics(
      asModel,
      {
        rigs: mod.physics.rigs.map((r) => ({
          ...r,
          // 本体のコライダーへの参照は取り付けのときに検査する
          colliders: r.colliders.filter((id) => ownColliders.has(id)),
        })),
        colliders: mod.physics.colliders,
      },
      { skipModelColliders: true },
    ),
  );
  if (!validModuleName(mod.name)) errors.push(`module name "${mod.name}" is not valid`);
  return errors;
}
