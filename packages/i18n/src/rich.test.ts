import { describe, expect, it } from "vitest";
import { parseRich } from "./rich";

describe("parseRich", () => {
  it("splits tags from text", () => {
    expect(parseRich("See <link>the manual</link> for details")).toEqual([
      { kind: "text", text: "See " },
      { kind: "tag", name: "link", text: "the manual" },
      { kind: "text", text: " for details" },
    ]);
  });

  it("keeps unclosed tags as text", () => {
    expect(parseRich("a <b>c")).toEqual([{ kind: "text", text: "a <b>c" }]);
  });
});
