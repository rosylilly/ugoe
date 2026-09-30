import { describe, expect, it } from "vitest";
import { baseName, detectModelKind } from "./modelFile";

const bytes = (s: string) => new TextEncoder().encode(s);

/** チャンク表だけのコンテナ (中身は無い)。 */
function container(...tags: string[]): Uint8Array {
  const out = new Uint8Array(16 + tags.length * 24);
  out.set(bytes("UGOE"));
  new DataView(out.buffer).setUint32(12, tags.length, true);
  tags.forEach((tag, i) => {
    out.set(bytes(tag), 16 + i * 24);
  });
  return out;
}

describe("detectModelKind", () => {
  it("reads the bundle magic regardless of the name", () => {
    expect(detectModelKind({ name: "x.bin", bytes: bytes("UGOE\0\0\0\0") })).toBe("bundle");
  });

  it("tells modules from bundles by the chunk table", () => {
    expect(detectModelKind({ name: "x.ugoe", bytes: container("MODU") })).toBe("module");
    expect(detectModelKind({ name: "x.ugoemod", bytes: container("FEAT", "MODL") })).toBe("bundle");
    expect(detectModelKind({ name: "x", bytes: container("META", "MODU") })).toBe("module");
  });

  it("falls back to the extension when the chunk table is unreadable", () => {
    expect(detectModelKind({ name: "a.ugoemod", bytes: bytes("UGOE") })).toBe("module");
    expect(detectModelKind({ name: "a.bin", bytes: bytes("UGOE") })).toBe("bundle");
    expect(detectModelKind({ name: "a.UGOEMOD", bytes: bytes("xx") })).toBe("module");
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
