import { describe, expect, it } from "vitest";
import type { Model, Parameter } from "../ipc";
import {
  defaultStages,
  EVENT_HISTORY,
  filterSections,
  followShown,
  groupParameters,
  modulePoseGroups,
  modulePoseLabels,
  previewPart,
  pushEvents,
  sliderStep,
  stageAvailability,
  visiblePoseParts,
} from "./preview";

function param(id: string, group: string | null = null, name = id): Parameter {
  return {
    id,
    name,
    min: -1,
    default: 0,
    max: 1,
    kind: "normal",
    mirror: null,
    group,
  } as Parameter;
}

describe("groupParameters", () => {
  it("follows the group order and keeps the model order inside a group", () => {
    const sections = groupParameters(
      [param("AngleX", "face"), param("EyeLOpen", "eyes"), param("AngleY", "face"), param("Other")],
      [
        { id: "eyes", name: "Eyes" },
        { id: "face", name: "Face" },
        { id: "empty", name: "Empty" },
      ],
      "Other",
    );
    expect(sections.map((s) => [s.id, s.name, s.parameters.map((p) => p.id)])).toEqual([
      ["eyes", "Eyes", ["EyeLOpen"]],
      ["face", "Face", ["AngleX", "AngleY"]],
      [null, "Other", ["Other"]],
    ]);
  });

  it("puts parameters of unknown groups in the last section", () => {
    const sections = groupParameters([param("A", "missing")], [], "Other");
    expect(sections).toEqual([{ id: null, name: "Other", parameters: [param("A", "missing")] }]);
  });

  it("names a group by its ID when the name is empty", () => {
    const sections = groupParameters([param("A", "g")], [{ id: "g", name: "" }], "Other");
    expect(sections[0]?.name).toBe("g");
  });
});

describe("filterSections", () => {
  const sections = groupParameters(
    [param("AngleX", "face", "顔 X"), param("EyeLOpen", "eyes", "左目")],
    [
      { id: "face", name: "Face" },
      { id: "eyes", name: "Eyes" },
    ],
    "Other",
  );

  it("matches the ID or the name, ignoring case", () => {
    expect(filterSections(sections, " anglex ").map((s) => s.id)).toEqual(["face"]);
    expect(filterSections(sections, "左目").map((s) => s.id)).toEqual(["eyes"]);
  });

  it("returns everything for an empty query and nothing for no match", () => {
    expect(filterSections(sections, "")).toBe(sections);
    expect(filterSections(sections, "zzz")).toEqual([]);
  });
});

describe("sliderStep", () => {
  it("is a round step of about a thousandth of the range", () => {
    expect(sliderStep({ min: -30, max: 30 })).toBeCloseTo(0.01);
    expect(sliderStep({ min: 0, max: 1 })).toBeCloseTo(0.001);
    expect(sliderStep({ min: 0, max: 0 })).toBe(0.001);
  });
});

describe("stages", () => {
  it("starts with lip sync off so the mouth slider stays usable", () => {
    expect(defaultStages()).toEqual({
      physics: true,
      eye_blink: true,
      breath: true,
      lip_sync: false,
      pose: true,
    });
  });

  it("reports which stages the model configures", () => {
    const model = {
      effects: {
        eye_blink: { parameters: ["EyeLOpen"] },
        breath: { parameters: [] },
        lip_sync: { parameters: ["MouthOpenY"] },
      },
      pose: { fade_duration: 0.5, groups: [] },
    } as unknown as Pick<Model, "effects" | "pose">;
    expect(stageAvailability(model, false)).toEqual({
      physics: false,
      eye_blink: true,
      breath: false,
      lip_sync: true,
      pose: false,
    });
    expect(stageAvailability(null, true).physics).toBe(true);
  });
});

describe("pushEvents", () => {
  it("puts the newest first and keeps a few", () => {
    expect(pushEvents(["a"], ["b", "c"])).toEqual(["c", "b", "a"]);
    const many = pushEvents([], ["1", "2", "3", "4", "5", "6", "7"]);
    expect(many).toHaveLength(EVENT_HISTORY);
    expect(many[0]).toBe("7");
  });

  it("returns the same array when nothing fired", () => {
    const history = ["a"];
    expect(pushEvents(history, [])).toBe(history);
  });
});

