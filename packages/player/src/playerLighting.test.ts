// Player の光源 (ドラッグ中は覚えない・モデルごとに覚える・覚える接頭辞。T0930・T0931)。wasm と DOM は偽物 (src/testing/)。
import { describe, expect, it, vi } from "vitest";
import { modelIdentity } from "./modelMemory";
import { bundle } from "./testing/fakePuppet";
import { ready, setupPlayer } from "./testing/player";

// 画面の部品と MediaPipe まで読み込むと遅いので、Player が使う関数だけにする。
vi.mock("@ugoe/face-tracking", () => import("./testing/faceTracking"));

setupPlayer();

describe("Player lighting", () => {
  const model = bundle("a.ugoe", 1, "path:/a.ugoe");
  const other = bundle("b.ugoe", 2, "path:/b.ugoe");
  const LAST = "ugoe-viewer.lighting";

  it("remembers under the key prefix the app passes and reads only its own values (T0930)", async () => {
    localStorage.setItem(
      LAST,
      JSON.stringify({ version: 1, settings: { ambient: { color: "#ffffff", intensity: 3 } } }),
    );
    localStorage.setItem("ugoe-viewer.auto-idle", "off");
    const { p } = await ready({}, { storagePrefix: "ugoe-studio.player" });
    expect(p.storage.prefix).toBe("ugoe-studio.player");
    expect(p.lighting.ambient.intensity).not.toBe(3);
    expect(p.autoIdle).toBe(true);
    await p.load(model);
    p.updateLighting((s) => {
      s.ambient.intensity = 2;
    });
    p.setAutoIdle(false);
    expect(localStorage.getItem("ugoe-studio.player.auto-idle")).toBe("off");
    const saved = JSON.parse(localStorage.getItem("ugoe-studio.player.lighting") ?? "null");
    expect(saved?.settings?.ambient?.intensity).toBe(2);
    expect(localStorage.getItem("ugoe-studio.player.lighting.models")).not.toBeNull();
    expect(JSON.parse(localStorage.getItem(LAST) ?? "null")?.settings?.ambient?.intensity).toBe(3);

    // 省くとビューアの接頭辞で、前から覚えていた値を読む。
    const { p: viewer } = await ready();
    expect(viewer.storage.prefix).toBe("ugoe-viewer");
    expect(viewer.lighting.ambient.intensity).toBe(3);
    expect(viewer.autoIdle).toBe(false);
  });

  it("draws a dragged light without remembering it until it is committed", async () => {
    const { p, state } = await ready();
    await p.load(model);
    const drawn = state.lighting.length;
    p.updateLighting(
      (s) => {
        s.ambient.intensity = 2;
      },
      { preview: true },
    );
    expect(state.lighting).toHaveLength(drawn + 1);
    expect(p.lighting.ambient.intensity).toBe(2);
    expect(localStorage.getItem(LAST)).toBeNull();
    expect(p.lightingRemembered).toBe(false);

    p.commitLighting();
    expect(JSON.parse(localStorage.getItem(LAST) ?? "null")?.settings?.ambient?.intensity).toBe(2);
    expect(p.lightingRemembered).toBe(true);
  });

  it("remembers a light dragged on the previous model when another model is opened", async () => {
    const { p } = await ready();
    await p.load(model);
    p.updateLighting(
      (s) => {
        s.ambient.intensity = 1.5;
      },
      { preview: true },
    );
    await p.load(other);
    // 覚えていないモデルでは、最後に使った設定を保つ。
    expect(p.lightingRemembered).toBe(false);
    expect(p.lighting.ambient.intensity).toBe(1.5);
    p.updateLighting((s) => {
      s.ambient.intensity = 0.5;
    });
    await p.load(model);
    expect(p.lightingRemembered).toBe(true);
    expect(p.lighting.ambient.intensity).toBe(1.5);
    await p.load(other);
    expect(p.lighting.ambient.intensity).toBe(0.5);
  });

  it("does not remember a dragged light again after it is forgotten", async () => {
    const { p } = await ready();
    await p.load(model);
    p.updateLighting(
      (s) => {
        s.ambient.intensity = 1.5;
      },
      { preview: true },
    );
    p.forgetRemembered(modelIdentity(model));
    await p.load(other);
    await p.load(model);
    expect(p.lightingRemembered).toBe(false);
  });
});
