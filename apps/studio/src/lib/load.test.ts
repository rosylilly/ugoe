import { describe, expect, it } from "vitest";
import type { RendererLoad, StatusMessage } from "./host";
import { loadWarnings, periodMs, rendererGpu } from "./load";

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
    trackingWait: null,
    renderers: [],
    ...patch,
  };
}

function renderer(patch: Partial<RendererLoad> = {}): RendererLoad {
  return {
    label: "output",
    fps: 60,
    refreshFps: 60,
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
    // トラッキングの待ちは目安で、刻みに遅れた値が多くても注意しない (T0863。遅れは刻みのスレッドが自分で減らす。T0864)。
    expect(loadWarnings(status({ trackingWait: { meanMs: 12, maxMs: 30, late: 0.5 } }))).toEqual(
      [],
    );
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
      status({ renderers: [renderer({ renderMs: 12, fps: 40, refreshFps: 40, dropped: 3 })] }),
    ).map((w) => w.kind);
    expect(kinds).toEqual(["renderHeavy", "renderSlow", "dropped"]);
  });

  it("judge an output by its chances to draw, not by the draws (T0857)", () => {
    // 変わったときだけ描くので、144 Hz の画面で刻み 30 なら描くのは約 30 回。フレームが 1 回の更新に重なって減っても注意しない。
    const output = renderer({ fps: 27, refreshFps: 144 });
    expect(loadWarnings(status({ tickRate: 30, renderers: [output] }))).toEqual([]);
    expect(loadWarnings(status({ tickRate: 60, renderers: [{ ...output, fps: 52 }] }))).toEqual([]);
    // 画面の更新が刻みの速さより少ない (60 Hz の画面で刻み 120) ときは、今までどおり注意する。
    const slow = loadWarnings(
      status({
        tickRate: 120,
        measuredRate: 120,
        maxIntervalMs: 9,
        renderers: [renderer({ fps: 60, refreshFps: 60 })],
      }),
    );
    expect(slow).toEqual([{ kind: "renderSlow", label: "output", fps: 60, target: 120 }]);
  });

  it("judge the virtual camera by its own frame rate", () => {
    const camera = renderer({
      label: "virtualCamera",
      targetFps: 30,
      fps: 30,
      refreshFps: null,
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

describe("renderer GPU", () => {
  it("names the API and the GPU of a receiver", () => {
    expect(rendererGpu(renderer({ backend: "webgpu", adapter: "Intel(R) UHD" }))).toEqual({
      backend: "WebGPU",
      adapter: "Intel(R) UHD",
    });
    expect(rendererGpu(renderer({ backend: "dx12", adapter: "NVIDIA GeForce" }))).toEqual({
      backend: "Direct3D 12",
      adapter: "NVIDIA GeForce",
    });
    // 知らない API はそのまま
    expect(rendererGpu(renderer({ backend: "noop", adapter: "x" }))?.backend).toBe("noop");
  });

  it("hides a GPU name the browser does not tell", () => {
    expect(rendererGpu(renderer({ backend: "webgl2", adapter: "" }))).toEqual({
      backend: "WebGL2",
      adapter: null,
    });
    expect(rendererGpu(renderer({ backend: "webgl2", adapter: null }))?.adapter).toBeNull();
  });

  it("shows nothing without the API", () => {
    expect(rendererGpu(renderer())).toBeNull();
    expect(rendererGpu(renderer({ backend: null, adapter: "x" }))).toBeNull();
  });
});
