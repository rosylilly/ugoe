// プレビューモードのフェイストラッキングの受け口 (PreviewMode の TrackingTarget、T0636)。WebPuppet とビューポートは偽物。
import { beforeAll, describe, expect, it } from "vitest";
import type { EditorDocument } from "../document.svelte";
import type { FrameDriver, ViewportController } from "../viewport/controller.svelte";
import type { WebPuppet } from "../viewport/wasm";
import type { PreviewMode as PreviewModeType } from "./preview.svelte";

let PreviewMode: typeof PreviewModeType;

beforeAll(async () => {
  // PreviewMode はヘッドレスのブラウザでの確認のため window にトラッカーを置く。
  (globalThis as { window?: unknown }).window ??= {};
  // 止めるときに FaceTracker が呼ぶ。
  globalThis.cancelAnimationFrame ??= () => {};
  ({ PreviewMode } = await import("./preview.svelte"));
});

/** 呼ばれた操作を記録する WebPuppet の偽物。 */
function fakePuppet() {
  const calls: string[] = [];
  const values: Record<string, number> = { AngleX: 0, EyeLOpen: 1, Other: 0.5 };
  const puppet = {
    has_model: () => true,
    has_physics: () => true,
    set_parameter(id: string, value: number) {
      values[id] = value;
      return true;
    },
    parameter: (id: string) => values[id],
    set_expression_json() {},
    stop_expressions() {},
    set_lip_sync_level() {},
    set_enabled(stage: string, enabled: boolean) {
      calls.push(`set_enabled ${stage} ${enabled}`);
    },
    tracking_parameter_ids: () => ["AngleX", "EyeLOpen"],
    set_tracking(names: string[], scores: Float32Array, yaw: number) {
      calls.push(`set_tracking ${names.join(",")} ${Array.from(scores).join(",")} ${yaw}`);
      values.AngleX = yaw;
    },
    clear_tracking() {
      calls.push("clear_tracking");
    },
    modules_json: () => "[]",
    detached_modules_json: () => "[]",
    attached_modules: () => [],
  };
  return { puppet: puppet as unknown as WebPuppet, calls, values };
}

function setup() {
  const { puppet, calls, values } = fakePuppet();
  let driver: FrameDriver | null = null;
  const viewport = {
    puppet,
    parameterValues: {},
    setDriver(d: FrameDriver | null) {
      driver = d;
      d?.loaded(puppet);
    },
    applyStages() {
      for (const [stage, enabled] of Object.entries(driver?.stages ?? {})) {
        puppet.set_enabled(stage, enabled);
      }
    },
  } as unknown as ViewportController;
  const doc = {
    parameters: [
      { id: "AngleX", default: 0 },
      { id: "EyeLOpen", default: 1 },
      { id: "Other", default: 0 },
    ],
    expressions: [],
    physics: null,
  } as unknown as EditorDocument;
  const preview = new PreviewMode(doc, viewport);
  preview.enter();
  calls.length = 0;
  return { preview, puppet, calls, values };
}

const frame = { blendShapes: { eyeBlinkLeft: 0.25 }, head: { yaw: 10, pitch: 0, roll: 0 } };

describe("PreviewMode as a tracking target", () => {
  it("reads the tracked parameters when the model loads", () => {
    const { preview } = setup();
    expect(preview.trackingParameterIds).toEqual(["AngleX", "EyeLOpen"]);
  });

  it("passes the latest frame once, right before the update", () => {
    const { preview, puppet, calls } = setup();
    preview.setTracking(frame);
    expect(preview.tracking).toBe(true);
    expect(calls).toEqual([]);
    preview.beforeUpdate(puppet);
    preview.beforeUpdate(puppet);
    expect(calls).toEqual(["set_tracking eyeBlinkLeft 0.25 10"]);
  });

  it("passes the last frame again after the model is reloaded", () => {
    const { preview, puppet, calls } = setup();
    preview.setTracking(frame);
    preview.beforeUpdate(puppet);
    preview.loaded(puppet);
    preview.beforeUpdate(puppet);
    expect(calls.filter((c) => c.startsWith("set_tracking"))).toHaveLength(2);
  });

  it("clears the tracking and puts the tracked parameters back to their defaults", () => {
    const { preview, puppet, calls, values } = setup();
    preview.setTracking(frame);
    preview.beforeUpdate(puppet);
    values.EyeLOpen = 0.2;
    values.Other = 0.7;
    preview.setTracking(null);
    expect(calls).toContain("clear_tracking");
    expect(preview.tracking).toBe(false);
    expect(values).toEqual({ AngleX: 0, EyeLOpen: 1, Other: 0.7 });
    expect(preview.values.EyeLOpen).toBe(1);
  });

  it("stops the eye blink stage only while the tracker asks, and keeps the stage switch", () => {
    const { preview, calls } = setup();
    preview.setAutoEyeBlink(false);
    expect(calls.at(-1)).toBe("set_enabled eye_blink false");
    // 段を切り替えても、トラッキング中は目パチを止めたまま。
    preview.setStage("eye_blink", true);
    expect(calls.at(-1)).toBe("set_enabled eye_blink false");
    preview.setAutoEyeBlink(true);
    expect(calls.at(-1)).toBe("set_enabled eye_blink true");
    preview.setStage("eye_blink", false);
    preview.setAutoEyeBlink(false);
    preview.setAutoEyeBlink(true);
    expect(calls.at(-1)).toBe("set_enabled eye_blink false");
  });

  it("stops the tracker when leaving the preview", () => {
    const { preview } = setup();
    preview.setTracking(frame);
    preview.exit();
    expect(preview.tracking).toBe(false);
    expect(preview.tracker.status).toBe("idle");
  });
});
