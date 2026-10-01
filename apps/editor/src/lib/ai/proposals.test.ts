import { describe, expect, it } from "vitest";
import type { Classification, NormalMapGroupView, ProposalCommand } from "../ipc";
import {
  classificationSummary,
  commandLabel,
  excludeCommand,
  excludedList,
  groupCommands,
  groupState,
  includeCommand,
  instructionChars,
  instructionsToSend,
  normalMapChoices,
  normalMapPicks,
  normalMapTargetCount,
  sectionGroups,
  surfaceGroupLabel,
  toggleCommand,
  toggleGroup,
} from "./proposals";

/** 0: パラメータ、1: デフォーマ、2: キー (0 と 1 を使う)、3: 2 の後のセル (2 を使う)、4: 独立。 */
const commands: ProposalCommand[] = [
  { op: "add_parameter", subject: "AngleX", requires: [] },
  { op: "create_warp_deformer", subject: "Head", requires: [] },
  { op: "add_keyform_key", subject: "deformer `Head`", requires: [0, 1] },
  { op: "set_keyform_cell", subject: "deformer `Head`", requires: [1, 2] },
  { op: "rename_mesh", subject: null, requires: [] },
];

describe("excludeCommand", () => {
  it("also leaves out the commands that use what it creates", () => {
    expect(excludedList(excludeCommand(commands, new Set(), 0))).toEqual([0, 2, 3]);
    expect(excludedList(excludeCommand(commands, new Set(), 1))).toEqual([1, 2, 3]);
    expect(excludedList(excludeCommand(commands, new Set(), 3))).toEqual([3]);
    expect(excludedList(excludeCommand(commands, new Set([4]), 2))).toEqual([2, 3, 4]);
  });
});

describe("includeCommand", () => {
  it("also brings back the commands it depends on", () => {
    const all = new Set([0, 1, 2, 3, 4]);
    expect(excludedList(includeCommand(commands, all, 3))).toEqual([4]);
    expect(excludedList(includeCommand(commands, all, 2))).toEqual([3, 4]);
    expect(excludedList(includeCommand(commands, all, 4))).toEqual([0, 1, 2, 3]);
  });
});

describe("toggleCommand", () => {
  it("switches between the two", () => {
    const once = toggleCommand(commands, new Set(), 1);
    expect(excludedList(once)).toEqual([1, 2, 3]);
    expect(excludedList(toggleCommand(commands, once, 3))).toEqual([]);
  });
});

describe("groupCommands", () => {
  it("groups consecutive commands on the same subject", () => {
    expect(groupCommands(commands)).toEqual([
      { label: "AngleX", indices: [0] },
      { label: "Head", indices: [1] },
      { label: "deformer `Head`", indices: [2, 3] },
      { label: "rename_mesh", indices: [4] },
    ]);
  });

  it("toggles a whole group with what depends on it", () => {
    const [, head, keys] = groupCommands(commands);
    if (!head || !keys) throw new Error("groups");
    const out = toggleGroup(commands, new Set(), keys);
    expect(excludedList(out)).toEqual([2, 3]);
    expect(groupState(keys, out)).toBe("none");
    expect(groupState(keys, new Set([3]))).toBe("some");
    expect(excludedList(toggleGroup(commands, new Set([3]), keys))).toEqual([2, 3]);
    expect(
      excludedList(toggleGroup(commands, excludeCommand(commands, new Set(), 1), keys)),
    ).toEqual([]);
    expect(groupState(head, new Set())).toBe("all");
  });
});

describe("commandLabel", () => {
  it("adds the subject when there is one", () => {
    expect(commandLabel(commands[0] as ProposalCommand)).toBe("add_parameter AngleX");
    expect(commandLabel(commands[4] as ProposalCommand)).toBe("rename_mesh");
  });
});

