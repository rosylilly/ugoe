// `createPuppet` が `WebPuppet.create` に渡す引数。wasm は使わず、`./wasm/ugoe_wasm.js` を差し替える。

import { describe, expect, it, vi } from "vitest";

const wasm = vi.hoisted(() => ({
  creates: [] as unknown[][],
}));

vi.mock("./wasm/ugoe_wasm.js", () => ({
  default: vi.fn(async () => ({})),
  sample_motion_json: vi.fn(),
  WebPuppet: {
    create: vi.fn(async (...args: unknown[]) => {
      wasm.creates.push(args);
      return {};
    }),
  },
}));

const { createPuppet } = await import("./runtime.js");

describe("createPuppet", () => {
  it("passes the backend and the power preference to WebPuppet.create (T1401)", async () => {
    const canvas = {} as HTMLCanvasElement;
    await createPuppet(canvas);
    await createPuppet(canvas, { backend: "webgl2", powerPreference: "low-power" });
    await createPuppet(canvas, { powerPreference: "high-performance" });
    expect(wasm.creates).toEqual([
      [canvas, "auto", "default"],
      [canvas, "webgl2", "low-power"],
      [canvas, "auto", "high-performance"],
    ]);
  });
});
