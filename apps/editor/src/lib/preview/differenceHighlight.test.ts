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
  state: "changed",
  changes: [{ kind: "count", what: "vertices", before: 4, after: 6 }],
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
      differenceTarget(item({ state: "removed", element: null }), "CatEars", exists),
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
    const removed = item({ kind: "pose_follow", id: "ArmA", state: "removed", element: null });
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
    // 範囲 40×20 (高さは最小の 24 にする) を、ビューポートの半分 (200×100) に収める: 倍率は
    // min(200 / 40, 100 / 24) = 25 / 6。中心 (120, 110) をビューポートの中央 (200, 100) へ。
    expect(move?.factor).toBeCloseTo(25 / 6);
    expect(move?.center).toEqual([120, 110]);
    expect(move?.pan).toEqual([80, -10]);
    // 点 1 つでも寄りすぎない (最小の大きさ 24 を 240 の半分に: 倍率 5)。
    expect(fitMove([[10, 10]], 240, 240)?.factor).toBeCloseTo(5);
  });

  it("keeps the surroundings of a thin element in view (T1671)", () => {
    // 縦に細長い袖 (20×150) を 960×760 のビューポートへ: 長い辺 (高さ) で決まり、高さの半分 (380) に寄せる。
    // 横は 20 × 380 / 150 ≈ 51 で、周りが見える。
    const move = fitMove(
      [
        [400, 300],
        [420, 450],
      ],
      960,
      760,
    );
    expect(move?.factor).toBeCloseTo(380 / 150);
    expect(150 * (move?.factor ?? 0)).toBeCloseTo(760 / 2);
    // 横に細長いものは幅で決まる。
    expect(
      fitMove(
        [
          [0, 0],
          [300, 4],
        ],
        960,
        760,
      )?.factor,
    ).toBeCloseTo(480 / 300);
    // 画面より大きい要素は縮小する。
    expect(
      fitMove(
        [
          [0, 0],
          [1920, 100],
        ],
        960,
        760,
      )?.factor,
    ).toBeCloseTo(0.25);
  });

  it("does not move without points or a viewport, and takes many points", () => {
    // 描いたものが無ければ「Fit」は押せず (`found`)、動きも無い。
    expect(fitMove([], 400, 200)).toBeNull();
    expect(fitMove([[10, 10]], 0, 200)).toBeNull();
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
    expect(highlightNotice({ id: "Brim", state: "added" }, own, true)).toBe(
      "Brim in your current edits",
    );
    expect(highlightNotice({ id: "Brim", state: "removed" }, own, false)).toBe(
      "Brim was removed in your current edits. Put on the file to see it.",
    );
    expect(highlightNotice({ id: "Brim", state: "added" }, file, false)).toBe(
      "Brim is not in the file. Put on your current edits to see it.",
    );
    // 除いた要素のメッシュが今の編集に残っていても、除いたと分かるようにする (T0661)。
    expect(highlightNotice({ id: "ArmA", state: "removed" }, own, true)).toBe(
      "Removed: ArmA is not in your current edits; the outline shows its meshes that are still there. Put on the file to see it.",
    );
    expect(highlightNotice({ id: "Brim", state: "added" }, file, true)).toBe(
      "Added: Brim is not in the file; the outline shows its meshes that are still there. Put on your current edits to see it.",
    );
    expect(missingIn({ state: "removed" }, own)).toBe(true);
    expect(missingIn({ state: "removed" }, file)).toBe(false);
    expect(missingIn({ state: "added" }, file)).toBe(true);
    expect(missingIn({ state: "added" }, null)).toBe(false);
    expect(highlightNotice({ id: "Brim", state: "added" }, null, false)).toBe(
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
