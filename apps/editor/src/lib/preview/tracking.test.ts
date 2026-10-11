// プレビューモードのフェイストラッキングの受け口 (PreviewMode の TrackingTarget、T0636)。WebPuppet とビューポートは偽物。
import { DEFAULT_TUNING, type TrackingTuning } from "@ugoe/face-tracking";
import { MemoryStorage } from "@ugoe/i18n/testing";
import { beforeAll, describe, expect, it, vi } from "vitest";
import type { EditorDocument } from "../document.svelte";
import { STORAGE_KEYS } from "../storageKeys";
import type { FrameDriver, ViewportController } from "../viewport/controller.svelte";
import type { WebPuppet } from "../viewport/wasm";
import type { PreviewMode as PreviewModeType } from "./preview.svelte";

let PreviewMode: typeof PreviewModeType;

// @ugoe/face-tracking の TrackingPanel.svelte が読む部品の集まり。ここでは描かないので空にする (読み込むと部品を
// すべて変換するので数秒かかり、負荷が高いと beforeAll が時間切れになる)。
vi.mock("@ugoe/ui", () => ({}));

beforeAll(async () => {
  // PreviewMode はヘッドレスのブラウザでの確認のため window にトラッカーを置く。
  (globalThis as { window?: unknown }).window ??= {};
  // 止めるときに FaceTracker が呼ぶ。
  globalThis.cancelAnimationFrame ??= () => {};
  ({ PreviewMode } = await import("./preview.svelte"));
});

/** 呼ばれた操作を記録する WebPuppet の偽物。 */
function fakePuppet(loadWarnings: unknown[] = []) {
  const calls: string[] = [];
  const values: Record<string, number> = { AngleX: 0, EyeLOpen: 1, Other: 0.5 };
  const puppet = {
    has_model: () => true,
    is_lost: () => false,
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
    load_warnings_json: () => JSON.stringify(loadWarnings),
    attached_modules: () => [],
  };
  return { puppet: puppet as unknown as WebPuppet, calls, values };
}

function setup(loadWarnings: unknown[] = [], storage: Storage = new MemoryStorage()) {
  const { puppet, calls, values } = fakePuppet(loadWarnings);
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
  const preview = new PreviewMode(doc, viewport, storage);
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

describe("PreviewMode load warnings", () => {
  it("shows the warnings of the last load in English, replacing the previous ones", () => {
    const { preview, puppet } = setup([
      { code: "texture_missing", warning: "no data for texture 1", texture: 1 },
      { code: "module_detached", warning: "module `Dress` was detached", module: "Dress" },
    ]);
    // 外したモジュールは着せ替えの知らせに出るので、ここには出さない。
    expect(preview.warnings).toEqual(["Texture 1 has no image, so it is drawn white."]);
    preview.loaded(puppet);
    expect(preview.warnings).toHaveLength(1);
  });
});

describe("PreviewMode remembers the tracking tuning", () => {
  const KEY = STORAGE_KEYS.trackingTuning;
  const TUNED: TrackingTuning = {
    ...DEFAULT_TUNING,
    yaw: { ...DEFAULT_TUNING.yaw, gain: 1.5, deadZone: 2 },
    jawOpen: { low: 0.1, high: 0.9 },
  };

  it("starts from the default when nothing is remembered", () => {
    const { preview } = setup([], new MemoryStorage());
    expect(preview.tracker.tuning).toEqual(DEFAULT_TUNING);
  });

  it("saves the tuning and reads it back when made again", () => {
    const storage = new MemoryStorage();
    const { preview } = setup([], storage);
    preview.tracker.setTuning(TUNED);
    preview.saveTuning();
    expect(storage.keys()).toEqual([KEY]);
    expect(setup([], storage).preview.tracker.tuning).toEqual(TUNED);
  });

  it("forgets the key when set back to the default", () => {
    const storage = new MemoryStorage();
    const { preview } = setup([], storage);
    preview.tracker.setTuning(TUNED);
    preview.saveTuning();
    preview.tracker.setTuning(DEFAULT_TUNING);
    preview.saveTuning();
    expect(storage.keys()).toEqual([]);
  });

  it("reads broken or out-of-range values as the default", () => {
    const storage = new MemoryStorage();
    storage.setItem(KEY, "{");
    expect(setup([], storage).preview.tracker.tuning).toEqual(DEFAULT_TUNING);
    storage.setItem(KEY, JSON.stringify({ yaw: { gain: 99, deadZone: -5 } }));
    expect(setup([], storage).preview.tracker.tuning).toEqual({
      ...DEFAULT_TUNING,
      yaw: { ...DEFAULT_TUNING.yaw, gain: 4 },
    });
  });
});
