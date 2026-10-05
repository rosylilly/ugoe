// Player のモーション (待機モーション・繰り返し・始まらなかった理由。T0931)。wasm と DOM は偽物 (src/testing/)。
import { describe, expect, it, vi } from "vitest";
import { t } from "./i18n";
import { bundle } from "./testing/fakePuppet";
import { ready, setupPlayer, step } from "./testing/player";

// 画面の部品と MediaPipe まで読み込むと遅いので、Player が使う関数だけにする。
vi.mock("@ugoe/face-tracking", () => import("./testing/faceTracking"));

setupPlayer();

describe("Player motions", () => {
  const model = bundle("a.ugoe", 1, "path:/a.ugoe");
  const motions = [
    { name: "idle", looping: true, parameters: ["Body"] },
    { name: "wave", parameters: ["Arm"] },
  ];
  const parameters = [{ id: "Arm" }, { id: "Body" }];

  it("starts the idle motion on open and goes back to it when another motion ends", async () => {
    const { p, state, log } = await ready({ motions, parameters });
    await p.load(model);
    expect(p.idleMotion).toBe("idle");
    expect(p.currentMotion).toBe("idle");
    expect(log.filter((l) => l.startsWith("start"))).toEqual(["start idle idle"]);

    expect(p.playMotion("wave")).toBe(true);
    step(0);
    expect(p.currentMotion).toBe("wave");
    expect([...p.driven]).toEqual(["Arm"]);

    state.playing = false;
    step(16);
    expect(p.currentMotion).toBe("idle");
    expect(log.filter((l) => l.startsWith("start")).at(-1)).toBe("start idle idle");
    step(32);
    expect([...p.driven]).toEqual(["Body"]);
  });

  it("starts an ended motion again with the same priority when repeating", async () => {
    const { p, state, log } = await ready({ motions, parameters });
    await p.load(model);
    p.setRepeatMotion(true);
    p.playMotion("wave", "force");
    state.playing = false;
    step(0);
    expect(p.currentMotion).toBe("wave");
    expect(log.filter((l) => l.startsWith("start")).slice(-2)).toEqual([
      "start wave force",
      "start wave force",
    ]);

    // 止めたら待機に戻る。待機を止めたら何も流さない。
    p.stopMotions();
    expect(p.currentMotion).toBe("idle");
    p.stopMotions();
    expect(p.currentMotion).toBeNull();
    expect(log.at(-1)).toBe("stop");
  });

  it("tells why a motion did not start and clears it when the playing motion ends", async () => {
    const { p, state } = await ready({ motions, parameters });
    await p.load(model);
    state.accept = false;
    expect(p.playMotion("wave", "idle")).toBe(false);
    expect(p.motionNotice).toBe(
      t("motion.rejected", { name: "wave", priority: t("motion.priority.idle.label") }),
    );
    expect(p.currentMotion).toBe("idle");
    state.playing = false;
    step(0);
    expect(p.motionNotice).toBeNull();
  });

  it("stops the idle motion when auto idle is turned off and remembers the idle choice per model", async () => {
    const { p, log } = await ready({ motions, parameters });
    await p.load(model);
    p.setAutoIdle(false);
    expect(p.currentMotion).toBeNull();
    expect(log.at(-1)).toBe("stop");
    p.setAutoIdle(true);
    expect(p.currentMotion).toBe("idle");

    p.setIdleMotion("wave");
    expect(p.currentMotion).toBe("wave");
    expect(p.idleRemembered).toBe(true);
    p.detach();
    const { p: next } = await ready({ motions, parameters });
    await next.load(model);
    expect(next.idleMotion).toBe("wave");
    expect(next.idleRemembered).toBe(true);
  });

  it("keeps the latest events newest first with the motion and the number that fired them", async () => {
    const { p, state } = await ready({ motions, parameters });
    await p.load(model);
    expect(p.motionEvents).toEqual([]);

    // クロスフェード中は、フェードアウト中の待機と始めたモーションの両方が発火する。
    state.events = [
      { value: "blink", time: 0.5, motion: "idle", number: 1 },
      { value: "hi", time: 0, motion: "wave", number: 2 },
    ];
    step(0);
    expect(p.motionEvents).toEqual([
      { value: "hi", time: 0, motion: "wave", number: 2 },
      { value: "blink", time: 0.5, motion: "idle", number: 1 },
    ]);

    // 同じ名前を始め直したものは番号で分かる。上限 (5 個) を超えた古いものは捨てる。
    state.events = [1, 2, 3, 4].map((i) => ({
      value: "hi",
      time: 0,
      motion: "wave",
      number: 2 + i,
    }));
    step(16);
    expect(p.motionEvents.map((e) => `${e.value}@${e.motion}#${e.number}`)).toEqual([
      "hi@wave#6",
      "hi@wave#5",
      "hi@wave#4",
      "hi@wave#3",
      "hi@wave#2",
    ]);

    // モデルを読み直すと消える。
    await p.load(model);
    expect(p.motionEvents).toEqual([]);
  });
});
