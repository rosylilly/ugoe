import { describe, expect, it } from "vitest";
import type { ModuleStatus, Pose } from "../../ipc";
import {
  addEntry,
  addGroup,
  canLinkTo,
  editableLinks,
  fadeSeconds,
  freeParts,
  isBaseGroup,
  moveEntry,
  removeEntry,
  removeGroup,
  setLinks,
  setPoseCommand,
  usedParts,
} from "./poses";

const pose: Pose = {
  fade_duration: 0.5,
  groups: [
    {
      entries: [
        { part: "ArmA", links: ["Hat/SleeveA"] },
        { part: "ArmB", links: [] },
      ],
    },
  ],
};

const module = {
  base_parts: ["ArmA", "ArmB", "Body"],
  base_pose_groups: 1,
  slot: { pose_parts: ["ArmA"] },
} as unknown as ModuleStatus;

describe("poses", () => {
  it("lists the parts in use and the free ones", () => {
    expect([...usedParts(pose)].sort()).toEqual(["ArmA", "ArmB", "Hat/SleeveA"]);
    const parts = [{ id: "ArmA" }, { id: "Body" }, { id: "Hat/SleeveB" }, { id: "Hat/SleeveA" }];
    expect(freeParts(parts, pose, null).map((p) => p.id)).toEqual(["Body", "Hat/SleeveB"]);
    // モジュールのプロジェクトではモジュールのパーツだけ。
    expect(freeParts(parts, pose, module).map((p) => p.id)).toEqual(["Hat/SleeveB"]);
  });

  it("adds, moves and removes entries and groups", () => {
    const added = addGroup(pose, "HatA");
    expect(added.groups[1]).toEqual({ entries: [{ part: "HatA", links: [] }] });
    const two = addEntry(added, 1, "HatB");
    expect(two.groups[1]?.entries.map((e) => e.part)).toEqual(["HatA", "HatB"]);
    expect(moveEntry(two, 1, 1, -1).groups[1]?.entries.map((e) => e.part)).toEqual([
      "HatB",
      "HatA",
    ]);
    expect(moveEntry(two, 1, 0, -1)).toEqual(two);
    // 最後の要素を消すとグループも消える。
    expect(removeEntry(removeEntry(two, 1, 0), 1, 0).groups).toHaveLength(1);
    expect(removeGroup(two, 0).groups[0]?.entries[0]?.part).toBe("HatA");
    expect(setPoseCommand(two)).toEqual({ op: "set_pose", pose: two });
    // もとのポーズは変えない。
    expect(pose.groups).toHaveLength(1);
  });

  it("keeps the base links in a module project", () => {
    const withBase: Pose = {
      ...pose,
      groups: [
        {
          entries: [
            { part: "ArmA", links: ["Body", "Hat/SleeveA"] },
            { part: "ArmB", links: [] },
          ],
        },
      ],
    };
    expect(editableLinks(withBase.groups[0]?.entries[0]?.links ?? [], module)).toEqual([
      "Hat/SleeveA",
    ]);
    const next = setLinks(withBase, 0, 0, ["Hat/SleeveB"], module);
    expect(next.groups[0]?.entries[0]?.links).toEqual(["Body", "Hat/SleeveB"]);
    // 本体のプロジェクトではすべてのリンクを選び直す。
    expect(setLinks(withBase, 0, 0, [], null).groups[0]?.entries[0]?.links).toEqual([]);
  });

  it("marks the base groups and the exposed entries", () => {
    expect(isBaseGroup(module, 0)).toBe(true);
    expect(isBaseGroup(module, 1)).toBe(false);
    expect(isBaseGroup(null, 0)).toBe(false);
    expect(canLinkTo(module, "ArmA")).toBe(true);
    expect(canLinkTo(module, "ArmB")).toBe(false);
    expect(canLinkTo(null, "ArmB")).toBe(true);
  });

  it("reads the fade time", () => {
    expect(fadeSeconds("0.25")).toBe(0.25);
    expect(fadeSeconds("0")).toBe(0);
    expect(fadeSeconds("-1")).toBeNull();
    expect(fadeSeconds("")).toBeNull();
    expect(fadeSeconds("abc")).toBeNull();
  });
});
