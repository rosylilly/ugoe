import { describe, expect, it } from "vitest";
import type { Slot } from "../../generated/types";
import { indexModel } from "../tree/build";
import { testModel } from "../tree/testModel";
import {
  drawOrderRows,
  exposeCommand,
  finite,
  freeRange,
  fullSlot,
  isValidSlotId,
  meshesInRange,
  newSlot,
  newSlotId,
  overlapping,
  rangesOverlap,
  resolveSlot,
  setSlotCommand,
  toggled,
} from "./slots";

const slot = (id: string, min: number, max: number, extra: Partial<Slot> = {}): Slot => ({
  id,
  draw_order: { min, max },
  ...extra,
});

describe("rangesOverlap", () => {
  const r = (min: number, max: number) => ({ min, max });
  it("matches the command's rule (touching ends do not overlap)", () => {
    const cases: [ReturnType<typeof r>, ReturnType<typeof r>, boolean][] = [
      [r(0, 10), r(5, 15), true],
      [r(0, 10), r(10, 20), false],
      [r(0, 10), r(0, 10), true],
      [r(5, 5), r(0, 10), true],
      [r(10, 10), r(0, 10), true],
      [r(5, 5), r(6, 6), false],
    ];
    for (const [a, b, expected] of cases) {
      expect(rangesOverlap(a, b)).toBe(expected);
      expect(rangesOverlap(b, a)).toBe(expected);
    }
  });

  it("lists the other slots a slot overlaps", () => {
    const slots = [slot("Hair", 550, 650), slot("Hat", 600, 700), slot("Outfit", 300, 400)];
    expect(overlapping(slots[0] as Slot, slots)).toEqual(["Hat"]);
    expect(overlapping(slots[2] as Slot, slots)).toEqual([]);
  });
});

describe("new slots", () => {
  it("picks an unused id", () => {
    expect(newSlotId([])).toBe("Slot");
    expect(newSlotId([slot("Slot", 0, 1), slot("Slot_2", 2, 3)])).toBe("Slot_3");
  });

  it("checks the id format", () => {
    expect(isValidSlotId("Hair_2")).toBe(true);
    expect(isValidSlotId("")).toBe(false);
    expect(isValidSlotId("Head/Hat")).toBe(false);
    expect(isValidSlotId("帽子")).toBe(false);
  });

  it("takes a free range from the front", () => {
    expect(freeRange([])).toEqual({ min: 950, max: 1000 });
    const slots = [slot("Hair", 550, 650), slot("Outfit", 300, 400)];
    expect(freeRange(slots)).toEqual({ min: 950, max: 1000 });
    expect(freeRange([slot("Top", 960, 1000), slot("Mid", 500, 950)])).toEqual({
      min: 450,
      max: 500,
    });
    // 狭い隙間しかなければ、その幅だけ取る。
    expect(freeRange([slot("A", 20, 1000), slot("B", 0, 10)])).toEqual({ min: 10, max: 20 });
    expect(freeRange([slot("All", 0, 1000)])).toBeNull();
    const created = newSlot(slots);
    expect(created?.id).toBe("Slot");
    expect(created && overlapping(created, slots)).toEqual([]);
  });
});

describe("setSlotCommand", () => {
  it("sends the whole slot with the patch applied", () => {
    const hair = slot("Hair", 550, 650, { anchor: "Head", replaces: ["Hair"] });
    expect(setSlotCommand(hair, { replaces: [] })).toEqual({
      op: "set_slot",
      id: "Hair",
      slot: { ...fullSlot(hair), replaces: [] },
    });
    // 名前を変えるときは、元の ID を指す。
    expect(setSlotCommand(hair, { id: "Hairstyle" })).toMatchObject({
      id: "Hair",
      slot: { id: "Hairstyle" },
    });
  });

  it("returns null when nothing changes", () => {
    const hair = slot("Hair", 550, 650, { anchor: "Head" });
    expect(setSlotCommand(hair, { anchor: "Head", replaces: [] })).toBeNull();
  });

  it("fills omitted fields", () => {
    expect(fullSlot(slot("Hair", 1, 2))).toEqual({
      id: "Hair",
      anchor: null,
      draw_order: { min: 1, max: 2 },
      replaces: [],
      parameters: [],
      mask_sources: [],
      part: null,
      colliders: [],
      pose_parts: [],
    });
  });
});

