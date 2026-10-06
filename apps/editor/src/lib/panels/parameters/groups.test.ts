import { describe, expect, it } from "vitest";
import type { Parameter, ParameterGroup } from "../../generated/types";
import {
  addGroupCommand,
  dropGroupCommand,
  dropIndexAt,
  groupOptions,
  memberCount,
  moveGroupCommand,
  setGroupCommand,
} from "./groups";

const group = (id: string, name = id): ParameterGroup => ({ id, name });

const param = (id: string, g: string | null): Parameter => ({
  id,
  name: id,
  min: 0,
  max: 1,
  default: 0,
  kind: "normal",
  mirror: null,
  group: g,
});

describe("parameter groups", () => {
  it("offers none and the groups in order", () => {
    expect(groupOptions([group("Face", "顔"), group("Body")])).toEqual([
      { value: "", label: "(none)" },
      { value: "Face", label: "顔 (Face)" },
      { value: "Body", label: "Body" },
    ]);
  });

  it("builds the assignment command", () => {
    expect(setGroupCommand("AngleX", "Face")).toEqual({
      op: "set_parameter_group",
      id: "AngleX",
      group: "Face",
    });
    expect(setGroupCommand("AngleX", "")).toEqual({
      op: "set_parameter_group",
      id: "AngleX",
      group: null,
    });
  });

  it("builds the add command, naming the group after its ID by default", () => {
    expect(addGroupCommand(" Face ", "")).toEqual({
      op: "add_parameter_group",
      group: { id: "Face", name: "Face" },
    });
    expect(addGroupCommand("Face", " 顔 ")).toEqual({
      op: "add_parameter_group",
      group: { id: "Face", name: "顔" },
    });
    expect(addGroupCommand("  ", "顔")).toBeNull();
  });

  it("moves a group up or down by one", () => {
    const groups = [group("A"), group("B"), group("C")];
    expect(moveGroupCommand(groups, "B", -1)).toEqual({
      op: "reorder_parameter_group",
      id: "B",
      before: "A",
    });
    expect(moveGroupCommand(groups, "A", 1)).toEqual({
      op: "reorder_parameter_group",
      id: "A",
      before: "C",
    });
    // 最後の 1 つ前を下げると末尾へ。
    expect(moveGroupCommand(groups, "B", 1)).toEqual({
      op: "reorder_parameter_group",
      id: "B",
      before: null,
    });
    expect(moveGroupCommand(groups, "A", -1)).toBeNull();
    expect(moveGroupCommand(groups, "C", 1)).toBeNull();
    expect(moveGroupCommand(groups, "Nope", 1)).toBeNull();
  });

  it("counts the members", () => {
    const parameters = [param("X", "Face"), param("Y", "Face"), param("Z", null)];
    expect(memberCount(parameters, "Face")).toBe(2);
    expect(memberCount(parameters, "Body")).toBe(0);
  });

  it("drops a group before another one or at the end, and not where it already is", () => {
    const groups = [group("A"), group("B"), group("C")];
    expect(dropGroupCommand(groups, "C", 0)).toEqual({
      op: "reorder_parameter_group",
      id: "C",
      before: "A",
    });
    expect(dropGroupCommand(groups, "A", 3)).toEqual({
      op: "reorder_parameter_group",
      id: "A",
      before: null,
    });
    expect(dropGroupCommand(groups, "A", 2)).toEqual({
      op: "reorder_parameter_group",
      id: "A",
      before: "C",
    });
    // 自分の前と後ろは今の位置
    expect(dropGroupCommand(groups, "B", 1)).toBeNull();
    expect(dropGroupCommand(groups, "B", 2)).toBeNull();
    expect(dropGroupCommand(groups, "Missing", 0)).toBeNull();
  });

  it("finds the drop index from the rows' middles", () => {
    const rows = [
      { top: 0, height: 20 },
      { top: 20, height: 40 },
    ];
    expect(dropIndexAt(rows, -5)).toBe(0);
    expect(dropIndexAt(rows, 9)).toBe(0);
    expect(dropIndexAt(rows, 11)).toBe(1);
    expect(dropIndexAt(rows, 39)).toBe(1);
    expect(dropIndexAt(rows, 41)).toBe(2);
    expect(dropIndexAt([], 0)).toBe(0);
  });
});
