import { describe, expect, it } from "vitest";
import type { Model, PhysicsParticle, PhysicsRig, PhysicsSettings } from "../../generated/types";
import {
  appendParticle,
  BLENDS,
  chainOrder,
  describeTarget,
  hasBlend,
  isDynamicRig,
  isDynamicSettings,
  makeTarget,
  moveRigCommand,
  newCollider,
  newOutput,
  newRig,
  outputBlend,
  relayInputs,
  removeParticle,
  setParticleField,
  targetKind,
  targetRef,
  uniqueId,
  worldCommand,
} from "./physics";

// 物理が参照するフィールドだけを持つモデル (型検査のためにキャストする)。
const model = {
  parameters: [{ id: "AngleX", name: "Angle X" }],
  deformers: [
    { id: "Hair2", name: "髪 2", parent: "Hair1", kind: { type: "rotation" } },
    { id: "Head", name: "頭", parent: null, kind: { type: "rotation" } },
    { id: "Hair1", name: "髪 1", parent: "Head", kind: { type: "rotation" } },
    { id: "Cloth", name: "Cloth", parent: null, kind: { type: "warp", rows: 1, cols: 2 } },
  ],
  meshes: [
    {
      id: "Skirt",
      name: "スカート",
      uvs: [
        [0, 0],
        [1, 0],
        [0, 1],
      ],
    },
  ],
} as unknown as Model;

const particle = (y: number, radius: number): PhysicsParticle => ({
  position: [0, y],
  mobility: 0.9,
  delay: 0.8,
  acceleration: 1.5,
  radius,
  collision_radius: 0,
});

describe("ids", () => {
  it("sanitizes and numbers ids like the backend", () => {
    expect(uniqueId("Hair", [])).toBe("Hair");
    expect(uniqueId("Hair", ["Hair", "Hair_2"])).toBe("Hair_3");
    expect(uniqueId("前髪 A", [])).toBe("___A");
    expect(uniqueId("前髪", [])).toBe("Item");
  });
});

describe("rigs", () => {
  it("creates an empty rig with a unique id", () => {
    const rig = newRig(null, "Rig 1");
    expect(rig.id).toBe("Rig_1");
    expect(rig.particles.map((p) => p.position)).toEqual([
      [0, 0],
      [0, 30],
    ]);
    expect(rig.particles[1]?.radius).toBe(30);
    const settings: PhysicsSettings = {
      gravity: [0, 1],
      wind: [0, 0],
      fps: 60,
      rigs: [rig],
      colliders: [],
    };
    expect(newRig({ ...settings, rigs: [rig] }, "Rig 1").id).toBe("Rig_1_2");
    expect(isDynamicRig(rig)).toBe(false);
    expect(isDynamicSettings({ ...settings, rigs: [rig] })).toBe(false);
    const dynamic: PhysicsRig = { ...rig, frame: "Head" };
    expect(isDynamicRig(dynamic)).toBe(true);
    expect(isDynamicSettings({ ...settings, rigs: [rig, dynamic] })).toBe(true);
    expect(isDynamicSettings(null)).toBe(false);
  });

  it("orders selected rotation deformers from root to tip", () => {
    expect(chainOrder(model, ["Hair2", "Cloth", "Hair1"])).toEqual(["Hair1", "Hair2"]);
  });

  it("builds world commands from the current settings", () => {
    expect(worldCommand(null, { fps: 30 })).toEqual({
      op: "set_physics_world",
      gravity: [0, 1],
      wind: [0, 0],
      fps: 30,
    });
  });
});

describe("particles", () => {
  it("appends along the last segment", () => {
    const chain = appendParticle([particle(0, 0), particle(20, 20)]);
    expect(chain[2]?.position).toEqual([0, 40]);
    expect(chain[2]?.radius).toBe(20);
    expect(chain[2]?.mobility).toBe(0.9);
    // 根元だけなら下向きに既定の長さ。
    const two = appendParticle([particle(0, 0)]);
    expect(two[1]?.position).toEqual([0, 30]);
    expect(appendParticle([])).toHaveLength(1);
  });

  it("removes a particle and joins its neighbours", () => {
    const chain = [particle(0, 0), particle(10, 10), particle(30, 20)];
    const removed = removeParticle(chain, 1);
    expect(removed.map((p) => p.position[1])).toEqual([0, 30]);
    expect(removed[1]?.radius).toBe(30);
    expect(removeParticle(chain, 0)).toEqual(chain);
  });

  it("sets fields and positions", () => {
    const chain = [particle(0, 0), particle(10, 10)];
    expect(setParticleField(chain, 1, "mobility", 0.5)[1]?.mobility).toBe(0.5);
    expect(setParticleField(chain, 1, "x", 4)[1]?.position).toEqual([4, 10]);
    expect(setParticleField(chain, 1, "y", 7)[1]?.position).toEqual([0, 7]);
    expect(chain[1]?.position).toEqual([0, 10]);
  });
});

