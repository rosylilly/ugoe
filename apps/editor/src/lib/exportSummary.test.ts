import { describe, expect, it } from "vitest";
import { exportSummary } from "./exportSummary";
import type { DrawEstimate, ExportReport } from "./ipc";

const estimate = (textures: number, state_changes: number, draw_calls: number): DrawEstimate => ({
  textures,
  drawables: 4,
  draw_calls,
  state_changes,
});

describe("export summary", () => {
  it("shows unchanged counts once without an atlas", () => {
    const report: ExportReport = {
      bytes: 100,
      before: estimate(3, 2, 3),
      after: estimate(3, 2, 3),
      atlas: null,
    };
    expect(exportSummary(report)).toEqual(["Textures: 3", "State changes: 2", "Draw calls: 3"]);
  });

  it("shows before and after with the atlas details", () => {
    const report: ExportReport = {
      bytes: 100,
      before: estimate(4, 3, 4),
      after: estimate(2, 1, 2),
      atlas: {
        packed: 3,
        atlases: [[10, 20]],
        kept: [["bg", "uv_out_of_range"]],
        removed: ["unused"],
        deduplicated: [["eye_r", "eye_l"]],
      },
    };
    expect(exportSummary(report)).toEqual([
      "Textures: 4 → 2",
      "State changes: 3 → 1",
      "Draw calls: 4 → 2",
      "Packed 3 textures into 1 atlas (10×20)",
      "Same image: eye_r → eye_l",
      "Kept separate: bg (UV outside 0–1)",
      "Removed unused: unused",
    ]);
  });
});
