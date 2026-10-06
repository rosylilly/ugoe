import { readFileSync } from "node:fs";
import { setLocale } from "@ugoe/i18n";
import { afterEach, describe, expect, it } from "vitest";
import type { Classification, NormalMapGroupView, ProposalCommand } from "../ipc";
import {
  CHANGE_KINDS,
  changeLabel,
  changeSummary,
  classificationSummary,
  commandLabel,
  correctionLabel,
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

describe("changeSummary", () => {
  it("counts the selected commands by kind in the order they appear", () => {
    const more = [...commands, { op: "add_parameter", subject: "AngleY", requires: [] }];
    expect(changeSummary(more, new Set())).toEqual([
      { op: "add_parameter", count: 2, label: "add 2 parameters" },
      { op: "create_warp_deformer", count: 1, label: "create 1 warp deformer" },
      { op: "add_keyform_key", count: 1, label: "add 1 keyform key" },
      { op: "set_keyform_cell", count: 1, label: "place 1 keyform" },
      { op: "rename_mesh", count: 1, label: "rename 1 mesh" },
    ]);
    // 外したコマンドは数えない (すべて外した種類は出さない)
    expect(changeSummary(more, new Set([0, 2, 3, 4]))).toEqual([
      { op: "create_warp_deformer", count: 1, label: "create 1 warp deformer" },
      { op: "add_parameter", count: 1, label: "add 1 parameter" },
    ]);
    expect(changeSummary(more, new Set([0, 1, 2, 3, 4, 5]))).toEqual([]);
  });

  it("leaves a kind missing from the catalog without a label", () => {
    const unknown = [...commands, { op: "future_command", subject: null, requires: [] }];
    expect(changeSummary(unknown, new Set([0, 1, 2, 3, 4]))).toEqual([
      { op: "future_command", count: 1, label: null },
    ]);
  });
});

describe("changeLabel", () => {
  afterEach(() => setLocale("en"));

  it("says the change in the user's words with the count", () => {
    expect(changeLabel("add_parameter", 1)).toBe("add 1 parameter");
    expect(changeLabel("add_parameter", 5)).toBe("add 5 parameters");
    expect(changeLabel("set_keyform_cell", 12)).toBe("place 12 keyforms");
    setLocale("ja");
    expect(changeLabel("add_parameter", 5)).toBe("パラメータを足す × 5");
    expect(changeLabel("set_keyform_cell", 12)).toBe("キーフォームを置く × 12");
  });

  it("has a label for every kind of editor command", () => {
    // `ugoe_editor_core::Command` の種類 (`#[serde(tag = "op", rename_all = "snake_case")]`)
    const source = readFileSync(
      new URL("../../../../../crates/ugoe-editor-core/src/command.rs", import.meta.url),
      "utf8",
    );
    const body = source.slice(source.indexOf("pub enum Command {"));
    const kinds = [
      ...body.slice(0, body.indexOf("\n}\n")).matchAll(/^ {4}([A-Z][A-Za-z]+)\b/gm),
    ].map((m) => String(m[1]).replace(/[A-Z]/g, (c, i) => (i === 0 ? "" : "_") + c.toLowerCase()));
    expect(kinds).toContain("add_parameter");
    expect(Object.keys(CHANGE_KINDS).sort()).toEqual([...new Set(kinds)].sort());
  });

  it("returns null for a kind missing from the catalog", () => {
    expect(changeLabel("future_command", 2)).toBeNull();
    // オブジェクトの元から来る名前を種類と取り違えない
    expect(changeLabel("constructor", 1)).toBeNull();
    expect(changeLabel("toString", 1)).toBeNull();
  });
});

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

describe("layer corrections attached to a rig", () => {
  /** 0–2: 添えた分類の直し、3: パラメータ、4: キー (3 を使う)、5: 後から 1 つだけの直し。 */
  const rig: ProposalCommand[] = [
    { op: "set_layer_correction", subject: "Face / Eye L", requires: [] },
    { op: "set_layer_correction", subject: "Face / Eye R", requires: [] },
    { op: "set_layer_correction", subject: "Face / Mouth", requires: [] },
    { op: "add_parameter", subject: "EyeOpen", requires: [] },
    { op: "add_keyform_key", subject: "EyeOpen", requires: [3] },
    { op: "set_layer_correction", subject: "Hair", requires: [] },
  ];

  it("shows consecutive corrections as one group", () => {
    expect(groupCommands(rig)).toEqual([
      { label: "Keep the classification of 3 layers", indices: [0, 1, 2], labels: true },
      { label: "EyeOpen", indices: [3, 4] },
      { label: "Hair", indices: [5] },
    ]);
  });

  it("leaves out all corrections at once and keeps the rig", () => {
    const [labels, params] = groupCommands(rig);
    if (!labels || !params) throw new Error("groups");
    const out = toggleGroup(rig, new Set(), labels);
    expect(excludedList(out)).toEqual([0, 1, 2]);
    expect(groupState(labels, out)).toBe("none");
    expect(groupState(params, out)).toBe("all");
    expect(excludedList(toggleGroup(rig, out, labels))).toEqual([]);
  });

  it("leaves out one correction on its own", () => {
    const [labels] = groupCommands(rig);
    if (!labels) throw new Error("groups");
    const out = toggleCommand(rig, new Set(), 1);
    expect(excludedList(out)).toEqual([1]);
    expect(groupState(labels, out)).toBe("some");
    // 一部だけ外したまとまりの切り替えは、残りもすべて外す
    expect(excludedList(toggleGroup(rig, out, labels))).toEqual([0, 1, 2]);
  });

  it("does not merge corrections with other commands on the same subject", () => {
    const mixed: ProposalCommand[] = [
      { op: "rename_mesh", subject: "Hair", requires: [] },
      { op: "set_layer_correction", subject: "Hair", requires: [] },
      { op: "set_layer_correction", subject: "Hair", requires: [] },
      { op: "rename_mesh", subject: "Hair", requires: [] },
    ];
    expect(groupCommands(mixed)).toEqual([
      { label: "Hair", indices: [0] },
      { label: "Keep the classification of 2 layers", indices: [1, 2], labels: true },
      { label: "Hair", indices: [3] },
    ]);
  });
});

describe("commandLabel", () => {
  it("adds the subject when there is one", () => {
    expect(commandLabel(commands[0] as ProposalCommand)).toBe("add_parameter AngleX");
    expect(commandLabel(commands[4] as ProposalCommand)).toBe("rename_mesh");
  });

  it("shows the label and side a layer correction keeps", () => {
    const kept: ProposalCommand = {
      op: "set_layer_correction",
      subject: "Face / Eye R",
      requires: [],
      correction: { label: "eye_white", side: "right" },
    };
    expect(correctionLabel(kept)).toBe("Face / Eye R → eye_white (right)");
    expect(commandLabel(kept)).toBe("set_layer_correction Face / Eye R → eye_white (right)");
    const cleared: ProposalCommand = { op: "set_layer_correction", subject: "Hair", requires: [] };
    expect(correctionLabel(cleared)).toBe("Hair → back to the classification");
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
      variant: null as number | null,
      name_disagreement: null,
      corrected: false,
    });
    const c: Classification = {
      psd: "/a.psd",
      by_ai: false,
      instructions: null,
      label_choices: [],
      layers: [layer("eye_white", false), layer("face", true), layer("eye_white", false)],
    };
    expect(classificationSummary(c)).toEqual({
      layers: 3,
      review: 1,
      labels: [
        ["eye_white", 2],
        ["face", 1],
      ],
      variants: [],
    });
  });

  it("lists the alternative drawings the AI grouped", () => {
    const layer = (name: string, variant: number | null) => ({
      path: ["costume", name],
      label: "headwear",
      side: "center",
      confidence: 0.9,
      needs_review: false,
      variant,
      name_disagreement: null,
      corrected: false,
    });
    const c: Classification = {
      psd: "/a.psd",
      by_ai: true,
      instructions: null,
      label_choices: [],
      layers: [
        layer("sleeve b", 3),
        layer("hat", 0),
        layer("sleeve a", 3),
        layer("cap", 0),
        layer("x", null),
      ],
    };
    expect(classificationSummary(c).variants).toEqual([
      ["hat", "cap"],
      ["sleeve b", "sleeve a"],
    ]);
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
