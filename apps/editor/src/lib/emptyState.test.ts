import { describe, expect, it } from "vitest";
import { isEmptyModel, treeEmptyState } from "./emptyState";
import { t } from "./i18n";

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

describe("treeEmptyState", () => {
  it("offers New from PSD for an empty project and Import PSD for a model without parts", () => {
    expect(treeEmptyState({ model: false, empty: false, view: "parts" }).action).toBeNull();
    expect(treeEmptyState({ model: true, empty: true, view: "parts" }).action).toBe("newFromPsd");
    expect(treeEmptyState({ model: true, empty: true, view: "deformers" }).action).toBe(
      "newFromPsd",
    );
    expect(treeEmptyState({ model: true, empty: false, view: "parts" }).action).toBe("importPsd");
    expect(treeEmptyState({ model: true, empty: false, view: "deformers" }).action).toBe(
      "importPsd",
    );
  });

  it("says what is missing for each view", () => {
    expect(treeEmptyState({ model: false, empty: false, view: "parts" }).text).toBe(
      t("tree.empty.noModel"),
    );
    expect(treeEmptyState({ model: true, empty: true, view: "parts" }).text).toBe(
      t("emptyState.tree"),
    );
    expect(treeEmptyState({ model: true, empty: false, view: "parts" }).text).toBe(
      t("tree.empty.parts"),
    );
    expect(treeEmptyState({ model: true, empty: false, view: "deformers" }).text).toBe(
      t("tree.empty.deformers"),
    );
  });
});