describe("resolveSlot", () => {
  const slots = [slot("Cap", 100, 200), slot("Hair", 550, 650)];

  it("finds the slot by its ID first", () => {
    expect(resolveSlot(slots, "Hair", new Map([["Hair", "Cap"]]))?.id).toBe("Hair");
  });

  it("follows the renames sent after the ID was taken", () => {
    expect(resolveSlot(slots, "Hat", new Map([["Hat", "Cap"]]))?.id).toBe("Cap");
    expect(
      resolveSlot(
        slots,
        "Hat",
        new Map([
          ["Hat", "Beret"],
          ["Beret", "Cap"],
        ]),
      )?.id,
    ).toBe("Cap");
  });

  it("returns null for a missing slot and stops on a cycle", () => {
    expect(resolveSlot(slots, null, new Map())).toBeNull();
    expect(resolveSlot(slots, "Hat", new Map())).toBeNull();
    expect(
      resolveSlot(
        slots,
        "A",
        new Map([
          ["A", "B"],
          ["B", "A"],
        ]),
      ),
    ).toBeNull();
  });
});

describe("lists", () => {
  it("adds to the end and removes", () => {
    expect(toggled(["A", "B"], "C", true)).toEqual(["A", "B", "C"]);
    expect(toggled(["A", "B"], "A", false)).toEqual(["B"]);
    expect(toggled(["A", "B"], "A", true)).toEqual(["B", "A"]);
  });

  it("changes the avatar profile's slots only when there is a profile", () => {
    expect(exposeCommand(["Hair"], "Outfit", true)).toEqual({
      op: "update_avatar_profile",
      slots: ["Hair", "Outfit"],
    });
    expect(exposeCommand(["Hair"], "Hair", false)).toEqual({
      op: "update_avatar_profile",
      slots: [],
    });
    expect(exposeCommand(["Hair"], "Hair", true)).toBeNull();
    expect(exposeCommand(null, "Hair", true)).toBeNull();
  });

  it("reads finite numbers", () => {
    expect(finite(" 12.5 ")).toBe(12.5);
    expect(finite("")).toBeNull();
    expect(finite("abc")).toBeNull();
  });
});

describe("draw order", () => {
  // メッシュ: EyeL / EyeMask / EyeR (600)、Skin / Ghost (500)。
  const index = indexModel(testModel());

  it("interleaves the slot ranges with the meshes from front to back", () => {
    const rows = drawOrderRows(index, [slot("Hat", 600, 700), slot("Body", 400, 450)]);
    expect(rows.map((r) => (r.kind === "mesh" ? r.id : `${r.id}:${r.edge}`))).toEqual([
      "Hat:top",
      "EyeR",
      "EyeMask",
      "EyeL",
      "Hat:bottom",
      "Ghost",
      "Skin",
      "Body:top",
      "Body:bottom",
    ]);
    const eye = rows.find((r) => r.kind === "mesh" && r.id === "EyeL");
    expect(eye).toMatchObject({ slots: ["Hat"], drawOrder: 600 });
    const skin = rows.find((r) => r.kind === "mesh" && r.id === "Skin");
    expect(skin).toMatchObject({ slots: [] });
  });

  it("selects the meshes inside a range", () => {
    expect(meshesInRange(index, { min: 500, max: 550 })).toEqual(["Ghost", "Skin"]);
    expect(meshesInRange(index, { min: 700, max: 800 })).toEqual([]);
  });
});