describe("previewPart", () => {
  it("renames the module's own parts while comparing", () => {
    expect(previewPart("Hat/Brim", null)).toBe("Hat/Brim");
    const own = { name: "Hat", editing: "Hat (editing)" };
    expect(previewPart("Hat/Brim", own)).toBe("Hat (editing)/Brim");
    // 本体のパーツと、名前が前方で重なるだけのパーツはそのまま。
    expect(previewPart("ArmA", own)).toBe("ArmA");
    expect(previewPart("Hats/Brim", own)).toBe("Hats/Brim");
  });
});

describe("modulePoseGroups (T0661)", () => {
  const entry = (part: string) => ({ part, links: [] as string[] });
  const own = [{ entries: [entry("ArmA"), entry("ArmB")] }, { entries: [entry("Hat/BrimA")] }];
  const composed = JSON.stringify({
    fade_duration: 0.5,
    groups: [
      // 本体のグループ (追従のリンクが増えていても同じグループ)
      { entries: [{ part: "ArmA", links: ["Sailor/SleeveA"] }, entry("ArmB")] },
      // 比べている間の今の編集のグループ
      { entries: [entry("Hat (editing)/BrimA")] },
      { entries: [entry("Sailor/CollarA"), entry("Sailor/CollarB")] },
      { entries: [entry("Sailor/TieA")] },
      { entries: [entry("LongHair/Tip")] },
    ],
  });
  it("keeps the groups of the attached modules only", () => {
    const groups = modulePoseGroups(composed, own, { name: "Hat", editing: "Hat (editing)" });
    expect(groups.map((g) => g.module)).toEqual(["Sailor", "Sailor", "LongHair"]);
    expect(groups[0]?.group.entries.map((e) => e.part)).toEqual([
      "Sailor/CollarA",
      "Sailor/CollarB",
    ]);
    expect(modulePoseLabels(groups)).toEqual([
      "Sailor group 1",
      "Sailor group 2",
      "LongHair group 1",
    ]);
    // 比べていなければ今の編集のグループはモジュールの名前のまま。
    expect(modulePoseGroups(composed, own, null).map((g) => g.module)).toEqual([
      "Hat (editing)",
      "Sailor",
      "Sailor",
      "LongHair",
    ]);
  });
  it("is empty without a pose", () => {
    expect(modulePoseGroups(undefined, own, null)).toEqual([]);
    expect(modulePoseGroups("not json", own, null)).toEqual([]);
  });
});

describe("following the visible pose entries (T0736)", () => {
  const entry = (part: string) => ({ part, links: [] as string[] });
  const groups = [
    { entries: [entry("ArmA"), entry("ArmB")] },
    { entries: [entry("Hat/On"), entry("Hat/Off")] },
  ];

  it("reads the visible parts", () => {
    expect(visiblePoseParts('[null,"ArmB"]')).toEqual(new Set(["ArmB"]));
    expect(visiblePoseParts(undefined)).toBeNull();
    expect(visiblePoseParts("{ not json")).toBeNull();
    expect(visiblePoseParts("{}")).toBeNull();
  });

  it("shows the visible entry of each group", () => {
    expect(followShown([], groups, new Set(["ArmB", "Hat/Off"]))).toEqual(["ArmB", "Hat/Off"]);
    expect(followShown(["ArmB", "Hat/Off"], groups, new Set(["ArmA", "Hat/Off"]))).toEqual([
      "Hat/Off",
      "ArmA",
    ]);
  });

  it("maps the parts to the puppet's ids", () => {
    const own = { name: "Hat", editing: "Hat (editing)" };
    const visible = new Set(["ArmA", "Hat (editing)/Off"]);
    expect(followShown([], groups, visible, (part) => previewPart(part, own))).toEqual(["Hat/Off"]);
  });

  it("returns the same array when nothing changes", () => {
    const shown = ["ArmB"];
    // 挙げていないグループは先頭の要素を表示している。表示中の要素の無いグループは前のまま。
    expect(followShown(shown, groups, new Set(["ArmB", "Hat/On"]))).toBe(shown);
    expect(followShown(shown, groups, new Set())).toBe(shown);
  });
});
