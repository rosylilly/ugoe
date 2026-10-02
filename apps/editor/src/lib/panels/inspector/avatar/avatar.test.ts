import { describe, expect, it } from "vitest";
import type { Message } from "../../../generated/types";
import {
  checkSummary,
  createCommand,
  defaultMeta,
  hitAreaCommand,
  metaCommand,
  optionalText,
  parseLines,
} from "./avatar";

describe("parseLines", () => {
  it("trims each line and drops empty ones", () => {
    expect(parseLines(" Alice \n\n  Bob\n ")).toEqual(["Alice", "Bob"]);
    expect(parseLines("")).toEqual([]);
  });
});

describe("optionalText", () => {
  it("turns blank text into null", () => {
    expect(optionalText("   ")).toBeNull();
    expect(optionalText(" 1.0 ")).toBe("1.0");
  });
});

describe("defaultMeta", () => {
  it("starts every permission at the most restrictive value", () => {
    const meta = defaultMeta();
    expect(meta.avatar_permission).toBe("only_author");
    expect(meta.commercial_usage).toBe("personal_non_profit");
    expect(meta.credit_notation).toBe("required");
    expect(meta.modification).toBe("prohibited");
    expect(meta.allow_redistribution).toBe(false);
    expect(meta.allow_violent_usage).toBe(false);
    expect(meta.allow_sexual_usage).toBe(false);
    expect(meta.allow_political_or_religious_usage).toBe(false);
    expect(meta.allow_antisocial_or_hate_usage).toBe(false);
  });
});

describe("createCommand", () => {
  it("fills the name from the model and leaves permissions at the defaults", () => {
    expect(createCommand(" Hiyori ")).toEqual({
      op: "update_avatar_profile",
      meta: { ...defaultMeta(), name: "Hiyori" },
    });
  });

  it("leaves everything to the command defaults without a model name", () => {
    expect(createCommand("")).toEqual({ op: "update_avatar_profile" });
  });
});

describe("metaCommand", () => {
  const meta = { ...defaultMeta(), name: "A", authors: ["x"] };

  it("replaces the whole metadata with the patch applied", () => {
    expect(metaCommand(meta, { commercial_usage: "corporation" })).toEqual({
      op: "update_avatar_profile",
      meta: { ...meta, commercial_usage: "corporation" },
    });
  });

  it("returns null when nothing changes", () => {
    expect(metaCommand(meta, { name: "A" })).toBeNull();
    expect(metaCommand(meta, { authors: ["x"] })).toBeNull();
  });
});

describe("hitAreaCommand", () => {
  it("keeps the standard names first", () => {
    const current = [{ name: "Tail", hit_area: "HitTail" }];
    expect(hitAreaCommand(current, "Body", "HitBody")).toEqual({
      op: "update_avatar_profile",
      hit_areas: [
        { name: "Body", hit_area: "HitBody" },
        { name: "Tail", hit_area: "HitTail" },
      ],
    });
  });

  it("replaces and removes a mapping", () => {
    const current = [
      { name: "Head", hit_area: "HitHead" },
      { name: "Body", hit_area: "HitBody" },
    ];
    expect(hitAreaCommand(current, "Head", "HitFace")).toEqual({
      op: "update_avatar_profile",
      hit_areas: [
        { name: "Head", hit_area: "HitFace" },
        { name: "Body", hit_area: "HitBody" },
      ],
    });
    expect(hitAreaCommand(current, "Head", null)).toEqual({
      op: "update_avatar_profile",
      hit_areas: [{ name: "Body", hit_area: "HitBody" }],
    });
  });

  it("returns null when nothing changes", () => {
    expect(hitAreaCommand([], "Head", null)).toBeNull();
    expect(hitAreaCommand([{ name: "Head", hit_area: "H" }], "Head", "H")).toBeNull();
  });
});

/** 検査の問題 (`avatar.*` のコードの Message)。 */
function issue(message: string): Message {
  return { code: "avatar.missing_profile", args: {}, message };
}

describe("checkSummary", () => {
  it("reports a model without a profile", () => {
    expect(checkSummary({ claimed: null, satisfied: 2, errors: [issue("no profile")] })).toEqual({
      ok: false,
      text: "No avatar profile: the model does not conform to any level. Satisfies level 2 (Standard).",
    });
  });

  it("reports conformance", () => {
    expect(checkSummary({ claimed: 2, satisfied: 3, errors: [] })).toEqual({
      ok: true,
      text: "Conforms to level 2 (Standard). Satisfies level 3 (Expressive).",
    });
  });

  it("reports errors and the satisfied level", () => {
    expect(checkSummary({ claimed: 3, satisfied: 0, errors: [issue("a"), issue("b")] })).toEqual({
      ok: false,
      text: "Claims level 3 (Expressive) but has 2 errors. Satisfies no level.",
    });
  });
});
