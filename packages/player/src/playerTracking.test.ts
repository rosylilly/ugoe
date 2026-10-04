// Player のドライバとトラッキング (動かされているパラメータ・モーションとの重なり。T0931)。wasm と DOM は偽物 (src/testing/)。
import { describe, expect, it, vi } from "vitest";
import { tracked } from "./testing/faceTracking";
import { bundle, type FakeOptions } from "./testing/fakePuppet";
import { ready, setupPlayer, step } from "./testing/player";

// 画面の部品と MediaPipe まで読み込むと遅いので、Player が使う関数だけにする。
vi.mock("@ugoe/face-tracking", () => import("./testing/faceTracking"));

setupPlayer();

describe("Player drivers and tracking", () => {
  const model = bundle("a.ugoe", 1);
  const options: FakeOptions = {
    parameters: [{ id: "Driven" }, { id: "Tracked", default: 0.25 }, { id: "Moved" }],
    trackingIds: ["Tracked"],
    motions: [{ name: "wave", parameters: ["Moved"] }],
  };
  const face = { head: { yaw: 0, pitch: 0, roll: 0 }, blendShapes: {} };

  it("marks the parameters set by drivers, tracking and the playing motion as driven", async () => {
    const { p, state, log } = await ready(options);
    await p.load(model);
    const remove = p.drivers.add({ name: "test", frame: () => [["Driven", 0.5]] });
    step(0);
    expect(state.values.get("Driven")).toBe(0.5);
    expect(p.values.Driven).toBe(0.5);
    expect([...p.driven]).toEqual(["Driven"]);
    // スライダーでは、動かされているパラメータを変えられない。
    p.setParameter("Driven", -1);
    expect(state.values.get("Driven")).toBe(0.5);

    p.setTracking(face);
    step(16);
    step(32);
    // 入力は 1 回だけ渡す (次の入力が来るまで渡し直さない)。
    expect(tracked).toEqual([face]);
    expect([...p.driven].sort()).toEqual(["Driven", "Tracked"]);

    // トラッキング中にモーションを流すと、そのパラメータではモーションを優先する。
    p.playMotion("wave");
    step(48);
    expect([...p.driven].sort()).toEqual(["Driven", "Moved", "Tracked"]);
    expect(log.filter((l) => l.startsWith("over"))).toEqual(["over true"]);

    // トラッキングを止めると、書き込んでいたパラメータを既定値に戻す。
    state.values.set("Tracked", 0.9);
    p.setTracking(null);
    expect(log).toContain("clear_tracking");
    expect(state.values.get("Tracked")).toBe(0.25);
    expect(p.values.Tracked).toBe(0.25);
    step(64);
    expect([...p.driven].sort()).toEqual(["Driven", "Moved"]);
    expect(log.filter((l) => l.startsWith("over"))).toEqual(["over true", "over false"]);

    remove();
    step(80);
    expect([...p.driven]).toEqual(["Moved"]);
    p.setParameter("Driven", -1);
    expect(state.values.get("Driven")).toBe(-1);
  });

  it("passes the kept tracking parameters on the first frame and after a change", async () => {
    const { p, log } = await ready(options);
    await p.load(model);
    step(0);
    step(16);
    p.setTrackingKept(["Tracked", "Tracked"]);
    step(32);
    expect(log.filter((l) => l.startsWith("kept"))).toEqual(["kept ", "kept Tracked"]);
  });
});
