import { describe, expect, it } from "vitest";
import {
  carryPoseShown,
  type PoseGroupInfo,
  parsePose,
  poseEntryLabel,
  poseGroupNumbers,
} from "./pose";

const pose = (groups: string[][]) =>
  JSON.stringify({
    fade_duration: 0.5,
    groups: groups.map((g) => ({ entries: g.map((part) => ({ part, links: [] })) })),
  });

describe("parsePose", () => {
  it("reads the entries of each group and the module of module groups", () => {
    expect(
      parsePose(
        pose([
          ["earwear", "no_earwear"],
          ["Hat/On", "Hat/Off"],
        ]),
      ),
    ).toEqual([
      { parts: ["earwear", "no_earwear"], module: null },
      { parts: ["Hat/On", "Hat/Off"], module: "Hat" },
    ]);
  });

  it("skips empty groups and malformed entries", () => {
    const json = JSON.stringify({
      groups: [{ entries: [] }, { entries: [{ part: 1 }, null, { part: "A" }] }, null, {}],
    });
    expect(parsePose(json)).toEqual([{ parts: ["A"], module: null }]);
    expect(parsePose(undefined)).toEqual([]);
    expect(parsePose(null)).toEqual([]);
    expect(parsePose("[]")).toEqual([]);
    expect(parsePose("null")).toEqual([]);
  });
});

describe("carryPoseShown", () => {
  const arms: PoseGroupInfo = { parts: ["ArmA", "ArmB"], module: null };
  const hat: PoseGroupInfo = { parts: ["Hat/On", "Hat/Off"], module: "Hat" };

  it("keeps the choice of groups with the same entries and shows the first entry of new ones", () => {
    expect(carryPoseShown([arms, hat], [arms], ["ArmB"])).toEqual(["ArmB", "Hat/On"]);
    expect(carryPoseShown([hat, arms], [arms, hat], ["ArmB", "Hat/Off"])).toEqual([
      "Hat/Off",
      "ArmB",
    ]);
    expect(carryPoseShown([arms], [arms, hat], ["ArmA", "Hat/Off"])).toEqual(["ArmA"]);
  });

  it("starts over when the entries change", () => {
    const changed: PoseGroupInfo = { parts: ["ArmA", "ArmB", "ArmC"], module: null };
    expect(carryPoseShown([changed], [arms], ["ArmB"])).toEqual(["ArmA"]);
    expect(carryPoseShown([arms], [], [])).toEqual(["ArmA"]);
  });
});

describe("labels", () => {
  it("drops the module prefix from entries of module groups", () => {
    const hat: PoseGroupInfo = { parts: ["Hat/On"], module: "Hat" };
    expect(poseEntryLabel(hat, "Hat/On")).toBe("On");
    expect(poseEntryLabel({ parts: ["a/b"], module: null }, "a/b")).toBe("a/b");
  });

  it("numbers the groups of the model and of each module separately", () => {
    const groups: PoseGroupInfo[] = [
      { parts: ["A"], module: null },
      { parts: ["B"], module: null },
      { parts: ["Hat/On"], module: "Hat" },
      { parts: ["Bag/On"], module: "Bag" },
      { parts: ["Hat/Brim"], module: "Hat" },
    ];
    expect(poseGroupNumbers(groups)).toEqual([1, 2, 1, 1, 2]);
  });
});
