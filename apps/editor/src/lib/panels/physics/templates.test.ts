import { describe, expect, it } from "vitest";
import type { Model } from "../../generated/types";
import {
  defaultNeck,
  missingParameters,
  neckCandidates,
  TEMPLATES,
  templateCommand,
} from "./templates";

// テンプレートが参照するフィールドだけを持つモデル (型検査のためにキャストする)。
const model = {
  parameters: [
    { id: "AngleX", name: "Angle X" },
    { id: "BodyAngleX", name: "Body X" },
  ],
  deformers: [
    { id: "Head", name: "頭", kind: { type: "rotation" } },
    { id: "Cloth", name: "Neck cloth", kind: { type: "warp", rows: 1, cols: 1 } },
    { id: "Rot2", name: "首の回転", kind: { type: "rotation" } },
  ],
} as unknown as Model;

describe("physics templates", () => {
  it("lists the missing standard parameters", () => {
    expect(missingParameters(model, "body_follow")).toEqual([
      "AngleY",
      "BodyAngleY",
      "AngleZ",
      "BodyAngleZ",
    ]);
    expect(missingParameters(model, "bounce")).toEqual([]);
    expect(missingParameters(model, "neck_follow")).toEqual(["AngleZ"]);
  });

  it("picks a rotation deformer named neck", () => {
    expect(neckCandidates(model).map((d) => d.id)).toEqual(["Head", "Rot2"]);
    // ワープは名前に neck があっても選ばない。
    expect(defaultNeck(model)).toBe("Rot2");
    expect(defaultNeck({ ...model, deformers: model.deformers.slice(0, 1) })).toBeNull();
  });

  it("builds the command", () => {
    expect(templateCommand("bounce", true, "add", "Rot2")).toEqual({
      op: "apply_physics_template",
      template: "bounce",
      options: { create_missing: true, blend: "add" },
    });
    expect(templateCommand("neck_follow", false, "overwrite", "Rot2")).toEqual({
      op: "apply_physics_template",
      template: "neck_follow",
      options: { create_missing: false, blend: "overwrite", neck: "Rot2" },
    });
    expect(TEMPLATES.map((t) => t.template)).toEqual(["body_follow", "bounce", "neck_follow"]);
  });
});
