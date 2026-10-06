// src/test-setup.ts の前提: 部品のテストは本物の wasm を読まない (T1641。docs/testing.md#エディタの部品のテスト)。
import { describe, expect, it, vi } from "vitest";
import { loadUgoeWasm } from "./lib/viewport/wasm";

describe("test-setup", () => {
  it("replaces loadUgoeWasm with a fake that loads nothing", async () => {
    expect(vi.isMockFunction(loadUgoeWasm)).toBe(true);
    expect(await loadUgoeWasm()).toBeNull();
  });
});
