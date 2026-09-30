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
  const defaults = { strength: 0.35, edge: 48, smoothing: 6 };
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
    defaults,
    from_classification: false,
  });
  const groups = [group("face", ["face", "eye"], ["face"]), group(null, ["prop"])];

  it("includes every group at its default strength and keeps earlier picks", () => {
    const picks = normalMapPicks(groups);
    expect(picks).toEqual({
      face: { include: true, strength: 0.35 },
      unlabeled: { include: true, strength: 0.35 },
    });
    const kept = normalMapPicks(groups, { face: { include: false, strength: 1 } });
    expect(kept.face).toEqual({ include: false, strength: 1 });
  });

  it("sends the included groups with their strength", () => {
    const picks = {
      face: { include: true, strength: 0.1 },
      unlabeled: { include: false, strength: 0.5 },
    };
    expect(normalMapChoices(groups, picks)).toEqual([
      { group: "face", settings: { strength: 0.1, edge: 48, smoothing: 6 } },
    ]);
    expect(normalMapTargetCount(groups, picks, false)).toBe(1);
    expect(normalMapTargetCount(groups, picks, true)).toBe(2);
  });

  it("names the sections", () => {
    expect(
      sectionGroups([
        { label: "face", indices: [0, 2], subjects: ["face", "eye"] },
        { label: "lighting", indices: [3], subjects: [""] },
      ]),
    ).toEqual([
      {
        label: "Face, eyes, nose, mouth, brows",
        indices: [0, 2],
        section: true,
        subjects: ["face", "eye"],
      },
      { label: "Turn on lighting", indices: [3], section: true, subjects: [""] },
    ]);
    expect(surfaceGroupLabel("wings")).toBe("wings");
  });
});
