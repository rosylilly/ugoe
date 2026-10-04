import { describe, expect, it } from "vitest";
import {
  formatParameters,
  isBundle,
  parseBackend,
  parseCrossOrigin,
  parseIdList,
  parseMotionPriority,
  parseParameters,
} from "./attributes.js";

describe("parseCrossOrigin", () => {
  it("sends credentials to other origins only for use-credentials", () => {
    expect(parseCrossOrigin(" Use-Credentials ")).toBe("include");
    expect(parseCrossOrigin("anonymous")).toBe("same-origin");
    expect(parseCrossOrigin("")).toBe("same-origin");
    expect(parseCrossOrigin("other")).toBe("same-origin");
    expect(parseCrossOrigin(null)).toBe("same-origin");
  });
});

describe("parseIdList", () => {
  it("splits ids on spaces, commas and semicolons and drops duplicates", () => {
    expect(parseIdList(" EyeLOpen, EyeROpen;EyeLOpen\n MouthOpenY ")).toEqual([
      "EyeLOpen",
      "EyeROpen",
      "MouthOpenY",
    ]);
    expect(parseIdList("")).toEqual([]);
    expect(parseIdList(null)).toEqual([]);
  });
});

describe("parseParameters", () => {
  it("reads id and value pairs separated by semicolons or newlines", () => {
    const params = parseParameters("AngleX: 15; AngleY=-10\n EyeLOpen : 0.5 ;");
    expect([...params]).toEqual([
      ["AngleX", 15],
      ["AngleY", -10],
      ["EyeLOpen", 0.5],
    ]);
  });

  it("keeps the case of the ids and uses the last duplicate", () => {
    expect([...parseParameters("angleX: 1; AngleX: 2; AngleX: 3")]).toEqual([
      ["angleX", 1],
      ["AngleX", 3],
    ]);
  });

  it("ignores malformed entries", () => {
    expect([...parseParameters("AngleX; : 3; AngleY: abc; Z: 1e1; W: Infinity")]).toEqual([
      ["Z", 10],
    ]);
    expect(parseParameters(null).size).toBe(0);
    expect(parseParameters("").size).toBe(0);
  });

  it("round-trips through formatParameters", () => {
    const text = formatParameters([
      ["AngleX", 15],
      ["MouthOpen", 0.25],
    ]);
    expect(text).toBe("AngleX: 15; MouthOpen: 0.25");
    expect([...parseParameters(text)]).toEqual([
      ["AngleX", 15],
      ["MouthOpen", 0.25],
    ]);
  });
});

describe("parseBackend", () => {
  it("accepts webgpu and webgl2 and falls back to auto", () => {
    expect(parseBackend("webgpu")).toBe("webgpu");
    expect(parseBackend(" WebGL2 ")).toBe("webgl2");
    expect(parseBackend("webgl")).toBe("auto");
    expect(parseBackend(null)).toBe("auto");
  });
});

describe("parseMotionPriority", () => {
  it("defaults to idle", () => {
    expect(parseMotionPriority(null)).toBe("idle");
    expect(parseMotionPriority("force")).toBe("force");
    expect(parseMotionPriority("NORMAL")).toBe("normal");
    expect(parseMotionPriority("high")).toBe("idle");
  });
});

describe("isBundle", () => {
  it("detects the UGOE magic", () => {
    expect(isBundle(new TextEncoder().encode("UGOE\u0001\u0000"))).toBe(true);
    expect(isBundle(new TextEncoder().encode('{"schema_version":1}'))).toBe(false);
    expect(isBundle(new Uint8Array([0x55, 0x47]))).toBe(false);
  });
});
