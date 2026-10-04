import { describe, expect, it } from "vitest";
import type { DifferenceItem } from "../ipc";
import {
  convexHull,
  differenceTarget,
  fitMove,
  highlightNotice,
  missingIn,
  selectTitle,
  shownVersion,
} from "./differenceHighlight";

const item = (fields: Partial<DifferenceItem>): DifferenceItem => ({
  kind: "mesh",
  id: "Ears",
  text: "vertices 4 → 6",
  element: "CatEars/Ears",
  meshes: ["Ears"],
  parts: [],
  ...fields,
});

describe("differences in the preview (T0657)", () => {
  it("selects parts, deformers and meshes in the tree and physics in the Physics tab", () => {
    const exists = () => true;
    expect(differenceTarget(item({}), "CatEars", exists)).toEqual({
      panel: "tree",
      refs: [{ kind: "mesh", id: "CatEars/Ears" }],
    });
    expect(
      differenceTarget(
        item({ kind: "physics_rig", id: "Sway", element: "CatEars/Sway" }),
        "CatEars",
        exists,
      ),
    ).toEqual({ panel: "physics", rig: "CatEars/Sway", collider: null });
    expect(
      differenceTarget(
        item({ kind: "collider", id: "Collar", element: "CatEars/Collar" }),
        "CatEars",
        exists,
      ),
    ).toEqual({ panel: "physics", rig: null, collider: "CatEars/Collar" });
    // 除いた要素は今の編集に無いので選べない。
    expect(
      differenceTarget(item({ text: "removed", element: null }), "CatEars", exists),
    ).toBeNull();
  });

  it("selects a glue in the Glue tab and its meshes that are still in the current edits (T0659)", () => {
    const glue = item({
      kind: "glue",
      id: "Seam",
      element: "CatEars/Seam",
      meshes: ["Band", "Ears", "Old"],
    });
    expect(differenceTarget(glue, "CatEars", (id) => id !== "CatEars/Old")).toEqual({
      panel: "glue",
      glue: "CatEars/Seam",
      refs: [
        { kind: "mesh", id: "CatEars/Band" },
        { kind: "mesh", id: "CatEars/Ears" },
      ],
    });
    expect(differenceTarget(glue, "CatEars", () => false)).toEqual({
      panel: "glue",
      glue: "CatEars/Seam",
      refs: [],
    });
    expect(selectTitle(glue)).toBe("Leave the preview and select Seam in the Glue tab");
  });

  it("selects pose groups and follows in the Poses tab with their parts (T0659)", () => {
    const group = item({
      kind: "pose_group",
      id: "HatA / HatB",
      element: "HatA / HatB",
      parts: ["Hat/HatA", "Hat/HatB"],
    });
    expect(differenceTarget(group, "Hat", () => true)).toEqual({
      panel: "poses",
      refs: [
        { kind: "part", id: "Hat/HatA" },
        { kind: "part", id: "Hat/HatB" },
      ],
    });
    expect(selectTitle(group)).toBe(
      "Leave the preview and select HatA / HatB in the Poses tab and its parts in the tree",
    );
    const removed = item({ kind: "pose_follow", id: "ArmA", text: "removed", element: null });
    expect(differenceTarget(removed, "Hat", () => true)).toBeNull();
  });

  it("zooms to the highlighted element and centers it (T0659)", () => {
    const move = fitMove(
      [
        [100, 100],
        [140, 120],
      ],
      400,
      200,
    );
    // 範囲 40×20 (高さは最小の 24 にする) を、ビューポートの 80% (320×160) に収める: 倍率は
    // min(320 / 40, 160 / 24) = 20 / 3。中心 (120, 110) をビューポートの中央 (200, 100) へ。
    expect(move?.factor).toBeCloseTo(20 / 3);
    expect(move?.center).toEqual([120, 110]);
    expect(move?.pan).toEqual([80, -10]);
    // 点 1 つでも寄りすぎない (最小の大きさ 24 を 240 の 80% に: 倍率 8)。
    expect(fitMove([[10, 10]], 240, 240)?.factor).toBeCloseTo(8);
    expect(fitMove([], 400, 200)).toBeNull();
    // 引数の数に上限のある Math.min(...) では止まる大きさ (JavaScriptCore は 65536 個まで)。
    const many = Array.from({ length: 300_000 }, (_, i): [number, number] => [i % 100, 0]);
    expect(fitMove(many, 400, 200)?.center).toEqual([49.5, 0]);
  });

  it("highlights the version drawn in the viewport", () => {
    expect(shownVersion("Hat", false, [])).toEqual({ name: "Hat", own: true });
    expect(shownVersion("Hat", true, ["Hat (editing)", "Dress"])).toEqual({
      name: "Hat (editing)",
      own: true,
    });
    expect(shownVersion("Hat", true, ["Hat"])).toEqual({ name: "Hat", own: false });
    expect(shownVersion("Hat", true, ["Dress"])).toBeNull();
  });

  it("tells where the highlighted element is, or how to see it", () => {
    const own = { name: "Hat (editing)", own: true };
    const file = { name: "Hat", own: false };
    expect(highlightNotice({ id: "Brim", text: "added" }, own, true)).toBe(
      "Brim in your current edits",
    );
    expect(highlightNotice({ id: "Brim", text: "removed" }, own, false)).toBe(
      "Brim was removed in your current edits. Put on the file to see it.",
    );
    expect(highlightNotice({ id: "Brim", text: "added" }, file, false)).toBe(
      "Brim is not in the file. Put on your current edits to see it.",
    );
    // 除いた要素のメッシュが今の編集に残っていても、除いたと分かるようにする (T0661)。
    expect(highlightNotice({ id: "ArmA", text: "removed" }, own, true)).toBe(
      "Removed: ArmA is not in your current edits; the outline shows its meshes that are still there. Put on the file to see it.",
    );
    expect(highlightNotice({ id: "Brim", text: "added" }, file, true)).toBe(
      "Added: Brim is not in the file; the outline shows its meshes that are still there. Put on your current edits to see it.",
    );
    expect(missingIn({ text: "removed" }, own)).toBe(true);
    expect(missingIn({ text: "removed" }, file)).toBe(false);
    expect(missingIn({ text: "added" }, file)).toBe(true);
    expect(missingIn({ text: "added" }, null)).toBe(false);
    expect(highlightNotice({ id: "Brim", text: "added" }, null, false)).toBe(
      "Put on your current edits or the file to see Brim.",
    );
  });

  it("outlines points by their convex hull", () => {
    expect(
      convexHull([
        [0, 0],
        [2, 0],
        [1, 1],
        [2, 2],
        [0, 2],
      ]),
    ).toEqual([
      [0, 0],
      [2, 0],
      [2, 2],
      [0, 2],
    ]);
    expect(convexHull([[1, 1]])).toEqual([[1, 1]]);
  });
});
