// 着せ替えのモジュールの取り付け (spec/model.md#composition)。

import type * as M from "./types.ts";
import {
  isWellKnownParameter,
  moduleParameters,
  validateModel,
  validatePhysics,
  validModuleName,
} from "./validate.ts";

export class ComposeError extends Error {
  readonly details: string[];
  constructor(details: string[]) {
    super(`composition failed:\n  ${details.join("\n  ")}`);
    this.details = details;
  }
}

/** モデルに物理の設定がないとき、モジュールの物理をまとめる設定 (参照実装に合わせた既定)。 */
const DEFAULT_PHYSICS: M.PhysicsSettings = {
  gravity: [0, 1],
  wind: [0, 0],
  fps: 60,
  rigs: [],
  colliders: [],
};

export interface Composed {
  model: M.Model;
  physics: M.PhysicsSettings | null;
}

/** モジュールを与えた順に取り付けた、普通のモデルと物理の設定を返す。 */
export function compose(
  model: M.Model,
  physics: M.PhysicsSettings | null,
  modules: M.Module[],
): Composed {
  // 1. 取り付けられるか
  const errors: string[] = [];
  const names = new Set<string>();
  const occupied = new Set<string>();
  const modelParams = new Set(model.parameters.map((p) => p.id));
  const modelSlots = new Map(model.slots.map((s) => [s.id, s]));
  for (const mod of modules) {
    if (!validModuleName(mod.name)) errors.push(`module name "${mod.name}" is not valid`);
    if (names.has(mod.name)) errors.push(`module name "${mod.name}" is used twice`);
    names.add(mod.name);
    const slot = modelSlots.get(mod.slot);
    if (!slot) {
      errors.push(`${mod.name}: the model has no slot ${mod.slot}`);
      continue;
    }
    if (occupied.has(mod.slot)) errors.push(`${mod.name}: slot ${mod.slot} is already occupied`);
    occupied.add(mod.slot);
    const r = mod.requires;
    // プロファイルの無い本体は満たさない (level 0 を求める場合も)
    if (r.avatar_level !== null && (model.avatar === null || model.avatar.level < r.avatar_level)) {
      errors.push(`${mod.name}: requires avatar level ${r.avatar_level}`);
    }
    for (const s of r.slots) if (!modelSlots.has(s)) errors.push(`${mod.name}: requires slot ${s}`);
    for (const p of r.parameters) {
      if (!modelParams.has(p)) errors.push(`${mod.name}: requires parameter ${p}`);
    }
    for (const p of moduleParameters(mod)) {
      if (!modelParams.has(p))
        errors.push(`${mod.name}: parameter ${p} does not exist in the model`);
      else if (!isWellKnownParameter(p) && !slot.parameters.includes(p)) {
        errors.push(`${mod.name}: parameter ${p} is not well-known nor listed in the slot`);
      }
    }
  }
  if (errors.length > 0) throw new ComposeError(errors);

  // 2. 要素を後ろに足す
  const out: M.Model = structuredClone(model);
  const outPhysics: M.PhysicsSettings = structuredClone(physics ?? DEFAULT_PHYSICS);
  const modelColliders = new Set((physics?.colliders ?? []).map((c) => c.id));
  for (const mod of modules) {
    const slot = modelSlots.get(mod.slot) as M.Slot;
    const rn = (id: string) => `${mod.name}/${id}`;
    const ownMeshes = new Set(mod.meshes.map((m) => m.id));
    const ownColliders = new Set(mod.physics.colliders.map((c) => c.id));
    const grid = <T>(g: M.KeyformGrid<T>, map: (k: T) => T): M.KeyformGrid<T> => ({
      ...g,
      keyforms: g.keyforms.map(map),
      blend_shapes: g.blend_shapes.map((b) => grid(b, map)),
      mirror_source: g.mirror_source && { ...g.mirror_source, id: rn(g.mirror_source.id) },
    });
    const same = <T>(k: T) => k;
    const textureBase = out.textures.length;
    const { min, max } = slot.draw_order;

    for (const p of mod.parts) {
      out.parts.push({
        ...p,
        id: rn(p.id),
        parent: p.parent === null ? slot.part : rn(p.parent),
        keyforms: grid(p.keyforms, same),
      });
    }
    for (const d of mod.deformers) {
      let kind: M.DeformerKind = d.kind;
      if (kind.type === "warp") kind = { ...kind, keyforms: grid(kind.keyforms, same) };
      if (kind.type === "rotation") kind = { ...kind, keyforms: grid(kind.keyforms, same) };
      out.deformers.push({
        ...d,
        id: rn(d.id),
        part: rn(d.part),
        parent: d.parent === null ? slot.anchor : rn(d.parent),
        kind,
        mirror: d.mirror === null ? null : rn(d.mirror),
      });
    }
    for (const m of mod.meshes) {
      const masks = m.masks.map((k) => {
        if (ownMeshes.has(k.mesh)) return { ...k, mesh: rn(k.mesh) };
        if (!slot.mask_sources.includes(k.mesh)) {
          throw new ComposeError([`${mod.name}: mask ${k.mesh} is not a mask source of the slot`]);
        }
        return k;
      });
      out.meshes.push({
        ...m,
        id: rn(m.id),
        part: rn(m.part),
        parent: m.parent === null ? slot.anchor : rn(m.parent),
        texture: m.texture + textureBase,
        masks,
        mirror: m.mirror === null ? null : rn(m.mirror),
        keyforms: grid(m.keyforms, (k) => ({
          ...k,
          draw_order: min + ((max - min) * k.draw_order) / 1000,
        })),
      });
    }
    for (const g of mod.glues) {
      out.glues.push({
        ...g,
        id: rn(g.id),
        mesh_a: rn(g.mesh_a),
        mesh_b: rn(g.mesh_b),
        keyforms: grid(g.keyforms, same),
      });
    }
    for (const t of mod.textures) out.textures.push({ ...t, name: rn(t.name) });
    for (const group of mod.pose.groups) {
      out.pose.groups.push({
        entries: group.entries.map((e) => ({ part: rn(e.part), links: e.links.map(rn) })),
      });
    }
    for (const link of mod.pose.links) {
      const entry = out.pose.groups.flatMap((g) => g.entries).find((e) => e.part === link.part);
      if (!slot.pose_parts.includes(link.part) || !entry) {
        throw new ComposeError([`${mod.name}: pose link to ${link.part} is not allowed`]);
      }
      entry.links.push(...link.links.map(rn));
    }
    for (const rig of mod.physics.rigs) {
      const colliders = rig.colliders.map((id) => {
        if (ownColliders.has(id)) return rn(id);
        if (!slot.colliders.includes(id) || !modelColliders.has(id)) {
          throw new ComposeError([`${mod.name}: collider ${id} is not allowed by the slot`]);
        }
        return id;
      });
      outPhysics.rigs.push({
        ...rig,
        id: rn(rig.id),
        colliders,
        frame: rig.frame === null ? null : rn(rig.frame),
        outputs: rig.outputs.map((o) => ({ ...o, target: renameTarget(o.target, rn) })),
      });
    }
    for (const c of mod.physics.colliders) {
      outPhysics.colliders.push({
        ...c,
        id: rn(c.id),
        attach: c.attach === null ? slot.anchor : rn(c.attach),
      });
    }
  }

  // 3. 置き換えるパーツを隠す
  const replaced = new Set<string>();
  for (const s of model.slots) {
    if (!occupied.has(s.id)) continue;
    for (const id of s.replaces) {
      if (replaced.has(id)) continue;
      replaced.add(id);
      const part = out.parts.find((p) => p.id === id) as M.Part;
      const hider: M.Part = {
        id: `#replaced/${id}`,
        name: `#replaced/${id}`,
        parent: part.parent,
        keyforms: {
          bindings: [],
          keyforms: [{ opacity: 0 }],
          interpolation: "linear",
          blend_shapes: [],
          origins: [],
          inference: null,
          mirror_source: null,
        },
      };
      part.parent = hider.id;
      out.parts.push(hider);
    }
  }

  // 4. 検証
  const composedPhysics = physics === null && outPhysics.rigs.length === 0 ? null : outPhysics;
  const invalid = validateModel(out);
  if (composedPhysics) invalid.push(...validatePhysics(out, composedPhysics));
  if (invalid.length > 0) throw new ComposeError(invalid);
  return { model: out, physics: composedPhysics };
}

function renameTarget(t: M.PhysicsTarget, rn: (id: string) => string): M.PhysicsTarget {
  switch (t.type) {
    case "deformer_angle":
    case "deformer_offset":
      return { ...t, deformer: rn(t.deformer) };
    case "vertices":
      return { ...t, owner: { ...t.owner, id: rn(t.owner.id) } };
    default:
      return t;
  }
}
