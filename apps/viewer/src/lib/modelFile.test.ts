import { describe, expect, it } from "vitest";
import { baseName, detectModelKind } from "./modelFile";

const bytes = (s: string) => new TextEncoder().encode(s);

describe("detectModelKind", () => {
  it("reads the bundle magic regardless of the name", () => {
    expect(detectModelKind({ name: "x.bin", bytes: bytes("UGOE\0\0\0\0") })).toBe("bundle");
  });

  it("detects JSON by its first non-space byte", () => {
    expect(detectModelKind({ name: "model", bytes: bytes('\ufeff \n{"a":1}') })).toBe("json");
  });

  it("falls back to the extension", () => {
    expect(detectModelKind({ name: "A.UGOE", bytes: bytes("xx") })).toBe("bundle");
    expect(detectModelKind({ name: "m.json", bytes: bytes("[]") })).toBe("json");
    expect(detectModelKind({ name: "c.psd", bytes: bytes("8BPS") })).toBeNull();
  });
});

describe("baseName", () => {
  it("handles both separators", () => {
    expect(baseName("/a/b/c.ugoe")).toBe("c.ugoe");
    expect(baseName("C:\\m\\d.json")).toBe("d.json");
    expect(baseName("e.ugoe")).toBe("e.ugoe");
  });
});
