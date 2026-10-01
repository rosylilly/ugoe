import { describe, expect, it } from "vitest";
import type { DifferenceItem } from "../ipc";
import { convexHull, differenceTarget, highlightNotice, shownVersion } from "./differenceHighlight";

const item = (fields: Partial<DifferenceItem>): DifferenceItem => ({
  kind: "mesh",
  id: "Ears",
  text: "vertices 4 → 6",
  element: "CatEars/Ears",
  meshes: ["Ears"],
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

  it("selects the meshes at both ends of a glue that are still in the current edits", () => {
    const glue = item({
      kind: "glue",
      id: "Seam",
      element: "CatEars/Seam",
      meshes: ["Band", "Ears", "Old"],
    });
    expect(differenceTarget(glue, "CatEars", (id) => id !== "CatEars/Old")).toEqual({
      panel: "tree",
      refs: [
        { kind: "mesh", id: "CatEars/Band" },
        { kind: "mesh", id: "CatEars/Ears" },
      ],
    });
    expect(differenceTarget(glue, "CatEars", () => false)).toBeNull();
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
