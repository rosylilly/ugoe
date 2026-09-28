import { describe, expect, it } from "vitest";
import { isEmptyModel } from "./emptyState";

const empty = { parts: [], meshes: [], deformers: [], textures: [] };

describe("isEmptyModel", () => {
  it("is true only when the model has no parts, meshes, deformers or textures", () => {
    expect(isEmptyModel(empty)).toBe(true);
    expect(isEmptyModel({ ...empty, parts: [{}] })).toBe(false);
    expect(isEmptyModel({ ...empty, meshes: [{}] })).toBe(false);
    expect(isEmptyModel({ ...empty, deformers: [{}] })).toBe(false);
    expect(isEmptyModel({ ...empty, textures: [{}] })).toBe(false);
  });
});
