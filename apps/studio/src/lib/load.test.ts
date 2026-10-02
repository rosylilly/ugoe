import { describe, expect, it } from "vitest";
import type { RendererLoad, StatusMessage } from "./host";
import { loadWarnings, periodMs } from "./load";

function status(patch: Partial<StatusMessage> = {}): StatusMessage {
  return {
    kind: "status",
    seq: 1,
    values: [],
    tickMs: 0.2,
    outputs: 1,
    tickRate: 60,
    measuredRate: 60,
    maxIntervalMs: 17,
    renderers: [],
    ...patch,
  };
}

function renderer(patch: Partial<RendererLoad> = {}): RendererLoad {
  return {
    label: "output",
    fps: 60,
    renderMs: 1,
    renderMaxMs: 2,
    dropped: 0,
    stale: false,
    targetFps: null,
    ...patch,
  };
}

describe("load warnings", () => {
  it("are empty when everything keeps up", () => {
    expect(loadWarnings(status({ renderers: [renderer()] }))).toEqual([]);
    // 測り終える前は速さと揺れを判断しない。
    expect(loadWarnings(status({ measuredRate: null, maxIntervalMs: null }))).toEqual([]);
  });

  it("flag a slow or jittery tick", () => {
    expect(loadWarnings(status({ measuredRate: 50 })).map((w) => w.kind)).toEqual([
      "rateShortfall",
    ]);
    expect(loadWarnings(status({ maxIntervalMs: 40 })).map((w) => w.kind)).toEqual([
      "intervalSpike",
    ]);
    // 120 刻みの間隔は約 8.3 ms。5 ms の刻みは重い。
    expect(periodMs(120)).toBeCloseTo(8.333, 3);
    expect(
      loadWarnings(status({ tickRate: 120, measuredRate: 120, maxIntervalMs: 9, tickMs: 5 })).map(
        (w) => w.kind,
      ),
    ).toEqual(["tickHeavy"]);
  });

  it("flag heavy, slow and dropping renderers", () => {
    const kinds = loadWarnings(
      status({ renderers: [renderer({ renderMs: 12, fps: 40, dropped: 3 })] }),
    ).map((w) => w.kind);
    expect(kinds).toEqual(["renderHeavy", "renderSlow", "dropped"]);
  });

  it("judge the virtual camera by its own frame rate", () => {
    const camera = renderer({
      label: "virtualCamera",
      targetFps: 30,
      fps: 30,
      renderMs: 10,
      dropped: 2,
    });
    expect(loadWarnings(status({ renderers: [camera] }))).toEqual([]);
    expect(
      loadWarnings(status({ renderers: [{ ...camera, fps: 20, renderMs: 20 }] })).map(
        (w) => w.kind,
      ),
    ).toEqual(["renderHeavy", "renderSlow"]);
  });

  it("report a stale renderer without judging its old numbers", () => {
    expect(loadWarnings(status({ renderers: [renderer({ stale: true, fps: 0 })] }))).toEqual([
      { kind: "stale", label: "output" },
    ]);
  });
});