describe("outputs", () => {
  it("makes targets of each kind", () => {
    expect(makeTarget(model, "deformer_angle", "Hair1")).toEqual({
      type: "deformer_angle",
      deformer: "Hair1",
    });
    const warp = makeTarget(model, "warp", "Cloth");
    expect(warp).toEqual({
      type: "vertices",
      owner: { warp: "Cloth" },
      weights: [1, 1, 1, 1, 1, 1],
    });
    expect(targetKind(warp)).toBe("warp");
    expect(targetRef(warp)).toBe("Cloth");
    // 同じ要素なら重みを引き継ぐ。
    const mesh = makeTarget(model, "mesh", "Skirt", {
      type: "vertices",
      owner: { mesh: "Skirt" },
      weights: [0, 0.5, 1],
    });
    expect(mesh).toEqual({ type: "vertices", owner: { mesh: "Skirt" }, weights: [0, 0.5, 1] });
  });

  it("describes targets with names", () => {
    expect(describeTarget(model, { type: "deformer_angle", deformer: "Hair1" })).toBe(
      "Deformer angle: 髪 1 (Hair1)",
    );
    expect(describeTarget(model, { type: "parameter", id: "AngleX" })).toBe(
      "Parameter: Angle X (AngleX)",
    );
  });

  it("creates outputs for the first candidate", () => {
    expect(newOutput(model, "deformer_angle", 2)).toEqual({
      target: { type: "deformer_angle", deformer: "Hair2" },
      particle: 2,
      source: "angle",
      scale: 1,
      weight: 100,
      reflect: false,
    });
    expect(newOutput(model, "mesh", 0)?.particle).toBe(1);
    expect(newOutput({ ...model, meshes: [] }, "mesh", 1)).toBeNull();
  });

  it("reads the blend of parameter outputs", () => {
    const output = newOutput(model, "parameter", 1);
    if (!output) throw new Error("no parameter output");
    expect(outputBlend(output)).toBe("overwrite");
    expect(hasBlend(output)).toBe(true);
    expect(outputBlend({ ...output, blend: "add" })).toBe("add");
    const angle = newOutput(model, "deformer_angle", 1);
    expect(angle && hasBlend(angle)).toBe(false);
    expect(BLENDS.map((b) => b.blend)).toEqual(["overwrite", "add"]);
  });
});

describe("colliders", () => {
  it("creates circles and capsules", () => {
    expect(newCollider(null, "circle", "Head", [0, 0], 10)).toEqual({
      id: "Circle",
      name: "Circle",
      attach: "Head",
      shape: { type: "circle", center: [0, 0], radius: 10 },
    });
    const capsule = newCollider(null, "capsule", null, [100, 50], 10);
    expect(capsule.attach).toBeUndefined();
    expect(capsule.shape).toEqual({ type: "capsule", a: [90, 50], b: [110, 50], radius: 10 });
  });
});

describe("rig order", () => {
  const rig = (id: string, inputs: string[], outputs: string[]): PhysicsRig => ({
    ...newRig(null, id),
    inputs: inputs.map((parameter) => ({ parameter, source: "x", weight: 100, reflect: false })),
    outputs: outputs.map((id) => ({
      target: { type: "parameter", id },
      particle: 1,
      source: "angle",
      scale: 1,
      weight: 100,
      reflect: false,
    })),
  });
  const settings: PhysicsSettings = {
    gravity: [0, 1],
    wind: [0, 0],
    fps: 60,
    rigs: [rig("A", ["AngleX"], ["Relay"]), rig("B", ["Relay", "AngleX"], []), rig("C", [], [])],
    colliders: [],
  };

  it("moves a rig up or down with reorder_physics_rig", () => {
    expect(moveRigCommand(settings, "B", -1)).toEqual({
      op: "reorder_physics_rig",
      id: "B",
      before: "A",
    });
    expect(moveRigCommand(settings, "A", 1)).toEqual({
      op: "reorder_physics_rig",
      id: "A",
      before: "C",
    });
    expect(moveRigCommand(settings, "B", 1)).toEqual({
      op: "reorder_physics_rig",
      id: "B",
      before: null,
    });
    expect(moveRigCommand(settings, "A", -1)).toBeNull();
    expect(moveRigCommand(settings, "C", 1)).toBeNull();
    expect(moveRigCommand(settings, "Nope", 1)).toBeNull();
    expect(moveRigCommand(null, "A", 1)).toBeNull();
  });

  it("finds inputs that read an earlier rig's output", () => {
    expect(relayInputs(settings, 0)).toEqual([]);
    expect(relayInputs(settings, 1)).toEqual(["Relay"]);
    expect(relayInputs({ ...settings, rigs: [...settings.rigs].reverse() }, 1)).toEqual([]);
  });
});