describe("classificationSummary", () => {
  it("counts labels and layers to review", () => {
    const layer = (label: string, needs_review: boolean) => ({
      path: ["x"],
      label,
      side: "center",
      confidence: needs_review ? 0.3 : 0.9,
      needs_review,
    });
    const c: Classification = {
      psd: "/a.psd",
      by_ai: false,
      instructions: null,
      layers: [layer("eye_white", false), layer("face", true), layer("eye_white", false)],
    };
    expect(classificationSummary(c)).toEqual({
      layers: 3,
      review: 1,
      labels: [
        ["eye_white", 2],
        ["face", 1],
      ],
    });
  });
});

describe("instructionsToSend", () => {
  it("sends trimmed instructions only with AI", () => {
    expect(instructionsToSend("  目は細く \n", true)).toBe("目は細く");
    expect(instructionsToSend("目は細く", false)).toBeNull();
    expect(instructionsToSend(" \n ", true)).toBeNull();
  });
  it("counts characters like the backend", () => {
    expect(instructionChars(" 帽子は headwear ")).toBe(12);
    expect(instructionChars("😀")).toBe(1);
  });
});

describe("normal map proposals", () => {
  const group = (
    name: string | null,
    textures: string[],
    withNormalMap: string[] = [],
  ): NormalMapGroupView => ({
    group: name,
    textures,
    shape_only: [],
    oriented: [],
    with_normal_map: withNormalMap,
    // まとまりの格子の 1 画素がキャンバスの 0.5 px (単独のものはテクスチャの画素)。
    defaults: name
      ? { strength: 0.35, edge: 48, smoothing: 6 }
      : { strength: 0.5, edge: null, smoothing: 1 },
    canvas_per_pixel: name ? 0.5 : null,
    from_classification: false,
  });
  const groups = [group("face", ["face", "eye"], ["face"]), group(null, ["prop"])];

  it("includes every group at its defaults in canvas px and keeps earlier picks", () => {
    const picks = normalMapPicks(groups);
    expect(picks).toEqual({
      face: { include: true, strength: 0.35, roundEdges: true, edge: 24, smoothing: 3 },
      unlabeled: { include: true, strength: 0.5, roundEdges: false, edge: 12, smoothing: 1 },
    });
    const earlier = { include: false, strength: 1, roundEdges: false, edge: 10, smoothing: 2 };
    const kept = normalMapPicks(groups, { face: earlier });
    expect(kept.face).toEqual(earlier);
  });

  it("sends the included groups with their settings in grid pixels", () => {
    const picks = {
      face: { include: true, strength: 0.1, roundEdges: true, edge: 30, smoothing: 2 },
      unlabeled: { include: false, strength: 0.5, roundEdges: false, edge: 12, smoothing: 1 },
    };
    expect(normalMapChoices(groups, picks)).toEqual([
      { group: "face", settings: { strength: 0.1, edge: 60, smoothing: 4 } },
    ]);
    picks.face.roundEdges = false;
    picks.unlabeled.include = true;
    picks.unlabeled.roundEdges = true;
    expect(normalMapChoices(groups, picks)).toEqual([
      { group: "face", settings: { strength: 0.1, edge: null, smoothing: 4 } },
      { group: null, settings: { strength: 0.5, edge: 12, smoothing: 1 } },
    ]);
    picks.unlabeled.include = false;
    expect(normalMapTargetCount(groups, picks, false)).toBe(1);
    expect(normalMapTargetCount(groups, picks, true)).toBe(2);
  });

  it("names the sections", () => {
    expect(
      sectionGroups([
        { label: "face", indices: [0, 2] },
        { label: "lighting", indices: [3] },
      ]),
    ).toEqual([
      {
        label: "Face, eyes, nose, mouth, brows",
        indices: [0, 2],
        section: true,
      },
      { label: "Turn on lighting", indices: [3], section: true },
    ]);
    expect(surfaceGroupLabel("wings")).toBe("wings");
  });
});
