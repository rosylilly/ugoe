import { describe, expect, it } from "vitest";
import { parseHexColor } from "./color";

describe("parseHexColor", () => {
  it("reads long and short forms", () => {
    expect(parseHexColor("#ff8000")).toEqual([1, 128 / 255, 0]);
    expect(parseHexColor("#FFF")).toEqual([1, 1, 1]);
  });

  it("rejects other strings", () => {
    expect(parseHexColor("red")).toBeNull();
    expect(parseHexColor("#12345")).toBeNull();
  });
});
